use crate::chars::{decompose, Letter, Mark, Tone};
use crate::syllable::{is_valid, split, tone_position};

/// Kiểu gõ.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputMethod {
    #[default]
    Telex,
    Vni,
}

/// Tùy chọn của engine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub method: InputMethod,
    /// `true`: đặt dấu kiểu mới (hoà, thuỷ). `false`: kiểu cũ (hòa, thủy), giống mặc định của Unikey.
    pub modern_tone: bool,
}

/// Việc mà lớp hệ điều hành cần làm sau một phím.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Để phím đi thẳng vào ứng dụng, không cần can thiệp.
    PassThrough,
    /// Chặn phím, xóa `backspaces` ký tự trước con trỏ rồi gõ `text`.
    Replace { backspaces: usize, text: String },
}

/// Bộ gõ cho một từ đang gõ. Gọi [`Engine::reset`] khi con trỏ di chuyển hoặc đổi cửa sổ.
#[derive(Clone, Debug, Default)]
pub struct Engine {
    opts: Options,
    /// Các phím đã gõ cho từ hiện tại.
    keys: Vec<char>,
    /// Chuỗi đang hiển thị trong ứng dụng cho từ hiện tại.
    shown: Vec<char>,
}

impl Engine {
    pub fn new(opts: Options) -> Engine {
        Engine {
            opts,
            keys: Vec::new(),
            shown: Vec::new(),
        }
    }

    pub fn options(&self) -> Options {
        self.opts
    }

    pub fn set_options(&mut self, opts: Options) {
        self.opts = opts;
        self.reset();
    }

    /// Bắt đầu từ mới (sau dấu cách, click chuột, đổi cửa sổ...).
    pub fn reset(&mut self) {
        self.keys.clear();
        self.shown.clear();
    }

    /// Từ đang hiển thị.
    pub fn current_word(&self) -> String {
        self.shown.iter().collect()
    }

    /// Phím có thuộc về từ đang gõ không. Phím khác (dấu cách, dấu câu...) kết thúc từ.
    pub fn is_word_key(&self, c: char) -> bool {
        c.is_ascii_alphabetic() || (self.opts.method == InputMethod::Vni && c.is_ascii_digit())
    }

    /// Xử lý một phím ký tự.
    pub fn process_key(&mut self, c: char) -> Action {
        if !self.is_word_key(c) {
            self.reset();
            return Action::PassThrough;
        }
        self.keys.push(c);
        let next = compose(&self.keys, self.opts);
        let action = diff(&self.shown, &next, c);
        self.shown = next;
        action
    }

    /// Người dùng nhấn Backspace: phím đi thẳng vào ứng dụng, engine bỏ ký tự cuối.
    pub fn backspace(&mut self) -> Action {
        if self.shown.pop().is_none() {
            self.reset();
            return Action::PassThrough;
        }
        // Dựng lại chuỗi phím từ phần còn lại để có thể tiếp tục bỏ dấu cho từ này.
        let keys = encode(&self.shown, self.opts.method);
        if compose(&keys, self.opts) == self.shown {
            self.keys = keys;
        } else {
            self.reset();
        }
        Action::PassThrough
    }
}

fn diff(old: &[char], new: &[char], typed: char) -> Action {
    if new.len() == old.len() + 1 && new[..old.len()] == *old && new[old.len()] == typed {
        return Action::PassThrough;
    }
    let prefix = old
        .iter()
        .zip(new.iter())
        .take_while(|(a, b)| a == b)
        .count();
    Action::Replace {
        backspaces: old.len() - prefix,
        text: new[prefix..].iter().collect(),
    }
}

/// Từ đang được dựng từ chuỗi phím.
#[derive(Clone, Debug, Default)]
struct Word {
    letters: Vec<Letter>,
    tone: Tone,
    /// Sau khi người dùng gõ lặp để hủy dấu (ví dụ `ass` → `as`), các phím còn lại là chữ thường.
    literal: bool,
}

impl Word {
    fn has_vowel(&self) -> bool {
        self.letters.iter().any(|l| l.is_vowel())
    }

    fn valid(&self) -> bool {
        is_valid(&self.letters, self.tone)
    }

    fn push_literal(&mut self, key: char) {
        self.letters.push(Letter::plain(key));
    }

