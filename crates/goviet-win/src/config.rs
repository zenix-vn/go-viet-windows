//! Cấu hình lưu tại `%APPDATA%\GoViet\config.toml` (dạng `khóa = giá trị` đơn giản).
#![allow(dead_code)]

use goviet_engine::{Conversion, InputMethod, Options};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Đang bật tiếng Việt (V) hay tắt (E).
    pub vietnamese: bool,
    pub method: InputMethod,
    pub modern_tone: bool,
    /// Bật gõ tắt.
    pub macros: bool,
    /// Kiểu chuyển mã clipboard dùng lần trước (Ctrl+Shift+F9 lặp lại).
    pub last_conversion: Conversion,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            vietnamese: true,
            method: InputMethod::Telex,
            modern_tone: false,
            macros: true,
            last_conversion: Conversion::TcvnToUnicode,
        }
    }
}

impl Config {
    pub fn engine_options(&self) -> Options {
        Options {
            method: self.method,
            modern_tone: self.modern_tone,
        }
    }

    pub fn parse(text: &str) -> Config {
        let mut c = Config::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim().trim_matches('"');
            match key.trim() {
                "vietnamese" => c.vietnamese = value == "true",
                "modern_tone" => c.modern_tone = value == "true",
                "macros" => c.macros = value == "true",
                "last_conversion" => {
                    if let Some(conv) = Conversion::from_id(value) {
                        c.last_conversion = conv;
                    }
                }
                "method" => {
                    c.method = if value.eq_ignore_ascii_case("vni") {
                        InputMethod::Vni
                    } else {
                        InputMethod::Telex
                    }
                }
                _ => {}
            }
        }
        c
    }

    pub fn to_text(self) -> String {
        format!(
            "# Cấu hình GoViet\nvietnamese = {}\nmethod = \"{}\"\nmodern_tone = {}\nmacros = {}\nlast_conversion = \"{}\"\n",
            self.vietnamese,
            match self.method {
                InputMethod::Telex => "telex",
                InputMethod::Vni => "vni",
            },
            self.modern_tone,
            self.macros,
            self.last_conversion.id()
        )
    }

    /// Thư mục `%APPDATA%\\GoViet`.
    pub fn dir() -> Option<PathBuf> {
        let base = std::env::var_os("APPDATA")?;
        Some(PathBuf::from(base).join("GoViet"))
    }

    fn path() -> Option<PathBuf> {
        Some(Self::dir()?.join("config.toml"))
    }

    /// File bảng gõ tắt `%APPDATA%\\GoViet\\macros.txt`.
    pub fn macros_path() -> Option<PathBuf> {
        Some(Self::dir()?.join("macros.txt"))
    }

    pub fn load() -> Config {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| Config::parse(&t))
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = Self::path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, self.to_text());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let c = Config {
            vietnamese: false,
            method: InputMethod::Vni,
            modern_tone: true,
            macros: false,
            last_conversion: Conversion::StripAccents,
        };
        assert_eq!(Config::parse(&c.to_text()), c);
    }

    #[test]
    fn bad_input_falls_back_to_defaults() {
        assert_eq!(Config::parse("rác\nmethod = ???"), Config::default());
    }
}
