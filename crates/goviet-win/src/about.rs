//! Cửa sổ "Giới thiệu GoViet": icon, phiên bản, phím tắt, tác giả Zenix Labs.
//!
//! Dùng TaskDialog (có liên kết bấm được). TaskDialog chỉ có trong Common Controls v6,
//! nên hàm được nạp lúc chạy; nếu không có thì dùng hộp thông báo thường.

use crate::wide::wide;
use std::ptr::{null, null_mut};
use windows_sys::core::HRESULT;
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows_sys::Win32::UI::Controls::{
    TASKDIALOGCONFIG, TASKDIALOGCONFIG_0, TASKDIALOGCONFIG_1, TASKDIALOG_NOTIFICATIONS,
    TDCBF_CLOSE_BUTTON, TDF_ALLOW_DIALOG_CANCELLATION, TDF_ENABLE_HYPERLINKS, TDF_USE_HICON_MAIN,
    TDN_HYPERLINK_CLICKED,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, MessageBoxIndirectW, HICON, MB_OK, MB_USERICON, MSGBOXPARAMSW, SM_CXICON,
    SW_SHOWNORMAL,
};

pub const AUTHOR: &str = "Zenix Labs";
pub const AUTHOR_URL: &str = "https://zenix.vn";
pub const SOURCE_URL: &str = "https://github.com/zenix-vn/go-viet-windows";

type TaskDialogIndirectFn =
    unsafe extern "system" fn(*const TASKDIALOGCONFIG, *mut i32, *mut i32, *mut BOOL) -> HRESULT;

fn open_url(url: *const u16) {
    let verb = wide("open");
    unsafe {
        ShellExecuteW(
            null_mut(),
            verb.as_ptr(),
            url,
            null(),
            null(),
            SW_SHOWNORMAL,
        );
    }
}

unsafe extern "system" fn callback(
    _hwnd: HWND,
    msg: TASKDIALOG_NOTIFICATIONS,
    _wparam: WPARAM,
    lparam: LPARAM,
    _data: isize,
) -> HRESULT {
    if msg == TDN_HYPERLINK_CLICKED {
        open_url(lparam as *const u16);
    }
    0
}

fn content() -> String {
    "Bộ gõ tiếng Việt cho Windows: nhẹ, nhanh, miễn phí.\n\n\
     Ctrl+Shift: bật/tắt tiếng Việt (hoặc click vào icon)\n\
     Ctrl+Shift+F9: chuyển mã clipboard như lần trước\n\
     Chuột phải vào icon: kiểu gõ, gõ tắt, chuyển mã, khởi động cùng Windows"
        .to_string()
}

pub fn show(owner: HWND, icon_id: u16) {
    let version = env!("CARGO_PKG_VERSION");
    unsafe {
        let lib = wide("comctl32.dll");
        let module = LoadLibraryW(lib.as_ptr());
        let proc = if module.is_null() {
            None
        } else {
            GetProcAddress(module, c"TaskDialogIndirect".as_ptr().cast())
        };

        let Some(proc) = proc else {
            show_fallback(owner, icon_id, version);
            return;
        };
        let task_dialog: TaskDialogIndirectFn = std::mem::transmute(proc);

        let icon: HICON = crate::app::load_icon(icon_id, SM_CXICON, ("V", crate::icon::BLUE));
        let title = wide("Giới thiệu GoViet");
        let main = wide(&format!("GoViet {version}"));
        let body = wide(&content());
        let footer = wide(&format!(
            "Phát triển bởi <a href=\"{AUTHOR_URL}\">{AUTHOR}</a> · \
             <a href=\"{SOURCE_URL}\">Mã nguồn trên GitHub</a> · Giấy phép MIT"
        ));

        let mut config: TASKDIALOGCONFIG = std::mem::zeroed();
        config.cbSize = std::mem::size_of::<TASKDIALOGCONFIG>() as u32;
        config.hwndParent = owner;
        config.hInstance = GetModuleHandleW(null());
        config.dwFlags = TDF_ENABLE_HYPERLINKS | TDF_USE_HICON_MAIN | TDF_ALLOW_DIALOG_CANCELLATION;
        config.dwCommonButtons = TDCBF_CLOSE_BUTTON;
        config.pszWindowTitle = title.as_ptr();
        config.Anonymous1 = TASKDIALOGCONFIG_0 { hMainIcon: icon };
        config.pszMainInstruction = main.as_ptr();
        config.pszContent = body.as_ptr();
        config.Anonymous2 = TASKDIALOGCONFIG_1 {
            hFooterIcon: null_mut(),
        };
        config.pszFooter = footer.as_ptr();
        config.pfCallback = Some(callback);

        let hr = task_dialog(&config, null_mut(), null_mut(), null_mut());
        DestroyIcon(icon);
        if hr < 0 {
            show_fallback(owner, icon_id, version);
        }
    }
}

fn show_fallback(owner: HWND, icon_id: u16, version: &str) {
    let text = wide(&format!(
        "GoViet {version}\n{}\n\nPhát triển bởi {AUTHOR} · {AUTHOR_URL}\nMã nguồn: {SOURCE_URL}\nGiấy phép MIT",
        content()
    ));
    let title = wide("Giới thiệu GoViet");
    let params = MSGBOXPARAMSW {
        cbSize: std::mem::size_of::<MSGBOXPARAMSW>() as u32,
        hwndOwner: owner,
        hInstance: unsafe { GetModuleHandleW(null()) },
        lpszText: text.as_ptr(),
        lpszCaption: title.as_ptr(),
        dwStyle: MB_OK | MB_USERICON,
        lpszIcon: icon_id as usize as *const u16,
        dwContextHelpId: 0,
        lpfnMsgBoxCallback: None,
        dwLanguageId: 0,
    };
    unsafe {
        MessageBoxIndirectW(&params);
    }
}
