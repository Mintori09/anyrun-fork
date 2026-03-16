# anyrun-applications

Plugin này cho phép bạn tìm kiếm và khởi chạy các ứng dụng từ các file desktop entry trên hệ thống của bạn.

## Tính năng

- Tìm kiếm mờ (fuzzy search) tên ứng dụng, mô tả và từ khóa.
- Hỗ trợ các hành động desktop (Desktop Actions).
- Tích hợp các lệnh hệ thống nhanh: Tắt máy, Khởi động lại, Khóa màn hình, v.v.
- Hỗ trợ chạy ứng dụng trong terminal.
- Cho phép xử lý lệnh thực thi thông qua script tùy chỉnh.

## Cách sử dụng

Plugin này hoạt động mặc định mà không cần tiền tố (prefix). Chỉ cần nhập tên ứng dụng bạn muốn tìm.

## Phụ thuộc

- `anyrun`
- Các file `.desktop` chuẩn (thường ở `/usr/share/applications` và `~/.local/share/applications`)

## Cấu hình

File cấu hình: `applications.ron`

```ron
Config(
  desktop_actions: true,
  max_entries: 5,
  hide_description: false,
  // Tùy chọn terminal để chạy các ứng dụng yêu cầu terminal
  terminal: Some(Terminal(
    command: "alacritty",
    args: "-e {}",
  )),
  // Tùy chọn script tiền xử lý lệnh thực thi
  // preprocess_exec_script: Some("/path/to/script.sh"),
)
```

### Các trường cấu hình:

- `desktop_actions`: (bool) Có hiển thị các hành động bổ sung của ứng dụng hay không.
- `max_entries`: (usize) Số lượng kết quả tối đa hiển thị.
- `hide_description`: (bool) Ẩn mô tả ứng dụng.
- `terminal`: (Option) Cấu hình terminal để chạy ứng dụng.
- `preprocess_exec_script`: (Option) Đường dẫn tới script xử lý chuỗi lệnh thực thi trước khi chạy.
