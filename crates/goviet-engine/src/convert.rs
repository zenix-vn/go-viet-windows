//! Chuyển mã văn bản (dùng cho chức năng chuyển mã clipboard).
//!
//! Văn bản bảng mã cũ (TCVN3, VNI Windows) khi copy từ Word/Excel nằm trong clipboard
//! dưới dạng Unicode theo trang mã Windows-1252: mỗi byte của bảng mã cũ là một ký tự.
//! Vì vậy ta đổi chuỗi ↔ byte qua cp1252 rồi tra bảng.

use crate::chars::{decompose, Letter, Mark, Tone};
use crate::legacy_tables::LEGACY;

/// Kiểu chuyển mã.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conversion {
    TcvnToUnicode,
    VniToUnicode,
    /// Unicode tổ hợp (dấu rời, thường gặp khi copy từ PDF/macOS) → Unicode dựng sẵn.
    ComposeUnicode,
    UnicodeToTcvn,
    UnicodeToVni,
    /// Bỏ dấu tiếng Việt: "Tiếng Việt" → "Tieng Viet".
    StripAccents,
}

impl Conversion {
    pub const ALL: [Conversion; 6] = [
        Conversion::TcvnToUnicode,
        Conversion::VniToUnicode,
        Conversion::ComposeUnicode,
        Conversion::UnicodeToTcvn,
        Conversion::UnicodeToVni,
        Conversion::StripAccents,
    ];

    /// Tên hiển thị trên menu.
    pub fn label(self) -> &'static str {
        match self {
            Conversion::TcvnToUnicode => "TCVN3 (ABC) → Unicode",
            Conversion::VniToUnicode => "VNI Windows → Unicode",
            Conversion::ComposeUnicode => "Unicode tổ hợp → Unicode dựng sẵn",
            Conversion::UnicodeToTcvn => "Unicode → TCVN3 (ABC)",
            Conversion::UnicodeToVni => "Unicode → VNI Windows",
            Conversion::StripAccents => "Bỏ dấu tiếng Việt",
        }
    }

    /// Tên dùng trong file cấu hình.
    pub fn id(self) -> &'static str {
        match self {
            Conversion::TcvnToUnicode => "tcvn3-unicode",
            Conversion::VniToUnicode => "vni-unicode",
            Conversion::ComposeUnicode => "compose-unicode",
            Conversion::UnicodeToTcvn => "unicode-tcvn3",
            Conversion::UnicodeToVni => "unicode-vni",
            Conversion::StripAccents => "strip-accents",
        }
    }

    pub fn from_id(id: &str) -> Option<Conversion> {
        Conversion::ALL.into_iter().find(|c| c.id() == id)
    }
}

pub fn convert(text: &str, conv: Conversion) -> String {
    match conv {
        Conversion::TcvnToUnicode => tcvn_to_unicode(text),
        Conversion::VniToUnicode => vni_to_unicode(text),
        Conversion::ComposeUnicode => compose_unicode(text),
        Conversion::UnicodeToTcvn => unicode_to_tcvn(text),
        Conversion::UnicodeToVni => unicode_to_vni(text),
        Conversion::StripAccents => strip_accents(text),
    }
}

/// Ký tự Windows-1252 ở vùng 0x80–0x9F.
const CP1252_HIGH: [(u8, char); 27] = [
    (0x80, '€'),
    (0x82, '‚'),
    (0x83, 'ƒ'),
    (0x84, '„'),
    (0x85, '…'),
    (0x86, '†'),
    (0x87, '‡'),
    (0x88, 'ˆ'),
    (0x89, '‰'),
    (0x8A, 'Š'),
    (0x8B, '‹'),
    (0x8C, 'Œ'),
    (0x8E, 'Ž'),
    (0x91, '‘'),
    (0x92, '’'),
    (0x93, '“'),
    (0x94, '”'),
    (0x95, '•'),
    (0x96, '–'),
    (0x97, '—'),
    (0x98, '˜'),
    (0x99, '™'),
    (0x9A, 'š'),
    (0x9B, '›'),
    (0x9C, 'œ'),
    (0x9E, 'ž'),
    (0x9F, 'Ÿ'),
];

/// Ký tự → byte theo cp1252; `None` nếu ký tự nằm ngoài cp1252.
fn char_to_byte(c: char) -> Option<u8> {
    if let Some(&(b, _)) = CP1252_HIGH.iter().find(|&&(_, ch)| ch == c) {
        return Some(b);
    }
    let code = c as u32;
    (code <= 0xFF).then_some(code as u8)
}

fn byte_to_char(b: u8) -> char {
    CP1252_HIGH
        .iter()
        .find(|&&(byte, _)| byte == b)
        .map(|&(_, c)| c)
        .unwrap_or(b as char)
}

fn tcvn_to_unicode(text: &str) -> String {
    text.chars()
        .map(|c| match char_to_byte(c) {
            // TCVN3 dùng chung một mã cho chữ hoa và chữ thường có thanh: ưu tiên chữ thường.
            Some(b) if b >= 0x80 => LEGACY
                .iter()
                .rev()
                .find(|&&(_, t, _)| t == b)
                .map(|&(u, _, _)| u)
                .unwrap_or(c),
            _ => c,
        })
        .collect()
}

fn unicode_to_tcvn(text: &str) -> String {
    text.chars()
        .map(|c| match LEGACY.iter().find(|&&(u, _, _)| u == c) {
            Some(&(_, t, _)) => byte_to_char(t),
            None => c,
        })
        .collect()
}

