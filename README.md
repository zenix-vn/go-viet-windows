# GoViet — bộ gõ tiếng Việt cho Windows

GoViet là bộ gõ tiếng Việt gọn nhẹ như Unikey: một file `GoViet.exe`, chạy ở khay hệ thống, gõ Telex hoặc VNI ra Unicode trong mọi ứng dụng. Viết bằng Rust, không cần cài runtime.

## Tính năng (v0.1)

- Kiểu gõ **Telex** và **VNI**, bảng mã Unicode dựng sẵn
- Bỏ dấu tự do: `tieengs`, `tiesng`, `tiengse` đều ra `tiếng`
- Đặt dấu kiểu cũ (`hòa`, `thủy`, mặc định như Unikey) hoặc kiểu mới (`hoà`, `thuỷ`)
- Gõ lặp để hủy dấu: `ass` → `as`, `aaa` → `aa`, `ddd` → `dd`
- Tự giữ nguyên từ tiếng Anh: `class`, `text`, `window`, `email` không bị biến dạng
- Backspace xong vẫn bỏ dấu tiếp được cho từ đang gõ
- **Ctrl+Shift** hoặc click icon khay để bật/tắt tiếng Việt (icon **V** xanh / **E** đỏ)
- Menu chuột phải: Telex/VNI, kiểu đặt dấu, khởi động cùng Windows
- Lưu cấu hình tại `%APPDATA%\GoViet\config.toml`

## Cách gõ

| Dấu | Telex | VNI |
| --- | --- | --- |
| sắc, huyền, hỏi, ngã, nặng | `s` `f` `r` `x` `j` | `1` `2` `3` `4` `5` |
| xóa dấu thanh | `z` | `0` |
| â ê ô | `aa` `ee` `oo` | `a6` `e6` `o6` |
| ă | `aw` | `a8` |
| ơ ư ươ | `ow` `uw` `uow` (hoặc `w` → `ư`) | `o7` `u7` `uo7` |
| đ | `dd` | `d9` |

## Tải về

Bản build mới nhất nằm ở mục **Actions → CI → Artifacts** (GoViet-windows-x64), hoặc **Releases** khi có tag `v*`.

## Build từ mã nguồn

Cần [Rust](https://rustup.rs) (stable) trên Windows 10/11:

```powershell
cargo build --release -p goviet-win
# file chạy: target\release\GoViet.exe
```

Gõ thử engine trên mọi hệ điều hành (`<` là Backspace):

```bash
cargo test --workspace
echo "tieengs Vieetj" | cargo run -p goviet-cli          # tiếng Việt
echo "tie61ng Vie65t" | cargo run -p goviet-cli -- --vni
```

Kiểm tra kiểu phần mã Windows trên Linux/macOS: `cargo check -p goviet-win --features check-on-other-os`.

## Cấu trúc

```
crates/
  goviet-engine/   Engine thuần Rust: Telex/VNI, đặt dấu, kiểm tra âm tiết (không phụ thuộc Windows)
  goviet-win/      App Windows: hook bàn phím/chuột, SendInput, icon khay, menu, cấu hình
  goviet-cli/      Gõ thử engine trên terminal
```

Cơ chế giống Unikey: `WH_KEYBOARD_LL` bắt phím → engine tính từ có dấu → `SendInput` gửi Backspace để xóa phần cũ rồi gõ chuỗi Unicode mới. Phím do GoViet gửi được đánh dấu bằng `dwExtraInfo` để hook bỏ qua. Click chuột, đổi cửa sổ, phím mũi tên hay dấu câu đều bắt đầu từ mới.

## Hạn chế đã biết

- Ứng dụng chạy quyền Administrator không nhận phím từ GoViet thường (giới hạn UIPI của Windows). Chạy GoViet bằng quyền Admin nếu cần gõ trong các ứng dụng đó.
- Ô có tự gợi ý (thanh địa chỉ trình duyệt, ô Excel) đôi khi nuốt Backspace. Sẽ xử lý ở bản sau.
- File .exe chưa ký số nên Windows SmartScreen có thể cảnh báo lần đầu chạy.

## Lộ trình

- [ ] Sửa lỗi ô tự gợi ý trên trình duyệt / Excel
- [ ] Gõ tắt (macro), danh sách ứng dụng loại trừ
- [ ] Bảng mã TCVN3, VNI-Windows, chuyển mã clipboard
- [ ] Icon và thông tin phiên bản nhúng vào .exe, bộ cài đặt, ký số

## Giấy phép

MIT
