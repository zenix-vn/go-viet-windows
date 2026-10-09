//! Vòng đời ứng dụng: cửa sổ ẩn, icon khay, menu, hook bàn phím/chuột.

use crate::config::Config;
use crate::icon::{letter_icon, BLUE, RED};
use crate::send::{self, INJECTED_MARK};
use crate::startup;
use crate::wide::{copy_into, wide};
use goviet_engine::{Action, Engine, InputMethod};
use std::cell::RefCell;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_LCONTROL, VK_LMENU,
    VK_LSHIFT, VK_LWIN, VK_MENU, VK_NUMPAD0, VK_NUMPAD9, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN,
    VK_SHIFT,
};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CallNextHookEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon,
    DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, GetForegroundWindow, GetMessageW,
    MessageBoxW, PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW,
    SetForegroundWindow, SetWindowsHookExW, TrackPopupMenu, TranslateMessage, UnhookWindowsHookEx,
    HC_ACTION, HHOOK, HICON, KBDLLHOOKSTRUCT, MB_ICONINFORMATION, MB_OK, MF_CHECKED, MF_SEPARATOR,
    MF_STRING, MSG, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WH_KEYBOARD_LL, WH_MOUSE_LL,
    WM_APP, WM_COMMAND, WM_DESTROY, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_NULL, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDCLASSW,
    WS_OVERLAPPED,
};

const WM_TRAY: u32 = WM_APP + 1;
const TRAY_ID: u32 = 1;

const CMD_TOGGLE: usize = 1001;
const CMD_TELEX: usize = 1002;
const CMD_VNI: usize = 1003;
const CMD_MODERN: usize = 1004;
const CMD_STARTUP: usize = 1005;
const CMD_ABOUT: usize = 1006;
const CMD_EXIT: usize = 1007;

/// Trạng thái phím tắt Ctrl+Shift: bật/tắt khi nhả phím mà không bấm phím nào khác.
#[derive(Default)]
struct Chord {
    armed: bool,
    dirty: bool,
}

struct App {
    engine: Engine,
    config: Config,
    hwnd: HWND,
    icon_v: HICON,
    icon_e: HICON,
    last_window: HWND,
    chord: Chord,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static HOOKS: RefCell<(HHOOK, HHOOK)> = const { RefCell::new((null_mut(), null_mut())) };
    static TASKBAR_CREATED: RefCell<u32> = const { RefCell::new(0) };
}

/// Chạy `f` với trạng thái ứng dụng. Trả `None` nếu đang bận (gọi lồng) hoặc chưa khởi tạo,
/// để hook không bao giờ panic.
fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|cell| {
        let mut guard = cell.try_borrow_mut().ok()?;
        guard.as_mut().map(f)
    })
}

fn key_down(vk: u16) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) < 0 }
}

fn is_ctrl(vk: u16) -> bool {
    matches!(vk, VK_CONTROL | VK_LCONTROL | VK_RCONTROL)
}

fn is_shift(vk: u16) -> bool {
    matches!(vk, VK_SHIFT | VK_LSHIFT | VK_RSHIFT)
}

fn is_alt_or_win(vk: u16) -> bool {
    matches!(vk, VK_MENU | VK_LMENU | VK_RMENU | VK_LWIN | VK_RWIN)
}

