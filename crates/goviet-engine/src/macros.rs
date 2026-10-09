//! Gõ tắt: thay một từ ngắn bằng cụm từ dài khi kết thúc từ (dấu cách, Enter, dấu câu).
//!
//! Định dạng file: mỗi dòng `từ tắt = nội dung`, dòng bắt đầu bằng `#` là ghi chú.
//! Từ tắt viết thường sẽ tự đổi theo kiểu chữ khi gõ: `vn` → Việt Nam,
//! `Vn` → Việt Nam, `VN` → VIỆT NAM.

use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Macros {
    map: HashMap<String, String>,
}

/// Nội dung mặc định khi người dùng mở bảng gõ tắt lần đầu.
pub const DEFAULT_MACROS: &str = "\
# Bảng gõ tắt GoViet
# Mỗi dòng: từ tắt = nội dung. Gõ từ tắt rồi nhấn dấu cách, Enter hoặc dấu câu.
# Từ tắt so với chữ đang hiển thị (ví dụ gõ 'dd' sẽ hiện 'đ', nên đừng dùng 'dd' làm từ tắt).
vn = Việt Nam
ko = không
dc = được
nc = nước
tphcm = Thành phố Hồ Chí Minh
hn = Hà Nội
";

impl Macros {
    pub fn parse(text: &str) -> Macros {
        let mut map = HashMap::new();
        for line in text.lines() {
            let line = line.trim_start_matches('\u{feff}').trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            if !key.is_empty() && !value.is_empty() && !key.contains(char::is_whitespace) {
                map.insert(key.to_string(), value.to_string());
            }
        }
        Macros { map }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Nội dung thay thế cho `word`, có điều chỉnh chữ hoa theo cách gõ.
    pub fn lookup(&self, word: &str) -> Option<String> {
        if word.is_empty() {
            return None;
        }
        if let Some(v) = self.map.get(word) {
            return Some(v.clone());
        }
        let lower = word.to_lowercase();
        let value = self.map.get(&lower)?;
        let mut chars = word.chars();
        let first_upper = chars.next().is_some_and(|c| c.is_uppercase());
        let rest_upper = chars.clone().all(|c| !c.is_lowercase());
        if first_upper && rest_upper && word.chars().count() > 1 {
            Some(value.to_uppercase())
        } else if first_upper {
            let mut v = value.chars();
            Some(match v.next() {
                Some(f) => f.to_uppercase().chain(v).collect(),
                None => String::new(),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_with_case() {
        let m = Macros::parse(DEFAULT_MACROS);
        assert_eq!(m.lookup("vn").as_deref(), Some("Việt Nam"));
        assert_eq!(m.lookup("VN").as_deref(), Some("VIỆT NAM"));
        assert_eq!(m.lookup("Ko").as_deref(), Some("Không"));
        assert_eq!(m.lookup("kO"), None);
        assert_eq!(m.lookup("xyz"), None);
        assert_eq!(m.lookup(""), None);
    }

    #[test]
    fn parse_skips_bad_lines() {
        let m = Macros::parse("\u{feff}# ghi chú\nab = \n = x\na b = c\nok=được\n");
        assert_eq!(m.len(), 1);
        assert_eq!(m.lookup("ok").as_deref(), Some("được"));
    }
}
