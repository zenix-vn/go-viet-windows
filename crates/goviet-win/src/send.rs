//! Gửi phím giả lập vào ứng dụng đang dùng.

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VK_BACK,
};

/// Đánh dấu phím do GoViet gửi, để hook bỏ qua chính phím của mình.
pub const INJECTED_MARK: usize = 0x474F_5654; // "GOVT"

fn key(vk: u16, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECTED_MARK,
            },
        },
    }
}

/// Xóa `backspaces` ký tự trước con trỏ rồi gõ `text`.
pub fn replace(backspaces: usize, text: &str) {
    replace_then_key(backspaces, text, None);
}

/// Ký tự đệm gõ trước khi xóa: thay thế phần gợi ý đang bôi đen (thanh địa chỉ trình duyệt,
/// ô Excel) để Backspace xóa đúng chữ đang gõ, rồi chính nó cũng bị xóa bằng một Backspace thêm.
const FILLER: u16 = 0x202F; // NARROW NO-BREAK SPACE

/// Như [`replace`], sau đó gõ lại phím `vk` (ví dụ dấu cách đã kích hoạt gõ tắt).
pub fn replace_then_key(backspaces: usize, text: &str, vk: Option<u16>) {
    let mut inputs = Vec::with_capacity(backspaces * 2 + text.len() * 2 + 6);
    let backspaces = if backspaces > 0 {
        inputs.push(key(0, FILLER, KEYEVENTF_UNICODE));
        inputs.push(key(0, FILLER, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
        backspaces + 1
    } else {
        0
    };
    for _ in 0..backspaces {
        inputs.push(key(VK_BACK, 0, 0));
        inputs.push(key(VK_BACK, 0, KEYEVENTF_KEYUP));
    }
    for unit in text.encode_utf16() {
        inputs.push(key(0, unit, KEYEVENTF_UNICODE));
        inputs.push(key(0, unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
    }
    if let Some(vk) = vk {
        inputs.push(key(vk, 0, 0));
        inputs.push(key(vk, 0, KEYEVENTF_KEYUP));
    }
    if inputs.is_empty() {
        return;
    }
    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
    }
}
