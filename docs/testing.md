# Kế hoạch Kiểm thử Tích hợp Tự động (Integrated Automation Testing Specification)

Phiên bản này được thiết kế cho môi trường CI/CD thực tế, tập trung vào:

- Kiểm thử hành vi (Behavior Testing)
- Kiểm thử giao tiếp Client ↔ Daemon ↔ Provider
- Kiểm thử khả năng chịu lỗi (Resilience Testing)
- Kiểm thử đồng thời (Concurrency Testing)
- Kiểm thử hiệu năng (Performance Regression Detection)

Tất cả các bài kiểm thử phải chạy hoàn toàn tự động trong môi trường headless, không yêu cầu tương tác người dùng.

---

# 1. Môi trường Kiểm thử

## 1.1 Headless GTK Environment

Thiết lập:

```bash
GDK_BACKEND=headless
GSK_RENDERER=cairo
GTK_A11Y=none
NO_AT_BRIDGE=1
```

Mục tiêu:

- Không kết nối Wayland/X11 thực tế.
- Không mở cửa sổ GUI.
- Tương thích CI/CD.

---

## 1.2 Isolated D-Bus Session

Mỗi bộ test phải chạy trong một Session Bus riêng biệt.

Ví dụ:

```bash
dbus-run-session -- cargo test --test integration
```

Hoặc:

```bash
dbus-daemon --session
```

và thiết lập:

```bash
DBUS_SESSION_BUS_ADDRESS=...
```

Mục tiêu:

- Không xung đột với phiên người dùng.
- Cho phép chạy song song nhiều pipeline CI.

---

## 1.3 Temporary Runtime Directory

Mỗi test tạo:

```text
/tmp/anyrun-test-<uuid>
```

Bao gồm:

```text
config/
plugins/
runtime/
logs/
```

Sau khi test kết thúc:

- tự động cleanup
- không để lại socket
- không để lại process zombie

---

# 2. Nguyên tắc Assertion

Ưu tiên:

1. API Contract
2. Exit Code
3. IPC Response
4. Process State

Không phụ thuộc vào:

- wording log
- màu sắc UI
- nội dung GTK warning

Ví dụ tốt:

```rust
assert!(response.success);
```

Ví dụ không tốt:

```rust
assert!(stderr.contains("Window opened"));
```

---

# 3. Nhóm A — Standalone Mode

## IT-01. Standalone Startup With Default Configuration

### Setup

- Không daemon.
- Config mặc định.

### Execution

```bash
anyrun
```

### Assertions

- Exit Code = 0
- Không panic
- GTK Application khởi tạo thành công

---

## IT-02. Standalone Startup With Custom Config Directory

### Setup

Tạo:

```text
config.ron
style.css
```

### Execution

```bash
anyrun --config-dir <temp-config>
```

### Assertions

- Config được parse thành công
- Exit Code = 0

---

## IT-03. Standalone Startup With Explicit Plugins

### Setup

```bash
--plugins plugin_a.so plugin_b.so
```

### Assertions

- Plugin chỉ định được load
- Không load plugin từ config

---

## IT-04. Missing Configuration Fallback

### Setup

```bash
XDG_CONFIG_HOME=<empty>
```

### Assertions

- Default configuration được sử dụng
- Exit Code = 0

---

## IT-05. Invalid Configuration Recovery

### Setup

```ron
invalid {
```

### Assertions

- Parse thất bại
- Default configuration được dùng
- Exit Code = 0

---

## IT-06. Home Expansion Resolution

### Setup

```ron
plugins: ["~/plugins/test.so"]
```

### Assertions

- Path được resolve thành absolute path
- Plugin load thành công

---

## IT-07. Missing Plugin Recovery

### Setup

```ron
plugins: ["/fake/plugin.so"]
```

### Assertions

- Plugin bị bỏ qua
- Ứng dụng vẫn chạy

---

## IT-08. Match Selection Output Integrity

### Setup

Plugin mock:

```rust
handler() -> "hello-world"
```

### Assertions

stdout:

```text
hello-world
```

khớp chính xác.

---

## IT-09. Plugin Initialization Failure Isolation

### Setup

Plugin mock panic khi load.

### Assertions

- Ứng dụng không crash
- Plugin lỗi bị vô hiệu hóa

---

# 4. Nhóm B — Daemon Lifecycle

## IT-10. Daemon Bus Registration

### Execution

```bash
anyrun daemon
```

### Assertions

Bus name:

```text
org.anyrun.anyrun
```

được acquire thành công.

---

## IT-11. Duplicate Daemon Prevention

### Setup

Daemon #1 đang chạy.

### Execution

Khởi động daemon #2.

### Assertions

- Daemon #2 thất bại
- Exit Code ≠ 0

---

## IT-12. Custom CSS Loading

### Setup

CSS hợp lệ.

### Assertions

- CSS được áp dụng thành công
- Daemon hoạt động bình thường

---

## IT-13. Invalid CSS Recovery

### Setup

CSS lỗi hoặc không đọc được.

### Assertions

- Fallback CSS mặc định
- Daemon vẫn đăng ký D-Bus

---

## IT-14. Provider Spawn

