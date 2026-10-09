//! Gõ thử engine trên terminal: mỗi dòng nhập vào được xử lý như chuỗi phím.
//!
//! `cargo run -p goviet-cli -- [--vni] [--modern]`, ký tự `<` là Backspace.

use goviet_engine::{Action, Engine, InputMethod, Options};
use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = Options {
        method: if args.iter().any(|a| a == "--vni") {
            InputMethod::Vni
        } else {
            InputMethod::Telex
        },
        modern_tone: args.iter().any(|a| a == "--modern"),
    };
    eprintln!(
        "GoViet CLI ({:?}). Gõ một dòng rồi Enter, Ctrl+D để thoát.",
        opts.method
    );
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let mut e = Engine::new(opts);
        let mut buf: Vec<char> = Vec::new();
        for c in line.chars() {
            if c == '<' {
                e.backspace();
                buf.pop();
                continue;
            }
            match e.process_key(c) {
                Action::PassThrough => buf.push(c),
                Action::Replace { backspaces, text } => {
                    buf.truncate(buf.len().saturating_sub(backspaces));
                    buf.extend(text.chars());
                }
            }
        }
        println!("{}", buf.iter().collect::<String>());
        io::stdout().flush().ok();
    }
}
