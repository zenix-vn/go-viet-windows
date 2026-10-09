//! GoViet cho Windows: bộ gõ tiếng Việt chạy ở khay hệ thống.
//!
//! Cơ chế giống Unikey: hook bàn phím mức thấp (`WH_KEYBOARD_LL`) bắt từng phím,
//! engine tính chữ có dấu, rồi `SendInput` xóa phần cũ và gõ chuỗi Unicode mới.

#![cfg_attr(windows, windows_subsystem = "windows")]

mod config;

#[cfg(any(windows, feature = "check-on-other-os"))]
mod app;
#[cfg(any(windows, feature = "check-on-other-os"))]
mod icon;
#[cfg(any(windows, feature = "check-on-other-os"))]
mod send;
#[cfg(any(windows, feature = "check-on-other-os"))]
mod startup;
#[cfg(any(windows, feature = "check-on-other-os"))]
mod wide;

fn main() {
    #[cfg(any(windows, feature = "check-on-other-os"))]
    app::run();

    #[cfg(not(any(windows, feature = "check-on-other-os")))]
    eprintln!("GoViet chỉ chạy trên Windows. Dùng `cargo run -p goviet-cli` để gõ thử engine.");
}