    /// Thử một thay đổi; chỉ giữ lại nếu kết quả vẫn là âm tiết hợp lệ.
    fn try_change(&mut self, f: impl FnOnce(&mut Word)) -> bool {
        let mut w = self.clone();
        f(&mut w);
        if w.valid() {
            *self = w;
            true
        } else {
            false
        }
    }

    /// Hủy dấu vừa gõ và thêm phím như chữ thường.
    fn undo_with_literal(&mut self, key: char, f: impl FnOnce(&mut Word)) {
        f(self);
        self.push_literal(key);
        self.literal = true;
    }

    fn vowel_range(&self) -> std::ops::Range<usize> {
        let s = split(&self.letters);
        s.start..s.end
    }

    fn render(&self, modern: bool) -> Vec<char> {
        let pos = if self.tone == Tone::None {
            None
        } else {
            tone_position(&self.letters, modern)
        };
        self.letters
            .iter()
            .enumerate()
            .map(|(i, l)| {
                l.render(if Some(i) == pos {
                    self.tone
                } else {
                    Tone::None
                })
            })
            .collect()
    }

    fn apply_tone(&mut self, key: char, tone: Tone) -> bool {
        if !self.has_vowel() {
            return false;
        }
        if self.tone == tone {
            self.undo_with_literal(key, |w| w.tone = Tone::None);
            return true;
        }
        self.try_change(|w| w.tone = tone)
    }

    fn remove_tone(&mut self) -> bool {
        if self.tone == Tone::None {
            return false;
        }
        self.tone = Tone::None;
        true
    }

    /// Đặt dấu phụ `mark` lên một trong các nguyên âm có chữ gốc thuộc `bases`.
    fn apply_mark(&mut self, key: char, bases: &[char], mark: Mark) -> bool {
        let range = self.vowel_range();
        let candidates: Vec<usize> = range
            .clone()
            .filter(|&i| bases.contains(&self.letters[i].base))
            .collect();

        // "uo" + móc → "ươ" (gõ một lần cho cả hai chữ).
        if mark == Mark::Horn {
            let pair = range.clone().find(|&i| {
                i + 1 < range.end && self.letters[i].base == 'u' && self.letters[i + 1].base == 'o'
            });
            if let Some(i) = pair {
                let both =
                    self.letters[i].mark == Mark::Horn && self.letters[i + 1].mark == Mark::Horn;
                if !both
                    && self.try_change(|w| {
                        w.letters[i].mark = Mark::Horn;
                        w.letters[i + 1].mark = Mark::Horn;
                    })
                {
                    return true;
                }
            }
        }

        // Gõ lặp lại → hủy dấu phụ.
        if candidates.iter().any(|&i| self.letters[i].mark == mark) {
            self.undo_with_literal(key, |w| {
                for &i in &candidates {
                    if w.letters[i].mark == mark {
                        w.letters[i].mark = Mark::None;
                    }
                }
            });
            return true;
        }

        for &i in candidates.iter().rev() {
            if self.try_change(|w| w.letters[i].mark = mark) {
                return true;
            }
        }
        false
    }

    /// Telex `w`: ă, ơ, ư, hoặc thêm `ư` khi chưa có nguyên âm nào phù hợp.
    fn apply_w(&mut self, key: char) -> bool {
        let range = self.vowel_range();
        let has_breve = range.clone().any(|i| self.letters[i].mark == Mark::Breve);
        let has_horn = range.clone().any(|i| self.letters[i].mark == Mark::Horn);
        if has_breve {
            return self.apply_mark(key, &['a'], Mark::Breve);
        }
        if self.apply_mark(key, &['o', 'u'], Mark::Horn) {
            return true;
        }
        if !has_horn && self.apply_mark(key, &['a'], Mark::Breve) {
            return true;
        }
        let upper = key.is_uppercase();
        self.try_change(|w| {
            w.letters.push(Letter {
                base: 'u',
                mark: Mark::Horn,
                upper,
            })
        })
    }

    fn apply_stroke(&mut self, key: char) -> bool {
        match self.letters.first() {
            Some(l) if l.base == 'd' && l.mark == Mark::Stroke => {
                self.undo_with_literal(key, |w| w.letters[0].mark = Mark::None);
                true
            }
            Some(l) if l.base == 'd' => self.try_change(|w| w.letters[0].mark = Mark::Stroke),
            _ => false,
        }
    }

