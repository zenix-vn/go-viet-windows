//! Cấu trúc âm tiết tiếng Việt: phụ âm đầu + vần (nguyên âm) + phụ âm cuối.
//!
//! Kiểm tra ở đây là "dễ dãi": chấp nhận cả trạng thái đang gõ dở
//! (ví dụ `tieng` trước khi gõ thêm `e` để thành `tiêng`), nhưng loại các tổ
//! hợp không thể là tiếng Việt (ví dụ `cl`, `fix`, `tẽt`). Khi từ không hợp lệ,
//! engine trả lại nguyên các phím đã gõ, nhờ vậy gõ tiếng Anh không bị biến dạng.

use crate::chars::{Letter, Mark, Tone};

const INITIALS: &[&str] = &[
    "b", "c", "ch", "d", "đ", "g", "gh", "gi", "h", "k", "kh", "l", "m", "n", "ng", "ngh", "nh",
    "p", "ph", "q", "qu", "r", "s", "t", "th", "tr", "v", "x",
];

const FINALS: &[&str] = &["c", "ch", "m", "n", "ng", "nh", "p", "t"];

/// Các vần hợp lệ (dạng có dấu phụ) và việc vần đó có thể đứng trước phụ âm cuối hay không.
const CLUSTERS: &[(&str, bool)] = &[
    // một nguyên âm
    ("a", true),
    ("ă", true),
    ("â", true),
    ("e", true),
    ("ê", true),
    ("i", true),
    ("o", true),
    ("ô", true),
    ("ơ", true),
    ("u", true),
    ("ư", true),
    ("y", false),
    // hai nguyên âm
    ("ai", false),
    ("ao", false),
    ("au", false),
    ("ay", false),
    ("âu", false),
    ("ây", false),
    ("eo", false),
    ("êu", false),
    ("ia", false),
    ("iê", true),
    ("iu", false),
    ("oa", true),
    ("oă", true),
    ("oe", true),
    ("oi", false),
    ("oo", true),
    ("ôi", false),
    ("ơi", false),
    ("ua", false),
    ("uâ", true),
    ("uê", true),
    ("ui", false),
    ("uô", true),
    ("uơ", false),
    ("uy", true),
    ("ưa", false),
    ("ưi", false),
    ("ưo", true), // trạng thái đang gõ dở của "ươ"
    ("ưu", false),
    ("ươ", true),
    ("yê", true),
    // ba nguyên âm
    ("iêu", false),
    ("yêu", false),
    ("oai", false),
    ("oao", false),
    ("oay", false),
    ("oeo", false),
    ("uây", false),
    ("uôi", false),
    ("ươi", false),
    ("ươu", false),
    ("uya", false),
    ("uyê", true),
    ("uyu", false),
];

/// Vị trí phần vần trong từ: `letters[start..end]` là các nguyên âm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Split {
    pub start: usize,
    pub end: usize,
}

impl Split {
    pub fn len(&self) -> usize {
        self.end - self.start
    }
}

/// Tìm phần vần. `qu` và `gi` (khi theo sau là nguyên âm) được tính là phụ âm đầu.
pub fn split(letters: &[Letter]) -> Split {
    let n = letters.len();
    let mut i = 0;
    while i < n && !letters[i].is_vowel() {
        i += 1;
    }
    if i == 1 && n > 1 && letters[1].mark == Mark::None {
        let first = letters[0];
        let is_qu = first.base == 'q' && letters[1].base == 'u';
        let is_gi = first.base == 'g'
            && first.mark == Mark::None
            && letters[1].base == 'i'
            && n > 2
            && letters[2].is_vowel();
        if is_qu || is_gi {
            i = 2;
        }
    }
    let start = i;
    let mut end = start;
    while end < n && letters[end].is_vowel() {
        end += 1;
    }
    Split { start, end }
}

fn forms(letters: &[Letter]) -> String {
    letters.iter().map(|l| l.form()).collect()
}

fn cluster_marks_fit(cluster: &[Letter], pattern: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    if pat.len() != cluster.len() {
        return false;
    }
    cluster.iter().zip(pat.iter()).all(|(l, &p)| {
        let plain = Letter {
            mark: Mark::None,
            ..*l
        };
        if l.mark == Mark::None {
            // Chữ chưa có dấu phụ khớp với bất kỳ dạng nào cùng chữ gốc (đang gõ dở).
            base_of(p) == plain.base
        } else {
            l.form() == p
        }
    })
}

fn base_of(c: char) -> char {
    match c {
        'ă' | 'â' => 'a',
        'ê' => 'e',
        'ô' | 'ơ' => 'o',
        'ư' => 'u',
        other => other,
    }
}

/// Từ có thể là (hoặc đang gõ dở thành) một âm tiết tiếng Việt hay không.
pub fn is_valid(letters: &[Letter], tone: Tone) -> bool {
    if letters.is_empty() {
        return true;
    }
    let s = split(letters);
    let initial = forms(&letters[..s.start]);
    if !initial.is_empty() && !INITIALS.contains(&initial.as_str()) {
        return false;
    }
    let tail = &letters[s.end..];
    if tail.iter().any(|l| l.is_vowel()) {
        return false;
    }
    let fin = forms(tail);
    if !fin.is_empty() && !FINALS.contains(&fin.as_str()) {
        return false;
    }
    let cluster = &letters[s.start..s.end];
    if cluster.is_empty() {
        // Chỉ có phụ âm: hợp lệ khi chưa có thanh và chưa có phụ âm cuối.
        return tone == Tone::None && fin.is_empty();
    }
    let has_final = !fin.is_empty();
    let cluster_ok = CLUSTERS
        .iter()
        .any(|&(pat, can_final)| (!has_final || can_final) && cluster_marks_fit(cluster, pat));
    if !cluster_ok {
        return false;
    }
    // Vần kết thúc bằng c, ch, p, t chỉ đi với thanh sắc hoặc nặng (hoặc chưa có thanh).
    if matches!(fin.as_str(), "c" | "ch" | "p" | "t")
        && matches!(tone, Tone::Huyen | Tone::Hoi | Tone::Nga)
    {
        return false;
    }
    true
}

/// Vị trí chữ nhận thanh điệu.
///
/// `modern = false` là kiểu cũ (hòa, thủy), `modern = true` là kiểu mới (hoà, thuỷ).
pub fn tone_position(letters: &[Letter], modern: bool) -> Option<usize> {
    let s = split(letters);
    if s.len() == 0 {
        return None;
    }
    if let Some(p) = (s.start..s.end)
        .rev()
        .find(|&k| matches!(letters[k].mark, Mark::Circumflex | Mark::Breve | Mark::Horn))
    {
        return Some(p);
    }
    match s.len() {
        1 => Some(s.start),
        2 => {
            let has_final = s.end < letters.len();
            if has_final {
                return Some(s.start + 1);
            }
            let pair: String = letters[s.start..s.end].iter().map(|l| l.base).collect();
            if modern && matches!(pair.as_str(), "oa" | "oe" | "uy") {
                Some(s.start + 1)
            } else {
                Some(s.start)
            }
        }
        _ => Some(s.start + 1),
    }
}
