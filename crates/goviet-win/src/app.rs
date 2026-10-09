//! Vòng đời ứng dụng: cửa sổ ẩn, icon khay, menu, hook bàn phím/chuột, phím tắt.

use crate::clipboard;
use crate::config::Config;
use crate::icon::{letter_icon, BLUE, RED};
use crate::send::{self, INJECTED_MARK};
use crate::startup;
use crate::wide::{copy_into, wide};
use goviet_engine::{convert, Action, Conversion, Engine, InputMethod, Macros, DEFAULT_MACROS};
use std::cell::RefCell;
use std::ptr::{null, null_mut};
use std::time::SystemTime;
use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, RegisterHotKey, UnregisterHotKey, MOD_CONTROL, MOD_NOREPEAT,
    MOD_SHIFT, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_F9, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN,
    VK_MENU, VK_NUMPAD0, VK_NUMPAD9, VK_OEM_1, VK_OEM_102, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5,
    VK_OEM_6, VK_OEM_7, VK_OEM_8, VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS,
    VK_RCONTROL, VK_RETURN, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB,
};
use windows_sys::Win32::UI::Shell::{
    ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO,
    NIIF_WARNING, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CallNextHookEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon,
    DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, GetForegroundWindow, GetMessageW,
    GetSystemMetrics, LoadImageW, PostMessageW, PostQuitMessage, RegisterClassW,
    RegisterWindowMessageW, SetForegroundWindow, SetWindowsHookExW, TrackPopupMenu,
    TranslateMessage, UnhookWindowsHookEx, HC_ACTION, HHOOK, HICON, IMAGE_ICON, KBDLLHOOKSTRUCT,
    LR_DEFAULTCOLOR, MF_CHECKED, MF_POPUP, MF_SEPARATOR, MF_STRING, MSG, SM_CXICON, SM_CXSMICON,
    SW_SHOWNORMAL, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WH_KEYBOARD_LL, WH_MOUSE_LL,
    WM_APP, WM_COMMAND, WM_DESTROY, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_NULL, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDCLASSW,
    WS_OVERLAPPED,
};

const WM_TRAY: u32 = WM_APP + 1;
const TRAY_ID: u32 = 1;
const HOTKEY_CONVERT: i32 = 1;

/// Mã tài nguyên icon trong GoViet.exe (xem build.rs).
const ICON_ON: u16 = 1;
const ICON_OFF: u16 = 2;

const CMD_TOGGLE: usize = 1001;
const CMD_TELEX: usize = 1002;
const CMD_VNI: usize = 1003;
const CMD_MODERN: usize = 1004;
const CMD_STARTUP: usize = 1005;
const CMD_ABOUT: usize = 1006;
const CMD_EXIT: usize = 1007;
const CMD_MACROS: usize = 1008;
const CMD_EDIT_MACROS: usize = 1009;
const CMD_CONVERT_LAST: usize = 1100;
/// `CMD_CONVERT_BASE + i` = `Conversion::ALL[i]`.
const CMD_CONVERT_BASE: usize = 1101;

/// Trạng thái phím tắt Ctrl+Shift: bật/tắt khi nhả phím mà không bấm phím/chuột nào khác.
#[derive(Default)]
struct Chord {
    armed: bool,
    dirty: bool,
}

