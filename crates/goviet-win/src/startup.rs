//! Khởi động cùng Windows qua khóa `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

use crate::wide::wide;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "GoViet";

pub fn is_enabled() -> bool {
    let key = wide(RUN_KEY);
    let name = wide(VALUE_NAME);
    let mut size: u32 = 0;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            null_mut(),
            &mut size,
        )
    };
    status == ERROR_SUCCESS
}

pub fn set_enabled(enable: bool) {
    let key = wide(RUN_KEY);
    let name = wide(VALUE_NAME);
    unsafe {
        if enable {
            let Ok(exe) = std::env::current_exe() else {
                return;
            };
            let data = wide(&format!("\"{}\"", exe.display()));
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                REG_SZ,
                data.as_ptr().cast(),
                (data.len() * 2) as u32,
            );
        } else {
            RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr());
        }
    }
}