    fn apply_key(&mut self, key: char, method: InputMethod) {
        // Từ đã không còn là tiếng Việt (hoặc đã hủy dấu): mọi phím sau đều là chữ thường.
        if self.literal || !self.valid() {
            self.push_literal(key);
            return;
        }
        let k = key.to_ascii_lowercase();
        let handled = match method {
            InputMethod::Telex => match k {
                's' => self.apply_tone(key, Tone::Sac),
                'f' => self.apply_tone(key, Tone::Huyen),
                'r' => self.apply_tone(key, Tone::Hoi),
                'x' => self.apply_tone(key, Tone::Nga),
                'j' => self.apply_tone(key, Tone::Nang),
                'z' => self.remove_tone(),
                'a' | 'e' | 'o' => self.apply_mark(key, &[k], Mark::Circumflex),
                'w' => self.apply_w(key),
                'd' => self.apply_stroke(key),
                _ => false,
            },
            InputMethod::Vni => match k {
                '1' => self.apply_tone(key, Tone::Sac),
                '2' => self.apply_tone(key, Tone::Huyen),
                '3' => self.apply_tone(key, Tone::Hoi),
                '4' => self.apply_tone(key, Tone::Nga),
                '5' => self.apply_tone(key, Tone::Nang),
                '0' => self.remove_tone(),
                '6' => self.apply_mark(key, &['a', 'e', 'o'], Mark::Circumflex),
                '7' => self.apply_mark(key, &['o', 'u'], Mark::Horn),
                '8' => self.apply_mark(key, &['a'], Mark::Breve),
                '9' => self.apply_stroke(key),
                _ => false,
            },
        };
        if !handled {
            self.push_literal(key);
        }
    }
}

/// Dựng chuỗi hiển thị từ chuỗi phím.
fn compose(keys: &[char], opts: Options) -> Vec<char> {
    let mut w = Word::default();
    for &k in keys {
        w.apply_key(k, opts.method);
    }
    if w.literal || w.valid() {
        w.render(opts.modern_tone)
    } else {
        // Không phải tiếng Việt (thường là tiếng Anh): trả lại đúng các phím đã gõ.
        keys.to_vec()
    }
}

/// Chuyển chuỗi đã hiển thị về chuỗi phím tương ứng (dùng khi Backspace).
fn encode(shown: &[char], method: InputMethod) -> Vec<char> {
    let mut keys = Vec::new();
    let mut tone = Tone::None;
    for &c in shown {
        let Some((l, t)) = decompose(c) else {
            keys.push(c);
            continue;
        };
        if t != Tone::None {
            tone = t;
        }
        keys.push(if l.upper {
            l.base.to_ascii_uppercase()
        } else {
            l.base
        });
        let mark_key = match (method, l.mark) {
            (_, Mark::None) => None,
            (InputMethod::Telex, Mark::Circumflex) => Some(l.base),
            (InputMethod::Telex, Mark::Breve | Mark::Horn) => Some('w'),
            (InputMethod::Telex, Mark::Stroke) => Some('d'),
            (InputMethod::Vni, Mark::Circumflex) => Some('6'),
            (InputMethod::Vni, Mark::Horn) => Some('7'),
            (InputMethod::Vni, Mark::Breve) => Some('8'),
            (InputMethod::Vni, Mark::Stroke) => Some('9'),
        };
        keys.extend(mark_key);
    }
    let tone_key = match (method, tone) {
        (_, Tone::None) => None,
        (InputMethod::Telex, Tone::Sac) => Some('s'),
        (InputMethod::Telex, Tone::Huyen) => Some('f'),
        (InputMethod::Telex, Tone::Hoi) => Some('r'),
        (InputMethod::Telex, Tone::Nga) => Some('x'),
        (InputMethod::Telex, Tone::Nang) => Some('j'),
        (InputMethod::Vni, Tone::Sac) => Some('1'),
        (InputMethod::Vni, Tone::Huyen) => Some('2'),
        (InputMethod::Vni, Tone::Hoi) => Some('3'),
        (InputMethod::Vni, Tone::Nga) => Some('4'),
        (InputMethod::Vni, Tone::Nang) => Some('5'),
    };
    keys.extend(tone_key);
    keys
}