struct App {
    engine: Engine,
    config: Config,
    macros: Macros,
    macros_modified: Option<SystemTime>,
    hwnd: HWND,
    icon_on: HICON,
    icon_off: HICON,
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

/// Phím kết thúc từ và kích hoạt gõ tắt: dấu cách, Enter, Tab, dấu câu.
fn is_macro_trigger(vk: u16) -> bool {
    matches!(
        vk,
        VK_SPACE
            | VK_RETURN
            | VK_TAB
            | VK_OEM_1
            | VK_OEM_2
            | VK_OEM_3
            | VK_OEM_4
            | VK_OEM_5
            | VK_OEM_6
            | VK_OEM_7
            | VK_OEM_8
            | VK_OEM_102
            | VK_OEM_PLUS
            | VK_OEM_COMMA
            | VK_OEM_MINUS
            | VK_OEM_PERIOD
            | 0x30..=0x39 // Shift + số: ! @ # ...
    )
}

/// Icon nhúng trong GoViet.exe; dùng icon vẽ tạm nếu không tải được.
pub(crate) fn load_icon(id: u16, size_metric: i32, fallback: (&str, u32)) -> HICON {
    unsafe {
        let size = GetSystemMetrics(size_metric);
        let icon = LoadImageW(
            GetModuleHandleW(null()),
            id as usize as *const u16,
            IMAGE_ICON,
            size,
            size,
            LR_DEFAULTCOLOR,
        );
        if icon.is_null() {
            letter_icon(fallback.0, fallback.1)
        } else {
            icon
        }
    }
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
            // Phím kết thúc từ: thử gõ tắt trước khi bắt đầu từ mới.
            if self.config.macros && is_macro_trigger(vk) && !self.engine.current_word().is_empty()
            {
                self.reload_macros_if_changed();
                if let Some(Action::Replace { backspaces, text }) =
                    self.engine.expand_macro(&self.macros)
                {
                    send::replace_then_key(backspaces, &text, Some(vk));
                    return true;
                }
            }
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

    fn reload_macros_if_changed(&mut self) {
        let Some(path) = Config::macros_path() else {
            return;
        };
        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if modified != self.macros_modified {
            self.macros_modified = modified;
            self.macros = std::fs::read_to_string(&path)
                .map(|t| Macros::parse(&t))
                .unwrap_or_default();
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

    fn tray_data(&self) -> NOTIFYICONDATAW {
        let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = self.hwnd;
        nid.uID = TRAY_ID;
        nid
    }

    fn update_tray(&self, action: u32) {
        let mut nid = self.tray_data();
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY;
        nid.hIcon = if self.config.vietnamese {
            self.icon_on
        } else {
            self.icon_off
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

    /// Thông báo nhỏ cạnh khay hệ thống.
    fn notify(&self, title: &str, text: &str, warning: bool) {
        let mut nid = self.tray_data();
        nid.uFlags = NIF_INFO;
        copy_into(&mut nid.szInfoTitle, title);
        copy_into(&mut nid.szInfo, text);
        nid.dwInfoFlags = if warning { NIIF_WARNING } else { NIIF_INFO };
        unsafe {
            Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }

    fn remove_tray(&self) {
        let nid = self.tray_data();
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &nid);
        }
    }

    fn convert_clipboard(&mut self, conv: Conversion) {
        if self.config.last_conversion != conv {
            self.config.last_conversion = conv;
            self.config.save();
        }
        match clipboard::read_text(self.hwnd) {
            None => self.notify("GoViet", "Clipboard không có văn bản để chuyển mã.", true),
            Some(text) => {
                let out = convert(&text, conv);
                if clipboard::write_text(self.hwnd, &out) {
                    self.notify("Đã chuyển mã clipboard", conv.label(), false);
                } else {
                    self.notify("GoViet", "Không ghi được vào clipboard, thử lại.", true);
                }
            }
        }
    }

    fn edit_macros(&mut self) {
        let Some(path) = Config::macros_path() else {
            return;
        };
        if !path.exists() {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&path, DEFAULT_MACROS);
        }
        let verb = wide("open");
        let notepad = wide("notepad.exe");
        let args = wide(&format!("\"{}\"", path.display()));
        unsafe {
            ShellExecuteW(
                self.hwnd,
                verb.as_ptr(),
                notepad.as_ptr(),
                args.as_ptr(),
                null(),
                SW_SHOWNORMAL,
            );
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
            CMD_MACROS => {
                self.config.macros = !self.config.macros;
                self.apply_config();
            }
            CMD_EDIT_MACROS => self.edit_macros(),
            CMD_STARTUP => startup::set_enabled(!startup::is_enabled()),
            CMD_CONVERT_LAST => self.convert_clipboard(self.config.last_conversion),
            id if (CMD_CONVERT_BASE..CMD_CONVERT_BASE + Conversion::ALL.len()).contains(&id) => {
                self.convert_clipboard(Conversion::ALL[id - CMD_CONVERT_BASE])
            }
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
        // Ctrl+Shift+click (chọn nhiều mục) không được tính là phím tắt bật/tắt.
        with_app(|app| {
            app.engine.reset();
            if app.chord.armed {
                app.chord.dirty = true;
            }
        });
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

type Menu = *mut core::ffi::c_void;

fn menu_item(menu: Menu, id: usize, text: &str, checked: bool) {
    let t = wide(text);
    let flags = MF_STRING | if checked { MF_CHECKED } else { 0 };
    unsafe {
        AppendMenuW(menu, flags, id, t.as_ptr());
    }
}

fn menu_separator(menu: Menu) {
    unsafe {
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
    }
}

fn menu_submenu(menu: Menu, sub: Menu, text: &str) {
    let t = wide(text);
    unsafe {
        AppendMenuW(menu, MF_POPUP, sub as usize, t.as_ptr());
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
        menu_item(
            menu,
            CMD_MODERN,
            "Đặt dấu kiểu mới (oà, uý)",
            config.modern_tone,
        );
        menu_separator(menu);
        menu_item(menu, CMD_MACROS, "Bật gõ tắt", config.macros);
        menu_item(menu, CMD_EDIT_MACROS, "Sửa bảng gõ tắt...", false);

        let convert_menu = CreatePopupMenu();
        menu_item(
            convert_menu,
            CMD_CONVERT_LAST,
            &format!("Lặp lại: {}\tCtrl+Shift+F9", config.last_conversion.label()),
            false,
        );
        menu_separator(convert_menu);
        for (i, conv) in Conversion::ALL.iter().enumerate() {
            if i == 3 {
                menu_separator(convert_menu);
            }
            menu_item(convert_menu, CMD_CONVERT_BASE + i, conv.label(), false);
        }
        menu_submenu(menu, convert_menu, "Chuyển mã clipboard");
        menu_separator(menu);
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
        DestroyMenu(menu); // hủy cả menu con

        match cmd {
            0 => {}
            CMD_ABOUT => crate::about::show(hwnd, ICON_ON),
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
        WM_HOTKEY => {
            if wparam as i32 == HOTKEY_CONVERT {
                with_app(|app| app.command(CMD_CONVERT_LAST));
            }
            0
        }
        WM_COMMAND => {
            let id = wparam & 0xFFFF;
            with_app(|app| app.command(id));
            0
        }
        WM_DESTROY => {
            UnregisterHotKey(hwnd, HOTKEY_CONVERT);
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
        // Manifest đã khai báo DPI awareness; gọi thêm cho trường hợp chạy bản build không có manifest.
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        let instance = GetModuleHandleW(null());
        let class_name = wide("GoVietTrayWindow");
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(window_proc);
        wc.hInstance = instance;
        wc.lpszClassName = class_name.as_ptr();
        wc.hIcon = load_icon(ICON_ON, SM_CXICON, ("V", BLUE));
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
        RegisterHotKey(
            hwnd,
            HOTKEY_CONVERT,
            MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT,
            VK_F9 as u32,
        );

        let config = Config::load();
        let mut app = App {
            engine: Engine::new(config.engine_options()),
            config,
            macros: Macros::default(),
            macros_modified: None,
            hwnd,
            icon_on: load_icon(ICON_ON, SM_CXSMICON, ("V", BLUE)),
            icon_off: load_icon(ICON_OFF, SM_CXSMICON, ("E", RED)),
            last_window: null_mut(),
            chord: Chord::default(),
        };
        app.reload_macros_if_changed();
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
            DestroyIcon(app.icon_on);
            DestroyIcon(app.icon_off);
        }
    }
}
