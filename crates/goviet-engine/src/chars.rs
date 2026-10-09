//! Chữ cái, dấu phụ (mũ, trăng, móc, gạch) và thanh điệu.

/// Thanh điệu của một âm tiết.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    None,
    /// Sắc (á)
    Sac,
    /// Huyền (à)
    Huyen,
    /// Hỏi (ả)
    Hoi,
    /// Ngã (ã)
    Nga,
    /// Nặng (ạ)
    Nang,
}

impl Tone {
    fn index(self) -> usize {
        match self {
            Tone::None => 0,
            Tone::Sac => 1,
            Tone::Huyen => 2,
            Tone::Hoi => 3,
            Tone::Nga => 4,
            Tone::Nang => 5,
        }
    }

    fn from_index(i: usize) -> Tone {
        match i {
            1 => Tone::Sac,
            2 => Tone::Huyen,
            3 => Tone::Hoi,
            4 => Tone::Nga,
            5 => Tone::Nang,
            _ => Tone::None,
        }
    }
}

/// Dấu phụ gắn với một chữ cái.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mark {
    #[default]
    None,
    /// Mũ: â ê ô
    Circumflex,
    /// Trăng: ă
    Breve,
    /// Móc: ơ ư
    Horn,
    /// Gạch: đ
    Stroke,
}

/// Một chữ cái trong từ đang gõ (chưa gắn thanh điệu).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Letter {
    /// Chữ cái gốc, viết thường (a–z hoặc chữ số với VNI).
    pub base: char,
    pub mark: Mark,
    pub upper: bool,
}

impl Letter {
    pub fn plain(key: char) -> Letter {
        Letter {
            base: key.to_ascii_lowercase(),
            mark: Mark::None,
            upper: key.is_uppercase(),
        }
    }

    pub fn is_vowel(&self) -> bool {
        is_vowel_base(self.base)
    }

    /// Dạng viết thường có dấu phụ, chưa có thanh: `â`, `ơ`, `đ`, `b`...
    pub fn form(&self) -> char {
        match (self.base, self.mark) {
            ('a', Mark::Circumflex) => 'â',
            ('a', Mark::Breve) => 'ă',
            ('e', Mark::Circumflex) => 'ê',
            ('o', Mark::Circumflex) => 'ô',
            ('o', Mark::Horn) => 'ơ',
            ('u', Mark::Horn) => 'ư',
            ('d', Mark::Stroke) => 'đ',
            (b, _) => b,
        }
    }

    /// Ký tự Unicode dựng sẵn, có thanh điệu `tone` (chỉ áp dụng cho nguyên âm).
    pub fn render(&self, tone: Tone) -> char {
        let form = self.form();
        let lower = if self.is_vowel() {
            VOWELS
                .iter()
                .find(|row| row[0] == form)
                .map(|row| row[tone.index()])
                .unwrap_or(form)
        } else {
            form
        };
        if self.upper {
            lower.to_uppercase().next().unwrap_or(lower)
        } else {
            lower
        }
    }
}

pub fn is_vowel_base(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
}

/// Bảng nguyên âm: cột 0 là dạng không thanh, cột 1..5 là sắc, huyền, hỏi, ngã, nặng.
const VOWELS: [[char; 6]; 12] = [
    ['a', 'á', 'à', 'ả', 'ã', 'ạ'],
    ['ă', 'ắ', 'ằ', 'ẳ', 'ẵ', 'ặ'],
    ['â', 'ấ', 'ầ', 'ẩ', 'ẫ', 'ậ'],
    ['e', 'é', 'è', 'ẻ', 'ẽ', 'ẹ'],
    ['ê', 'ế', 'ề', 'ể', 'ễ', 'ệ'],
    ['i', 'í', 'ì', 'ỉ', 'ĩ', 'ị'],
    ['o', 'ó', 'ò', 'ỏ', 'õ', 'ọ'],
    ['ô', 'ố', 'ồ', 'ổ', 'ỗ', 'ộ'],
    ['ơ', 'ớ', 'ờ', 'ở', 'ỡ', 'ợ'],
    ['u', 'ú', 'ù', 'ủ', 'ũ', 'ụ'],
    ['ư', 'ứ', 'ừ', 'ử', 'ữ', 'ự'],
    ['y', 'ý', 'ỳ', 'ỷ', 'ỹ', 'ỵ'],
];

/// Tách một ký tự tiếng Việt thành (chữ gốc, dấu phụ, thanh, viết hoa).
pub fn decompose(c: char) -> Option<(Letter, Tone)> {
    let lower = c.to_lowercase().next()?;
    let upper = lower != c;
    if lower == 'đ' {
        return Some((
            Letter {
                base: 'd',
                mark: Mark::Stroke,
                upper,
            },
            Tone::None,
        ));
    }
    for row in VOWELS.iter() {
        if let Some(t) = row.iter().position(|&v| v == lower) {
            let (base, mark) = match row[0] {
                'ă' => ('a', Mark::Breve),
                'â' => ('a', Mark::Circumflex),
                'ê' => ('e', Mark::Circumflex),
                'ô' => ('o', Mark::Circumflex),
                'ơ' => ('o', Mark::Horn),
                'ư' => ('u', Mark::Horn),
                b => (b, Mark::None),
            };
            return Some((Letter { base, mark, upper }, Tone::from_index(t)));
        }
    }
    if lower.is_ascii_alphanumeric() {
        return Some((
            Letter {
                base: lower,
                mark: Mark::None,
                upper,
            },
            Tone::None,
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_and_decompose_roundtrip() {
        for row in VOWELS.iter() {
            for &c in row.iter() {
                for ch in [c, c.to_uppercase().next().unwrap()] {
                    let (l, t) = decompose(ch).unwrap();
                    assert_eq!(l.render(t), ch);
                }
            }
        }
        let (d, _) = decompose('Đ').unwrap();
        assert_eq!(d.render(Tone::None), 'Đ');
    }
}
