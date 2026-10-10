//! Hộp thoại chính: bật/tắt tiếng Việt, kiểu gõ, tùy chọn. Hiện khi mở GoViet.
//!
//! Giao diện nằm trong tài nguyên của GoViet.exe (`res/dialog.rc`), nên Windows tự lo
//! co giãn theo DPI và phím Tab/Esc. Bấm Đóng chỉ ẩn hộp thoại, GoViet vẫn chạy dưới khay.

use crate::app::{self, ICON_ON};
use crate::config::Config;
use crate::icon::BLUE;
use crate::startup;
use crate::wide::wide;
use goviet_engine::InputMethod;
use std::cell::Cell;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontIndirectW, CreateSolidBrush, DeleteObject, FillRect, GetObjectW, GetSysColor,
    GetSysColorBrush, SetBkColor, SetTextColor, COLOR_BTNFACE, COLOR_GRAYTEXT, COLOR_WINDOW, HDC,
    HFONT, LOGFONTW,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{
    CheckDlgButton, CheckRadioButton, IsDlgButtonChecked, BST_CHECKED, BST_UNCHECKED,
};
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateDialogParamW, DestroyIcon, DestroyWindow, GetClientRect, GetDlgCtrlID, IsDialogMessageW,
    IsIconic, LoadImageW, MapDialogRect, PostMessageW, SendDlgItemMessageW, SendMessageW,
    SetForegroundWindow, SetWindowLongPtrW, ShowWindow, BN_CLICKED, CBN_SELCHANGE, CB_ADDSTRING,
    CB_GETCURSEL, CB_SETCURSEL, DM_SETDEFID, DWLP_MSGRESULT, HICON, ICON_BIG, ICON_SMALL, IDCANCEL,
    IDOK, IMAGE_ICON, LR_DEFAULTCOLOR, MSG, SM_CXICON, SM_CXSMICON, STM_SETICON, SW_RESTORE,
    SW_SHOW, WM_APP, WM_CLOSE, WM_COMMAND, WM_CTLCOLORDLG, WM_CTLCOLORSTATIC, WM_DESTROY,
    WM_DPICHANGED, WM_ERASEBKGND, WM_GETFONT, WM_INITDIALOG, WM_SETFONT, WM_SETICON,
};

/// Mã tài nguyên và điều khiển, khớp với `res/dialog.rc`.
const DIALOG_ID: u16 = 100;
const IDC_VIETNAMESE: i32 = 201;
const IDC_ENGLISH: i32 = 202;
const IDC_METHOD: i32 = 203;
const IDC_MODERN: i32 = 204;
const IDC_MACROS: i32 = 205;
const IDC_EDIT_MACROS: i32 = 206;
const IDC_STARTUP: i32 = 207;
const IDC_SHOW_DIALOG: i32 = 208;
const IDC_ABOUT: i32 = 209;
const IDC_EXIT: i32 = 210;
const IDC_ICON: i32 = 211;
const IDC_TITLE: i32 = 212;
const IDC_HINT: i32 = 214;
/// Mép trên của dải nút phía dưới, theo đơn vị hộp thoại.
const BUTTON_BAND_TOP: i32 = 218;

/// Dựng lại font tiêu đề và icon sau khi đổi DPI (đợi Windows co giãn xong các điều khiển).
const WM_REFRESH_DPI: u32 = WM_APP + 10;

#[derive(Clone, Copy)]
struct State {
    hwnd: HWND,
    title_font: HFONT,
    header_icon: HICON,
    icon_big: HICON,
    icon_small: HICON,
}

thread_local! {
    static STATE: Cell<Option<State>> = const { Cell::new(None) };
}

fn state() -> Option<State> {
    STATE.with(Cell::get)
}

/// Hiện hộp thoại (tạo mới nếu chưa có) và đưa lên trước.
pub fn show() {
    unsafe {
        let hwnd = match state() {
            Some(s) => s.hwnd,
            None => {
                let hwnd = CreateDialogParamW(
                    GetModuleHandleW(null()),
                    DIALOG_ID as usize as *const u16,
                    null_mut(),
                    Some(dialog_proc),
                    0,
                );
                if hwnd.is_null() {
                    return;
                }
                hwnd
            }
        };
        ShowWindow(
            hwnd,
            if IsIconic(hwnd) != 0 {
                SW_RESTORE
            } else {
                SW_SHOW
            },
        );
        SetForegroundWindow(hwnd);
    }
}

