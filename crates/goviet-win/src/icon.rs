//! Vẽ icon khay hệ thống (chữ V / E trên nền màu) lúc chạy, không cần file .ico.

use crate::wide::wide;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{RECT, TRUE};
use windows_sys::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush, DeleteDC,
    DeleteObject, DrawTextW, FillRect, GetDC, ReleaseDC, SelectObject, SetBkMode, SetTextColor,
    ANTIALIASED_QUALITY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CLIP_DEFAULT_PRECIS,
    DEFAULT_CHARSET, DIB_RGB_COLORS, DT_CENTER, DT_SINGLELINE, DT_VCENTER, FF_SWISS, FW_BOLD,
    OUT_DEFAULT_PRECIS, TRANSPARENT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, GetSystemMetrics, HICON, ICONINFO, SM_CXSMICON,
};

/// Màu dạng COLORREF (0x00BBGGRR).
const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub const BLUE: u32 = rgb(0x1D, 0x4E, 0xD8);
pub const RED: u32 = rgb(0xC6, 0x28, 0x28);

pub fn letter_icon(letter: &str, background: u32) -> HICON {
    unsafe {
        let size = GetSystemMetrics(SM_CXSMICON).max(16);
        let screen = GetDC(null_mut());
        let dc = CreateCompatibleDC(screen);

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size,
            biHeight: -size, // từ trên xuống
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        let mut bits: *mut core::ffi::c_void = null_mut();
        let color = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        let old_bitmap = SelectObject(dc, color);

        let mut rc = RECT {
            left: 0,
            top: 0,
            right: size,
            bottom: size,
        };
        let brush = CreateSolidBrush(background);
        FillRect(dc, &rc, brush);
        DeleteObject(brush);

        let face = wide("Segoe UI");
        let font = CreateFontW(
            -(size * 7 / 8),
            0,
            0,
            0,
            FW_BOLD as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            ANTIALIASED_QUALITY as u32,
            FF_SWISS as u32,
            face.as_ptr(),
        );
        let old_font = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, rgb(0xFF, 0xFF, 0xFF));
        let text = wide(letter);
        DrawTextW(
            dc,
            text.as_ptr(),
            -1,
            &mut rc,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        SelectObject(dc, old_font);
        DeleteObject(font);

        // GDI không ghi kênh alpha: đặt alpha = 255 cho mọi điểm ảnh để icon không bị trong suốt.
        if !bits.is_null() {
            let px = std::slice::from_raw_parts_mut(bits as *mut u32, (size * size) as usize);
            for p in px.iter_mut() {
                *p |= 0xFF00_0000;
            }
        }
        SelectObject(dc, old_bitmap);

        // Mặt nạ đơn sắc toàn 0 = mọi điểm ảnh đều hiện.
        let stride = ((size + 15) / 16 * 2) as usize;
        let mask_bits = vec![0u8; stride * size as usize];
        let mask = CreateBitmap(size, size, 1, 1, mask_bits.as_ptr().cast());

        let info = ICONINFO {
            fIcon: TRUE,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: color,
        };
        let icon = CreateIconIndirect(&info);

        DeleteObject(mask);
        DeleteObject(color);
        DeleteDC(dc);
        ReleaseDC(null_mut(), screen);
        icon
    }
}
