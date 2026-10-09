/// Chuỗi UTF-16 kết thúc bằng 0 cho Win32 API.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Chép `s` vào mảng cố định (ví dụ `szTip` của icon khay), cắt bớt nếu quá dài.
pub fn copy_into(dst: &mut [u16], s: &str) {
    let src: Vec<u16> = s.encode_utf16().take(dst.len().saturating_sub(1)).collect();
    dst[..src.len()].copy_from_slice(&src);
    dst[src.len()] = 0;
}
