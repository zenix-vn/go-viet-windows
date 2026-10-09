//! Kiểm thử đầu-cuối: giả lập một ô soạn thảo nhận phím và thao tác từ engine.

use goviet_engine::{Action, Engine, InputMethod, Options};

/// Gõ chuỗi `keys` vào một ô soạn thảo giả lập. `<` là phím Backspace.
fn type_keys(opts: Options, keys: &str) -> String {
    let mut e = Engine::new(opts);
    let mut buf: Vec<char> = Vec::new();
    for c in keys.chars() {
        if c == '<' {
            e.backspace();
            buf.pop();
            continue;
        }
        match e.process_key(c) {
            Action::PassThrough => buf.push(c),
            Action::Replace { backspaces, text } => {
                for _ in 0..backspaces {
                    buf.pop();
                }
                buf.extend(text.chars());
            }
        }
    }
    buf.into_iter().collect()
}

fn telex(keys: &str) -> String {
    type_keys(Options::default(), keys)
}

fn telex_modern(keys: &str) -> String {
    type_keys(
        Options {
            modern_tone: true,
            ..Options::default()
        },
        keys,
    )
}

fn vni(keys: &str) -> String {
    type_keys(
        Options {
            method: InputMethod::Vni,
            ..Options::default()
        },
        keys,
    )
}

fn check(f: fn(&str) -> String, cases: &[(&str, &str)]) {
    let mut failed = Vec::new();
    for &(input, want) in cases {
        let got = f(input);
        if got != want {
            failed.push(format!("{input:?}: got {got:?}, want {want:?}"));
        }
    }
    assert!(failed.is_empty(), "\n{}", failed.join("\n"));
}

#[test]
fn telex_basic_words() {
    check(
        telex,
        &[
            ("xin chaof", "xin chào"),
            ("tieengs Vieetj", "tiếng Việt"),
            ("dduowcj", "được"),
            ("nguowif", "người"),
            ("ddaau", "đâu"),
            ("ddi", "đi"),
            ("hoaf", "hòa"),
            ("thuyr", "thủy"),
            ("cuar", "của"),
            ("quas", "quá"),
            ("quoocs", "quốc"),
            ("gif", "gì"),
            ("gios", "gió"),
            ("giuwax", "giữa"),
            ("giwx", "giữ"),
            ("ruwowuj", "rượu"),
            ("ddwowngf", "đường"),
            ("dduongwf", "đường"),
            ("nghieeng", "nghiêng"),
            ("khuya", "khuya"),
            ("hoawcj", "hoặc"),
            ("muwa", "mưa"),
            ("muaw", "mưa"),
            ("cuwu", "cưu"),
            ("tw", "tư"),
            ("aw", "ă"),
            ("oo", "ô"),
            ("ee", "ê"),
            ("mias", "mía"),
            ("toans", "toán"),
            ("hoangf", "hoàng"),
            ("tuaans", "tuấn"),
            ("yeeu", "yêu"),
            ("khuyeen", "khuyên"),
            ("quyeets", "quyết"),
            ("tiengse", "tiếng"),
            ("nguyeenx", "nguyễn"),
            ("nguyenex", "nguyễn"),
        ],
    );
}

#[test]
fn telex_uppercase() {
    check(
        telex,
        &[
            ("VIEETJ NAM", "VIỆT NAM"),
            ("Ddaf Nawngx", "Đà Nẵng"),
            ("DDuwowngf", "Đường"),
            ("TIEENGS", "TIẾNG"),
        ],
    );
}

#[test]
fn telex_tone_change_and_undo() {
    check(
        telex,
        &[
            ("asf", "à"),  // đổi thanh
            ("ass", "as"), // gõ lặp → hủy
            ("aaa", "aa"),
            ("aww", "aw"),
            ("ddd", "dd"),
            ("booong", "boong"),
            ("asz", "a"), // z xóa thanh
            ("az", "az"), // không có thanh thì z là chữ thường
        ],
    );
}

#[test]
fn telex_english_restore() {
    check(
        telex,
        &[
            ("class", "class"),
            ("fix", "fix"),
            ("text", "text"),
            ("email", "email"),
            ("window", "window"),
            ("what", "what"),
            ("string", "string"),
            ("we", "we"),
        ],
    );
}

#[test]
fn modern_tone_placement() {
    check(
        telex_modern,
        &[
            ("hoaf", "hoà"),
            ("thuyr", "thuỷ"),
            ("khoer", "khoẻ"),
            ("hoangf", "hoàng"),
            ("cuar", "của"),
        ],
    );
}

#[test]
fn vni_words() {
    check(
        vni,
        &[
            ("tie61ng Vie65t", "tiếng Việt"),
            ("d9u7o7c5", "được"),
            ("ngu7o7i2", "người"),
            ("hoa2", "hòa"),
            ("a8n", "ăn"),
            ("D9a2 Na8ng4", "Đà Nẵng"),
            ("a11", "a1"),
            ("a66", "a6"),
            ("123", "123"),
        ],
    );
}

#[test]
fn backspace_keeps_editing_word() {
    check(
        telex,
        &[
            ("tieeng<<s", "tiế"),
            ("tieengs<g", "tiếng"),
            ("hoaf<", "hò"),
            ("hoaf<a", "hòa"),
            ("dduowcj<c", "được"),
            ("dduowcj<j", "đươj"), // j lần nữa = hủy thanh nặng
            ("vieet<<eej", "việ"),
        ],
    );
}

#[test]
fn plain_append_passes_through() {
    let mut e = Engine::new(Options::default());
    assert_eq!(e.process_key('t'), Action::PassThrough);
    assert_eq!(e.process_key('i'), Action::PassThrough);
    assert_eq!(e.process_key('e'), Action::PassThrough);
    assert_eq!(
        e.process_key('e'),
        Action::Replace {
            backspaces: 1,
            text: "ê".into()
        }
    );
    assert_eq!(e.process_key('n'), Action::PassThrough);
    assert_eq!(
        e.process_key('s'),
        Action::Replace {
            backspaces: 2,
            text: "ến".into()
        }
    );
    assert_eq!(e.process_key(' '), Action::PassThrough);
    assert_eq!(e.current_word(), "");
}