fn vni_to_unicode(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let b0 = char_to_byte(chars[i]);
        if let (Some(lo), Some(hi)) = (b0, chars.get(i + 1).and_then(|&c| char_to_byte(c))) {
            let code = (lo as u16) | ((hi as u16) << 8);
            if let Some(&(u, _, _)) = LEGACY.iter().find(|&&(_, _, v)| v == code && v > 0xFF) {
                out.push(u);
                i += 2;
                continue;
            }
        }
        let single = b0.and_then(|b| {
            LEGACY
                .iter()
                .find(|&&(_, _, v)| v == b as u16)
                .map(|&(u, _, _)| u)
        });
        out.push(single.unwrap_or(chars[i]));
        i += 1;
    }
    out
}

fn unicode_to_vni(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for c in text.chars() {
        match LEGACY.iter().find(|&&(u, _, _)| u == c) {
            Some(&(_, _, v)) => {
                out.push(byte_to_char((v & 0xFF) as u8));
                if v > 0xFF {
                    out.push(byte_to_char((v >> 8) as u8));
                }
            }
            None => out.push(c),
        }
    }
    out
}

fn strip_accents(text: &str) -> String {
    compose_unicode(text)
        .chars()
        .map(|c| match decompose(c) {
            Some((l, _)) if c as u32 > 0x7F => {
                let base = l.base;
                if l.upper {
                    base.to_ascii_uppercase()
                } else {
                    base
                }
            }
            _ => c,
        })
        .collect()
}

fn combining(c: char) -> Option<Result<Tone, Mark>> {
    Some(match c {
        '\u{0301}' => Ok(Tone::Sac),
        '\u{0300}' => Ok(Tone::Huyen),
        '\u{0309}' => Ok(Tone::Hoi),
        '\u{0303}' => Ok(Tone::Nga),
        '\u{0323}' => Ok(Tone::Nang),
        '\u{0302}' => Err(Mark::Circumflex),
        '\u{0306}' => Err(Mark::Breve),
        '\u{031B}' => Err(Mark::Horn),
        _ => return None,
    })
}

fn compose_unicode(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let mut j = i + 1;
        while j < chars.len() && combining(chars[j]).is_some() {
            j += 1;
        }
        if j == i + 1 {
            out.push(c);
            i = j;
            continue;
        }
        match decompose(c) {
            Some((mut letter, mut tone)) if letter.is_vowel() => {
                let mut leftover = Vec::new();
                for &m in &chars[i + 1..j] {
                    match combining(m) {
                        Some(Ok(t)) => tone = t,
                        Some(Err(mark)) if fits(letter, mark) => letter.mark = mark,
                        _ => leftover.push(m),
                    }
                }
                out.push(letter.render(tone));
                out.extend(leftover);
            }
            _ => out.extend(&chars[i..j]),
        }
        i = j;
    }
    out
}

fn fits(l: Letter, mark: Mark) -> bool {
    matches!(
        (l.base, mark),
        ('a' | 'e' | 'o', Mark::Circumflex) | ('a', Mark::Breve) | ('o' | 'u', Mark::Horn)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Tiếng Việt có dấu: Đà Nẵng, Huế, người ĐƯỢC ỦY QUYỀN.";

    #[test]
    fn tcvn_roundtrip() {
        let legacy = convert(SAMPLE, Conversion::UnicodeToTcvn);
        assert_ne!(legacy, SAMPLE);
        // TCVN3 không phân biệt hoa/thường cho chữ có thanh, nên so sánh bản viết thường.
        assert_eq!(
            convert(&legacy, Conversion::TcvnToUnicode).to_lowercase(),
            SAMPLE.to_lowercase()
        );
        // "á" trong TCVN3 là byte 0xB8 (hiện thành '¸' trong cp1252).
        assert_eq!(convert("á", Conversion::UnicodeToTcvn), "¸");
    }

    #[test]
    fn vni_roundtrip() {
        let legacy = convert(SAMPLE, Conversion::UnicodeToVni);
        assert_eq!(convert("á", Conversion::UnicodeToVni), "aù");
        assert_eq!(convert(&legacy, Conversion::VniToUnicode), SAMPLE);
    }

    #[test]
    fn strip() {
        assert_eq!(
            convert(SAMPLE, Conversion::StripAccents),
            "Tieng Viet co dau: Da Nang, Hue, nguoi DUOC UY QUYEN."
        );
    }

    #[test]
    fn compose() {
        let nfd = "Tie\u{0302}\u{0301}ng Vie\u{0323}\u{0302}t, u\u{031B}o\u{031B}\u{0300}n";
        assert_eq!(convert(nfd, Conversion::ComposeUnicode), "Tiếng Việt, ườn");
        assert_eq!(convert("abc", Conversion::ComposeUnicode), "abc");
    }

    #[test]
    fn ids_roundtrip() {
        for c in Conversion::ALL {
            assert_eq!(Conversion::from_id(c.id()), Some(c));
        }
    }
}

#[cfg(test)]
mod landing_page_sample {
    use super::*;

    /// Mẫu "Trước" trên trang giới thiệu phải đúng là TCVN3 của "Tiếng Việt có dấu".
    #[test]
    fn sample_matches() {
        assert_eq!(
            convert("Tiếng Việt có dấu", Conversion::UnicodeToTcvn),
            "TiÕng ViÖt cã dÊu"
        );
    }
}