/// Cho hộp thoại xử lý Tab, Enter, Esc. Trả `true` nếu thông điệp đã được xử lý.
pub fn handle_message(msg: &MSG) -> bool {
    match state() {
        Some(s) => unsafe { IsDialogMessageW(s.hwnd, msg) != 0 },
        None => false,
    }
}

/// Cập nhật các điều khiển theo cấu hình (khi đổi bằng phím tắt hoặc menu khay).
pub fn refresh(config: &Config) {
    let Some(s) = state() else {
        return;
    };
    let hwnd = s.hwnd;
    let check = |id: i32, on: bool| unsafe {
        CheckDlgButton(hwnd, id, if on { BST_CHECKED } else { BST_UNCHECKED });
    };
    unsafe {
        CheckRadioButton(
            hwnd,
            IDC_VIETNAMESE,
            IDC_ENGLISH,
            if config.vietnamese {
                IDC_VIETNAMESE
            } else {
                IDC_ENGLISH
            },
        );
        let method = match config.method {
            InputMethod::Telex => 0,
            InputMethod::Vni => 1,
        };
        SendDlgItemMessageW(hwnd, IDC_METHOD, CB_SETCURSEL, method, 0);
    }
    check(IDC_MODERN, config.modern_tone);
    check(IDC_MACROS, config.macros);
    check(IDC_STARTUP, startup::is_enabled());
    check(IDC_SHOW_DIALOG, config.show_dialog);
}

fn checked(hwnd: HWND, id: i32) -> bool {
    unsafe { IsDlgButtonChecked(hwnd, id) == BST_CHECKED }
}

fn load_icon_px(size: i32) -> HICON {
    unsafe {
        LoadImageW(
            GetModuleHandleW(null()),
            ICON_ON as usize as *const u16,
            IMAGE_ICON,
            size,
            size,
            LR_DEFAULTCOLOR,
        )
    }
}

/// Font tiêu đề: font của hộp thoại, to hơn và đậm hơn. Icon đầu trang 32 px theo DPI.
fn update_header(hwnd: HWND, s: &mut State) {
    unsafe {
        let font = SendMessageW(hwnd, WM_GETFONT, 0, 0) as HFONT;
        let mut lf: LOGFONTW = std::mem::zeroed();
        if !font.is_null()
            && GetObjectW(
                font,
                std::mem::size_of::<LOGFONTW>() as i32,
                (&mut lf as *mut LOGFONTW).cast(),
            ) != 0
        {
            lf.lfHeight = lf.lfHeight * 5 / 3;
            lf.lfWeight = 600;
            let title_font = CreateFontIndirectW(&lf);
            SendDlgItemMessageW(hwnd, IDC_TITLE, WM_SETFONT, title_font as usize, 1);
            if !s.title_font.is_null() {
                DeleteObject(s.title_font);
            }
            s.title_font = title_font;
        }

        let size = 32 * GetDpiForWindow(hwnd) as i32 / 96;
        let icon = load_icon_px(size);
        if !icon.is_null() {
            SendDlgItemMessageW(hwnd, IDC_ICON, STM_SETICON, icon as usize, 0);
            if !s.header_icon.is_null() {
                DestroyIcon(s.header_icon);
            }
            s.header_icon = icon;
        }
    }
}

fn init(hwnd: HWND) {
    let mut s = State {
        hwnd,
        title_font: null_mut(),
        header_icon: null_mut(),
        icon_big: app::load_icon(ICON_ON, SM_CXICON, ("V", BLUE)),
        icon_small: app::load_icon(ICON_ON, SM_CXSMICON, ("V", BLUE)),
    };
    unsafe {
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, s.icon_big as isize);
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, s.icon_small as isize);
        for name in ["Telex", "VNI"] {
            let text = wide(name);
            SendDlgItemMessageW(hwnd, IDC_METHOD, CB_ADDSTRING, 0, text.as_ptr() as isize);
        }
    }
    update_header(hwnd, &mut s);
    unsafe {
        SendMessageW(hwnd, DM_SETDEFID, IDCANCEL as usize, 0);
    }
    STATE.with(|c| c.set(Some(s)));
    if let Some(config) = app::config() {
        refresh(&config);
    }
}