impl App {
    /// Trả `true` nếu GoViet đã xử lý phím (chặn phím gốc).
    fn on_key(&mut self, msg: u32, kb: &KBDLLHOOKSTRUCT) -> bool {
        let vk = kb.vkCode as u16;
        let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
        let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

        if is_ctrl(vk) || is_shift(vk) {
            if down {
                let other = if is_ctrl(vk) { VK_SHIFT } else { VK_CONTROL };
                if key_down(other) && !self.chord.armed {
                    self.chord = Chord {
                        armed: true,
                        dirty: false,
                    };
                }
            } else if up && self.chord.armed {
                let toggle = !self.chord.dirty;
                self.chord = Chord::default();
                if toggle {
                    self.toggle();
                }
            }
            return false;
        }
        if !down {
            return false;
        }
        if self.chord.armed {
            self.chord.dirty = true;
        }
        if is_alt_or_win(vk) || vk == VK_CAPITAL {
            return false;
        }
        if !self.config.vietnamese {
            return false;
        }

        let window = unsafe { GetForegroundWindow() };
        if window != self.last_window {
            self.last_window = window;
            self.engine.reset();
        }
        if key_down(VK_CONTROL) || key_down(VK_MENU) || key_down(VK_LWIN) || key_down(VK_RWIN) {
            self.engine.reset();
            return false;
        }

        let shift = key_down(VK_SHIFT);
        let ch = match vk {
            VK_BACK => {
                self.engine.backspace();
                return false;
            }
            0x41..=0x5A => {
                let caps = unsafe { GetKeyState(VK_CAPITAL as i32) } & 1 != 0;
                let c = vk as u8 as char;
                Some(if shift != caps {
                    c
                } else {
                    c.to_ascii_lowercase()
                })
            }
            0x30..=0x39 if !shift => Some(vk as u8 as char),
            VK_NUMPAD0..=VK_NUMPAD9 => Some((b'0' + (vk - VK_NUMPAD0) as u8) as char),
            _ => None,
        };
        let Some(ch) = ch else {
            self.engine.reset();
            return false;
        };
        match self.engine.process_key(ch) {
            Action::PassThrough => false,
            Action::Replace { backspaces, text } => {
                send::replace(backspaces, &text);
                true
            }
        }
    }

    fn toggle(&mut self) {
        self.config.vietnamese = !self.config.vietnamese;
        self.engine.reset();
        self.config.save();
        self.update_tray(NIM_MODIFY);
    }

    fn apply_config(&mut self) {
        self.engine.set_options(self.config.engine_options());
        self.config.save();
        self.update_tray(NIM_MODIFY);
    }

    fn update_tray(&self, action: u32) {
        let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = self.hwnd;
        nid.uID = TRAY_ID;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY;
        nid.hIcon = if self.config.vietnamese {
            self.icon_v
        } else {
            self.icon_e
        };
        let method = match self.config.method {
            InputMethod::Telex => "Telex",
            InputMethod::Vni => "VNI",
        };
        let tip = if self.config.vietnamese {
            format!("GoViet: Tiếng Việt ({method}) · Ctrl+Shift để tắt")
        } else {
            "GoViet: English · Ctrl+Shift để bật tiếng Việt".to_string()
        };
        copy_into(&mut nid.szTip, &tip);
        unsafe {
            Shell_NotifyIconW(action, &nid);
        }
    }