### Assertions

- Provider được tạo
- IPC endpoint khả dụng

---

## IT-15. Custom Provider Spawn

### Setup

```ron
provider: "/tmp/mock-provider"
```

### Assertions

- Mock provider được thực thi

---

## IT-16. Stale Socket Cleanup

### Setup

Socket cũ tồn tại.

### Assertions

- Socket được thay thế
- IPC hoạt động bình thường

---

## IT-17. Provider Crash Recovery

### Setup

Provider bị SIGKILL.

### Assertions

Một trong hai:

- Provider được restart

hoặc

- Daemon báo lỗi có kiểm soát

Daemon không được crash.

---

# 5. Nhóm C — Client / Daemon IPC

## IT-18. Show Request Success

### Setup

Daemon đang chạy.

### Execution

```bash
anyrun
```

### Assertions

- RPC thành công
- Daemon trả phản hồi hợp lệ

---

## IT-19. STDIN Transfer Integrity

### Execution

```bash
echo "automation-test" | anyrun
```

### Assertions

Provider nhận:

```text
automation-test
```

chính xác.

---

## IT-20. Environment Transfer Integrity

### Setup

```bash
TEST_ENV=abc123
```

### Assertions

Provider nhận:

```text
abc123
```

---

## IT-21. Close Request

### Execution

```bash
anyrun close
```

### Assertions

- RPC thành công
- Window state = Hidden

---

## IT-22. Quit Request

### Execution

```bash
anyrun quit
```

### Assertions

- Daemon thoát sạch
- Provider bị dừng
- Bus name được giải phóng

---

## IT-23. Reload Request

### Execution

```bash
anyrun reload
```

### Assertions

- Config reload thành công
- Plugin reload thành công

---

## IT-24. Daemon Unavailable Fallback

### Setup

Không daemon.

### Execution

```bash
anyrun
```

### Assertions

- Standalone mode được kích hoạt
- Không timeout vô hạn

---

## IT-25. Concurrent Client Requests

### Setup

20 client đồng thời.

### Assertions

- Không deadlock
- Tất cả request hoàn thành

---

## IT-26. Reload During Active Query

### Setup

Provider đang xử lý query.

### Execution

```bash
anyrun reload
```

### Assertions

- Không crash
- Query hoàn thành

---

# 6. Nhóm D — Search Pipeline

## IT-27. Immediate Search On Startup

### Setup

```ron
show_results_immediately: true
```

### Assertions

Provider nhận:

```text
query=""
```

ngay khi mở.

---

## IT-28. Debounce Behavior

### Setup

```text
a
ab
abc
```

được nhập liên tục.

### Assertions

Chỉ query cuối được gửi.

---

## IT-29. Query Cancellation

### Setup

Provider xử lý chậm.

### Assertions

- Query cũ bị hủy
- Query mới được xử lý

---

## IT-30. Large Query Handling

### Setup

Chuỗi:

```text
10000 ký tự
```

### Assertions

- Không panic
- Không OOM

---

# 7. Nhóm E — Reliability & Fault Injection

## IT-31. D-Bus Restart Recovery

### Setup

Khởi động lại Session Bus.

### Assertions

- Daemon xử lý lỗi có kiểm soát
- Không crash bất thường

---

## IT-32. Provider Timeout Handling

### Setup

Provider treo.

### Assertions

- Timeout xảy ra đúng cấu hình
- UI không bị block

---

## IT-33. Graceful Shutdown During Active Query

### Setup

Query đang chạy.

### Execution

```bash
anyrun quit
```

### Assertions

- Không corruption
- Không zombie process

---

## IT-34. Repeated Open/Close Stability

### Setup

500 vòng:

```text
Show
Close
```

### Assertions

- Không memory leak đáng kể
- Không crash

---

# 8. Nhóm F — Performance Regression

Các benchmark chỉ chạy trên profile release.

```bash
cargo test --release
```

---

## IT-35. Client Startup Latency

### Quy trình

- Chạy 10 lần.
- Đo thời gian từ process start tới hoàn thành kết nối IPC.

### Assertions

```text
P95 < Baseline + 10%
```

---

## IT-36. IPC Round-trip Latency

### Quy trình

Đo:

```text
Client → Daemon → Client
```

### Assertions

```text
P95 < 20 ms
```

---

## IT-37. CSS Reload Throttle

### Quy trình

20 lần Show trong 1 giây.

### Assertions

- Chỉ 1 lần đọc CSS
- Không I/O thừa

---

## IT-38. Burst Request Stress Test

### Quy trình

100 yêu cầu Show liên tiếp.

### Assertions

- Không deadlock
- Không tăng bộ nhớ bất thường
- Không mất phản hồi

---

# Tiêu chí Hoàn thành (Exit Criteria)

Một bản build được xem là đạt chất lượng khi:

- 100% test contract pass.
- 100% test integration pass.
- Không có memory leak nghiêm trọng.
- Không có zombie process sau test.
- Không có deadlock.
- Không có panic.
- Không có regression hiệu năng vượt ngưỡng cho phép.
- Toàn bộ 38 kịch bản hoàn thành thành công trên Linux CI headless.