fn on_command(hwnd: HWND, id: i32, code: u32) {
    match (id, code) {
        (IDC_VIETNAMESE | IDC_ENGLISH, BN_CLICKED) => {
            let on = id == IDC_VIETNAMESE;
            app::update_config(|c| c.vietnamese = on);
        }
        (IDC_METHOD, CBN_SELCHANGE) => {
            let sel = unsafe { SendDlgItemMessageW(hwnd, IDC_METHOD, CB_GETCURSEL, 0, 0) };
            let method = if sel == 1 {
                InputMethod::Vni
            } else {
                InputMethod::Telex
            };
            app::update_config(|c| c.method = method);
        }
        (IDC_MODERN, BN_CLICKED) => {
            let on = checked(hwnd, id);
            app::update_config(|c| c.modern_tone = on);
        }
        (IDC_MACROS, BN_CLICKED) => {
            let on = checked(hwnd, id);
            app::update_config(|c| c.macros = on);
        }
        (IDC_SHOW_DIALOG, BN_CLICKED) => {
            let on = checked(hwnd, id);
            app::update_config(|c| c.show_dialog = on);
        }
        (IDC_STARTUP, BN_CLICKED) => {
            startup::set_enabled(checked(hwnd, id));
            if let Some(config) = app::config() {
                refresh(&config);
            }
        }
        (IDC_EDIT_MACROS, BN_CLICKED) => app::edit_macros(),
        (IDC_ABOUT, BN_CLICKED) => crate::about::show(hwnd, ICON_ON),
        (IDC_EXIT, BN_CLICKED) => app::exit(),
        (IDOK | IDCANCEL, _) => unsafe {
            DestroyWindow(hwnd);
        },
        _ => {}
    }
}

/// Nền trắng, dải nút phía dưới màu xám nhạt có viền trên (giống hộp thoại của Windows).
fn paint_background(hwnd: HWND, hdc: HDC) {
    unsafe {
        let mut client = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetClientRect(hwnd, &mut client);
        let mut band = RECT {
            left: 0,
            top: BUTTON_BAND_TOP,
            right: 0,
            bottom: 0,
        };
        MapDialogRect(hwnd, &mut band);
        let top = band.top;

        FillRect(
            hdc,
            &RECT {
                bottom: top,
                ..client
            },
            GetSysColorBrush(COLOR_WINDOW),
        );
        FillRect(
            hdc,
            &RECT { top, ..client },
            GetSysColorBrush(COLOR_BTNFACE),
        );
        let border = CreateSolidBrush(0x00DF_DFDF);
        FillRect(
            hdc,
            &RECT {
                top,
                bottom: top + 1,
                ..client
            },
            border,
        );
        DeleteObject(border);
    }
}

unsafe extern "system" fn dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    match msg {
        WM_INITDIALOG => {
            init(hwnd);
            1
        }
        WM_COMMAND => {
            on_command(
                hwnd,
                (wparam & 0xFFFF) as i32,
                ((wparam >> 16) & 0xFFFF) as u32,
            );
            1
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            1
        }
        WM_ERASEBKGND => {
            paint_background(hwnd, wparam as HDC);
            SetWindowLongPtrW(hwnd, DWLP_MSGRESULT as i32, 1);
            1
        }
        WM_CTLCOLORDLG => GetSysColorBrush(COLOR_WINDOW) as isize,
        WM_CTLCOLORSTATIC => {
            let hdc = wparam as HDC;
            SetBkColor(hdc, GetSysColor(COLOR_WINDOW));
            if GetDlgCtrlID(lparam as HWND) == IDC_HINT {
                SetTextColor(hdc, GetSysColor(COLOR_GRAYTEXT));
            }
            GetSysColorBrush(COLOR_WINDOW) as isize
        }
        WM_DPICHANGED => {
            PostMessageW(hwnd, WM_REFRESH_DPI, 0, 0);
            0
        }
        WM_REFRESH_DPI => {
            if let Some(mut s) = state() {
                update_header(hwnd, &mut s);
                STATE.with(|c| c.set(Some(s)));
            }
            1
        }
        WM_DESTROY => {
            if let Some(s) = STATE.with(|c| c.take()) {
                DeleteObject(s.title_font);
                DestroyIcon(s.header_icon);
                DestroyIcon(s.icon_big);
                DestroyIcon(s.icon_small);
            }
            0
        }
        _ => 0,
    }
}