    fn remove_tray(&self) {
        let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = self.hwnd;
        nid.uID = TRAY_ID;
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &nid);
        }
    }

    fn command(&mut self, id: usize) {
        match id {
            CMD_TOGGLE => self.toggle(),
            CMD_TELEX => {
                self.config.method = InputMethod::Telex;
                self.apply_config();
            }
            CMD_VNI => {
                self.config.method = InputMethod::Vni;
                self.apply_config();
            }
            CMD_MODERN => {
                self.config.modern_tone = !self.config.modern_tone;
                self.apply_config();
            }
            CMD_STARTUP => startup::set_enabled(!startup::is_enabled()),
            _ => {}
        }
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        if kb.dwExtraInfo != INJECTED_MARK
            && with_app(|app| app.on_key(wparam as u32, kb)).unwrap_or(false)
        {
            return 1;
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32
        && matches!(
            wparam as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN
        )
    {
        // Click chuột có thể dời con trỏ: bắt đầu từ mới.
        with_app(|app| app.engine.reset());
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

fn menu_item(menu: *mut core::ffi::c_void, id: usize, text: &str, checked: bool) {
    let t = wide(text);
    let flags = MF_STRING | if checked { MF_CHECKED } else { 0 };
    unsafe {
        AppendMenuW(menu, flags, id, t.as_ptr());
    }
}

fn menu_separator(menu: *mut core::ffi::c_void) {
    unsafe {
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
    }
}

fn show_menu(hwnd: HWND) {
    // Đọc trạng thái rồi trả lại ngay: menu chạy vòng lặp riêng, trong lúc đó hook vẫn cần APP.
    let Some(config) = with_app(|app| app.config) else {
        return;
    };
    let on_startup = startup::is_enabled();
    unsafe {
        let menu = CreatePopupMenu();
        menu_item(
            menu,
            CMD_TOGGLE,
            "Bật tiếng Việt\tCtrl+Shift",
            config.vietnamese,
        );
        menu_separator(menu);
        menu_item(
            menu,
            CMD_TELEX,
            "Kiểu gõ Telex",
            config.method == InputMethod::Telex,
        );
        menu_item(
            menu,
            CMD_VNI,
            "Kiểu gõ VNI",
            config.method == InputMethod::Vni,
        );
        menu_separator(menu);
        menu_item(
            menu,
            CMD_MODERN,
            "Đặt dấu kiểu mới (oà, uý)",
            config.modern_tone,
        );
        menu_item(menu, CMD_STARTUP, "Khởi động cùng Windows", on_startup);
        menu_separator(menu);
        menu_item(menu, CMD_ABOUT, "Giới thiệu GoViet", false);
        menu_item(menu, CMD_EXIT, "Thoát", false);

        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            0,
            hwnd,
            null(),
        ) as usize;
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);

        match cmd {
            0 => {}
            CMD_ABOUT => {
                let text = wide(&format!(
                    "GoViet {}\nBộ gõ tiếng Việt cho Windows.\n\nCtrl+Shift: bật/tắt tiếng Việt\nhttps://github.com/zenix-vn/go-viet-windows",
                    env!("CARGO_PKG_VERSION")
                ));
                let title = wide("Giới thiệu GoViet");
                MessageBoxW(
                    hwnd,
                    text.as_ptr(),
                    title.as_ptr(),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
            CMD_EXIT => {
                DestroyWindow(hwnd);
            }
            id => {
                with_app(|app| app.command(id));
            }
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAY => {
            match (lparam as u32) & 0xFFFF {
                WM_LBUTTONUP => {
                    with_app(|app| app.toggle());
                }
                WM_RBUTTONUP => show_menu(hwnd),
                _ => {}
            }
            0
        }
        WM_COMMAND => {
            let id = wparam & 0xFFFF;
            with_app(|app| app.command(id));
            0
        }
        WM_DESTROY => {
            with_app(|app| app.remove_tray());
            PostQuitMessage(0);
            0
        }
        _ => {
            // Explorer khởi động lại → thêm lại icon khay.
            if msg != 0 && msg == TASKBAR_CREATED.with(|t| *t.borrow()) {
                with_app(|app| app.update_tray(NIM_ADD));
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
}

pub fn run() {
    unsafe {
        // Chỉ cho phép một bản GoViet chạy.
        let mutex_name = wide("Local\\GoViet.SingleInstance");
        let _mutex = CreateMutexW(null(), 0, mutex_name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }
        // Icon khay nét trên màn hình độ phân giải cao.
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        let instance = GetModuleHandleW(null());
        let class_name = wide("GoVietTrayWindow");
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(window_proc);
        wc.hInstance = instance;
        wc.lpszClassName = class_name.as_ptr();
        RegisterClassW(&wc);

        let title = wide("GoViet");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        if hwnd.is_null() {
            return;
        }

        let taskbar = wide("TaskbarCreated");
        TASKBAR_CREATED.with(|t| *t.borrow_mut() = RegisterWindowMessageW(taskbar.as_ptr()));

        let config = Config::load();
        let app = App {
            engine: Engine::new(config.engine_options()),
            config,
            hwnd,
            icon_v: letter_icon("V", BLUE),
            icon_e: letter_icon("E", RED),
            last_window: null_mut(),
            chord: Chord::default(),
        };
        app.update_tray(NIM_ADD);
        APP.with(|cell| *cell.borrow_mut() = Some(app));

        let kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), instance, 0);
        let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), instance, 0);
        HOOKS.with(|h| *h.borrow_mut() = (kb, mouse));

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        HOOKS.with(|h| {
            let (kb, mouse) = *h.borrow();
            UnhookWindowsHookEx(kb);
            UnhookWindowsHookEx(mouse);
        });
        if let Some(app) = APP.with(|cell| cell.borrow_mut().take()) {
            DestroyIcon(app.icon_v);
            DestroyIcon(app.icon_e);
        }
    }
}
