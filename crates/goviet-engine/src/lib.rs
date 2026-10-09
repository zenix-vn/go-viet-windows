//! GoViet engine: chuyển chuỗi phím Telex/VNI thành chữ tiếng Việt Unicode.
//!
//! Engine không phụ thuộc Windows. Nó giữ chuỗi phím của từ đang gõ, mỗi lần
//! có phím mới thì tính lại toàn bộ từ, rồi trả về thao tác cần làm với ứng
//! dụng: để phím đi qua, hoặc xóa `n` ký tự và gõ chuỗi mới.
//!
//! ```
//! use goviet_engine::{Engine, Options};
//! let mut e = Engine::new(Options::default());
//! for c in "tieengs".chars() { e.process_key(c); }
//! assert_eq!(e.current_word(), "tiếng");
//! ```

mod chars;
mod convert;
mod engine;
mod legacy_tables;
mod macros;
mod syllable;

pub use chars::{Mark, Tone};
pub use convert::{convert, Conversion};
pub use engine::{Action, Engine, InputMethod, Options};
pub use macros::{Macros, DEFAULT_MACROS};
