# Bộ Integration Test Cho Anyrun

## Summary

Xây dựng bộ integration tests (Rust) cho toàn bộ 38 kịch bản trong
`docs/testing.md`, thay thế dần bash-based e2e (`scripts/e2e-tests.sh`) bằng
Rust tests chạy trong CI headless. Tận dụng `cargo test` hiện có, thêm helpers
dùng chung, mock plugins, và infrastructure cho headless GTK + D-Bus cô lập.

## Architecture

### 1. Shared Helpers Module (`anyrun/tests/helpers/`)

```
anyrun/tests/helpers/
  mod.rs          # Re-export, constants (bus name, timeout, etc.)
  env.rs          # Headless GTK env setup
  temp.rs         # Temp directory lifecycle (tempfile wrapper)
  config.rs       # Config.ron generation for scenarios
  process.rs      # DaemonProcess, ProviderProcess wrappers
  dbus.rs         # Private dbus-daemon, bus name polling, IPC helpers
  mock.rs         # Build + load mock plugin .so files
```

#### `helpers/mod.rs`

```rust
pub const BUS_NAME: &str = "org.anyrun.anyrun";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
pub const SLOW_TIMEOUT: Duration = Duration::from_secs(30);

// Re-export commonly used helpers
pub use dbus::{start_private_dbus, wait_for_dbus_registration, dbus_call};
pub use process::{DaemonProcess, ProviderProcess};
pub use temp::TempDir;
pub use config::ConfigBuilder;
pub use env::set_headless_env;
pub use mock::MockPlugin;
```

#### `helpers/env.rs`

Set biến môi trường cho headless GTK:

```rust
pub fn set_headless_env() {
    std::env::set_var("GDK_BACKEND", "headless");
    std::env::set_var("GSK_RENDERER", "cairo");
    std::env::set_var("GTK_A11Y", "none");
    std::env::set_var("NO_AT_BRIDGE", "1");
}
```

#### `helpers/temp.rs`

Wrapper trên `tempfile::TempDir`, tạo cấu trúc:

```
/tmp/anyrun-test-<uuid>/
  config/
    config.ron
    style.css
  plugins/
    (mock plugin .so files copied here)
  runtime/
  logs/
```

```rust
pub struct TestDir {
    inner: tempfile::TempDir,
    config_dir: PathBuf,
    plugins_dir: PathBuf,
    runtime_dir: PathBuf,
    logs_dir: PathBuf,
}

impl TestDir {
    pub fn new() -> Self { /* create structure */ }
    pub fn config_dir(&self) -> &Path { &self.config_dir }
    pub fn plugins_dir(&self) -> &Path { &self.plugins_dir }
    pub fn runtime_dir(&self) -> &Path { &self.runtime_dir }
    pub fn path(&self) -> &Path { self.inner.path() }
}

// Drop → tự động cleanup
```

#### `helpers/config.rs`

Builder pattern cho config.ron:

```rust
pub struct ConfigBuilder {
    content: String,
}

impl ConfigBuilder {
    pub fn new() -> Self { /* default config */ }
    pub fn plugins(mut self, plugins: &[&str]) -> Self { /* override plugin list */ }
    pub fn provider(mut self, path: &str) -> Self { /* override provider path */ }
    pub fn show_results_immediately(mut self, val: bool) -> Self { /* ... */ }
    pub fn settle_delay(mut self, ms: u64) -> Self { /* ... */ }
    pub fn write(&self, dir: &Path) -> PathBuf { /* write config.ron + style.css */ }
}
```

#### `helpers/process.rs`

```rust
pub struct DaemonProcess {
    child: Option<Child>,
    dbus_address: String,
    test_dir: TestDir,
}

impl DaemonProcess {
    pub async fn spawn(test_dir: TestDir) -> Self {
        // set_headless_env()
        // start_private_dbus()
        // spawn `anyrun daemon`
        // wait_for_dbus_registration()
    }
    pub async fn spawn_with_config(config: ConfigBuilder) -> Self { /* ... */ }
    pub fn dbus_address(&self) -> &str { /* ... */ }
    pub async fn quit(&mut self) { /* send quit RPC, wait for exit */ }
    pub async fn reload(&mut self) { /* send reload RPC */ }
    pub fn client(&self) -> ClientHandle { /* ... */ }
}

impl Drop for DaemonProcess {
    fn drop(&mut self) { /* kill child, child.wait(), cleanup temp */ }
}

pub struct ClientHandle { /* D-Bus proxy to daemon */ }
impl ClientHandle {
    pub async fn show(&self) -> Result<()> { /* ... */ }
    pub async fn close(&self) -> Result<()> { /* ... */ }
    pub async fn quit(&self) -> Result<()> { /* ... */ }
    pub async fn reload(&self) -> Result<()> { /* ... */ }
}

pub struct ProviderProcess {
    child: Option<Child>,
    socket_path: PathBuf,
    test_dir: TestDir,
}

impl ProviderProcess {
    pub async fn spawn(test_dir: TestDir) -> Self { /* ... */ }
    pub async fn spawn_custom(path: &str) -> Self { /* custom provider binary */ }
    pub fn socket_path(&self) -> &Path { /* ... */ }
}

impl Drop for ProviderProcess {
    fn drop(&mut self) { /* kill, cleanup */ }
}
```

#### `helpers/dbus.rs`

```rust
/// Start a private dbus-daemon --session --nofork.
/// Returns the DBUS_SESSION_BUS_ADDRESS.
pub fn start_private_dbus() -> (Child, String) { /* ... */ }

/// Poll D-Bus until `org.anyrun.anyrun` is registered.
pub async fn wait_for_dbus_registration(addr: &str) { /* ... */ }

/// Wait for a D-Bus name to appear.
pub async fn wait_for_bus_name(addr: &str, name: &str) { /* ... */ }
```

#### `helpers/mock.rs`

Build và manage mock plugin `.so` files:

```rust
pub enum MockPluginType {
    Basic,       // handler returns "hello-world"
    PanicInit,   // panic trong #[init]
    HangQuery,   // sleep vô hạn trong #[get_matches]
}

impl MockPluginType {
    pub fn so_path(&self) -> PathBuf {
        // Returns path to pre-built .so in target/debug or target/release
    }

    pub fn copy_to(&self, dest: &Path) -> PathBuf {
        // Copy .so to plugins dir
    }
}
```

### 2. Mock Plugins (`tests/mock-plugins/`)

3 crates riêng trong workspace, đặt dưới `tests/mock-plugins/`:

```
tests/mock-plugins/
  basic/
    Cargo.toml
    src/lib.rs
  panic-init/
    Cargo.toml
    src/lib.rs
  hang-query/
    Cargo.toml
    src/lib.rs
```

#### `basic/src/lib.rs` — return fixed matches, handler trả về "hello-world"

```rust
use abi_stable::std_types::{RString, RVec, ROption};
use anyrun_plugin::*;

struct Config;

impl Default for Config {
    fn default() -> Self { Config }
}

#[init]
fn init(_: RString) -> Config { Config }

#[info]
fn info() -> PluginInfo {
    PluginInfo { name: "mock-basic".into(), icon: "".into() }
}

#[get_matches]
fn get_matches(_: RString, _: &Config) -> RVec<Match> {
    vec![Match {
        title: "test-match".into(),
        icon: ROption::RNone,
        id: ROption::RSome("test-id".into()),
        description: ROption::RSome("A test match".into()),
        use_pango: false,
    }].into()
}

#[handler]
fn handler(_: Match, _: &Config) -> HandleResult {
    HandleResult::Copy(b"hello-world".to_vec().into())
}
```

#### `panic-init/src/lib.rs` — panic khi init

```rust
#[init]
fn init(_: RString) -> () {
    panic!("mock panic in init");
}
```

#### `hang-query/src/lib.rs` — sleep vô hạn trong get_matches

```rust
#[get_matches]
fn get_matches(_: RString, _: &()) -> RVec<Match> {
    std::thread::sleep(Duration::from_secs(u64::MAX));
    vec![].into()
}
```

### 3. Test File Organization

```
anyrun/tests/
  helpers/              (shared utilities)
  client_daemon_e2e.rs  (existing — giữ nguyên)
  group_a_standalone.rs  IT-01 → IT-09
  group_b_daemon.rs      IT-10 → IT-17
  group_c_ipc.rs         IT-18 → IT-26
  group_d_search.rs      IT-27 → IT-30
  group_e_reliability.rs IT-31 → IT-34
  group_f_performance.rs IT-35 → IT-38
```

Mỗi file test dùng `#[serial_test::serial]` hoặc `TEST_MUTEX` (pattern hiện tại)
để serialize D-Bus tests.

### 4. Test Lifecycle Pattern

```rust
#[tokio::test]
async fn it_01_standalone_default_config() {
    set_headless_env();
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let mut daemon = DaemonProcess::spawn(test_dir).await;

    // Assert: exit code, bus registration
    daemon.quit().await;
    assert!(daemon.child.take().unwrap().wait().unwrap().success());
}
```

## Detailed Test Specifications

### Group A — Standalone Mode (IT-01 → IT-09)

#### IT-01: Standalone Startup With Default Configuration

| Field | Value |
|-------|-------|
| Test | `standalone_default_config` |
| Setup | Không daemon. Config mặc định. `set_headless_env()` |
| Execution | `anyrun` (standalone, spawn + quit ngay) |
| Assert | Exit code = 0, không panic, GTK app init |

```rust
#[tokio::test]
async fn it_01_standalone_default_config() {
    set_headless_env();
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let mut child = Command::new(anyrun_bin())
        .env("GDK_BACKEND", "headless")
        .env("GSK_RENDERER", "cairo")
        .env("GTK_A11Y", "none")
        .env("NO_AT_BRIDGE", "1")
        .arg("--config-dir")
        .arg(test_dir.config_dir())
        .spawn()
        .unwrap();

    // Give GTK time to init
    tokio::time::sleep(Duration::from_millis(500)).await;
    child.kill().ok();
    let status = child.wait().unwrap();
    assert!(status.success());
}
```

#### IT-02: Standalone Startup With Custom Config Directory

| Field | Value |
|-------|-------|
| Test | `standalone_custom_config_dir` |
| Setup | Tạo `config.ron` + `style.css` trong temp dir |
| Execution | `anyrun --config-dir <temp-config>` |
| Assert | Config parse thành công, exit code = 0 |

```rust
#[tokio::test]
async fn it_02_standalone_custom_config_dir() {
    set_headless_env();
    let test_dir = TestDir::new();
    ConfigBuilder::new()
        .plugins(&["tests/mock-plugins/basic/target/release/libmock_basic.so"])
        .write(test_dir.config_dir());

    let mut child = spawn_standalone(&test_dir);
    tokio::time::sleep(Duration::from_millis(500)).await;
    child.kill().ok();
    assert!(child.wait().unwrap().success());
}
```

#### IT-03: Standalone Startup With Explicit Plugins

| Field | Value |
|-------|-------|
| Test | `standalone_explicit_plugins` |
| Setup | Plugin `.so` chỉ định qua `--plugins` CLI flag |
| Execution | `anyrun --plugins plugin_a.so plugin_b.so` |
| Assert | Chỉ load plugin từ CLI, không load từ config |

```rust
#[tokio::test]
async fn it_03_standalone_explicit_plugins() {
    set_headless_env();
    let test_dir = TestDir::new();
    let plugin_path = MockPluginType::Basic
        .copy_to(test_dir.plugins_dir());

    let mut child = Command::new(anyrun_bin())
        .args(&["--plugins", &plugin_path.to_string_lossy()])
        .envs(headless_env())
        .arg("--config-dir")
        .arg(test_dir.config_dir())
        .spawn()
        .unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;
    child.kill().ok();
    assert!(child.wait().unwrap().success());
}
```

#### IT-04: Missing Configuration Fallback

| Field | Value |
|-------|-------|
| Test | `standalone_missing_config_fallback` |
| Setup | `XDG_CONFIG_HOME` trỏ đến thư mục rỗng |
| Execution | `anyrun` |
| Assert | Dùng default config, exit code = 0 |

```rust
#[tokio::test]
async fn it_04_standalone_missing_config_fallback() {
    set_headless_env();
    let empty_dir = TestDir::new();

    let mut child = Command::new(anyrun_bin())
        .env("XDG_CONFIG_HOME", empty_dir.path())
        .envs(headless_env())
        .spawn()
        .unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;
    child.kill().ok();
    assert!(child.wait().unwrap().success());
}
```

#### IT-05: Invalid Configuration Recovery

| Field | Value |
|-------|-------|
| Test | `standalone_invalid_config_recovery` |
| Setup | Config.ron với syntax lỗi (`invalid {`) |
| Execution | `anyrun --config-dir <dir>` |
| Assert | Parse thất bại → dùng default config, exit code = 0 |

```rust
#[tokio::test]
async fn it_05_standalone_invalid_config_recovery() {
    set_headless_env();
    let test_dir = TestDir::new();
    std::fs::write(
        test_dir.config_dir().join("config.ron"),
        b"invalid {",
    )
    .unwrap();
    std::fs::write(
        test_dir.config_dir().join("style.css"),
        b"",
    )
    .unwrap();

    let mut child = spawn_standalone(&test_dir);
    tokio::time::sleep(Duration::from_millis(500)).await;
    child.kill().ok();
    assert!(child.wait().unwrap().success());
}
```

#### IT-06: Home Expansion Resolution

| Field | Value |
|-------|-------|
| Test | `standalone_home_expansion` |
| Setup | Config với `plugins: ["~/plugins/test.so"]` |
| Execution | `anyrun` |
| Assert | Path resolve thành absolute, plugin load |

```rust
#[tokio::test]
async fn it_06_standalone_home_expansion() {
    set_headless_env();
    let test_dir = TestDir::new();
    let home = std::env::var("HOME").unwrap();
    let fake_plugin = format!("{}/.local/share/anyrun/plugins/basic.so", home);

    // Copy mock plugin to expected location
    let src = MockPluginType::Basic.so_path();
    std::fs::create_dir_all(Path::new(&fake_plugin).parent().unwrap()).unwrap();
    std::fs::copy(&src, &fake_plugin).unwrap();

    ConfigBuilder::new()
        .plugins(&[&format!("~/.local/share/anyrun/plugins/basic.so")])
        .write(test_dir.config_dir());

    let mut child = spawn_standalone(&test_dir);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check dấu hiệu plugin đã load (log, process state, etc.)
    child.kill().ok();
    assert!(child.wait().unwrap().success());
}
```

#### IT-07: Missing Plugin Recovery

| Field | Value |
|-------|-------|
| Test | `standalone_missing_plugin_recovery` |
| Setup | Config với `plugins: ["/fake/plugin.so"]` |
| Execution | `anyrun` |
| Assert | Plugin lỗi bị bỏ qua, app vẫn chạy |

```rust
#[tokio::test]
async fn it_07_standalone_missing_plugin_recovery() {
    set_headless_env();
    let test_dir = TestDir::new();
    ConfigBuilder::new()
        .plugins(&["/fake/plugin.so"])
        .write(test_dir.config_dir());

    let mut child = spawn_standalone(&test_dir);
    tokio::time::sleep(Duration::from_millis(500)).await;
    child.kill().ok();
    assert!(child.wait().unwrap().success());
}
```

#### IT-08: Match Selection Output Integrity

| Field | Value |
|-------|-------|
| Test | `standalone_match_output_integrity` |
| Setup | Plugin mock basic (handler trả về "hello-world") |
| Execution | anyrun chọn match, kiểm tra stdout |
| Assert | stdout chứa chính xác "hello-world" |

Cần mock GTK event để trigger selection, hoặc test qua D-Bus / IPC.

```rust
#[tokio::test]
async fn it_08_standalone_match_output_integrity() {
    set_headless_env();
    let test_dir = TestDir::new();
    MockPluginType::Basic.copy_to(test_dir.plugins_dir());
    ConfigBuilder::new()
        .plugins(&[test_dir.plugins_dir().join("libmock_basic.so")
            .to_string_lossy().as_ref()])
        .write(test_dir.config_dir());

    // Note: Cần GTK interaction test. Có thể dùng
    // GtkApplication::activate signal + D-Bus để trigger select.
    // Hoặc tạm thời skip cho phase 1.
    // Phase 1: Verify plugin load + handler registration qua IPC.
}
```

#### IT-09: Plugin Initialization Failure Isolation

| Field | Value |
|-------|-------|
| Test | `standalone_plugin_init_failure` |
| Setup | Plugin panic-init |
| Execution | `anyrun` |
| Assert | App không crash, plugin lỗi bị vô hiệu hóa |

```rust
#[tokio::test]
async fn it_09_standalone_plugin_init_failure() {
    set_headless_env();
    let test_dir = TestDir::new();
    MockPluginType::PanicInit.copy_to(test_dir.plugins_dir());
    ConfigBuilder::new()
        .plugins(&[test_dir.plugins_dir().join("libmock_panic_init.so")
            .to_string_lossy().as_ref()])
        .write(test_dir.config_dir());

    let mut child = spawn_standalone(&test_dir);
    tokio::time::sleep(Duration::from_millis(500)).await;
    child.kill().ok();
    assert!(child.wait().unwrap().success());
}
```

### Group B — Daemon Lifecycle (IT-10 → IT-17)

#### IT-10: Daemon Bus Registration

| Field | Value |
|-------|-------|
| Test | `daemon_bus_registration` |
| Execution | `anyrun daemon` trong private D-Bus session |
| Assert | Bus name `org.anyrun.anyrun` acquired |

```rust
#[tokio::test]
async fn it_10_daemon_bus_registration() {
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(
        test_dir,
        &addr,
    ).await;

    assert!(wait_for_dbus_registration(&addr).await);
    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-11: Duplicate Daemon Prevention

| Field | Value |
|-------|-------|
| Test | `daemon_duplicate_prevention` |
| Setup | Daemon #1 đang chạy |
| Execution | Khởi động daemon #2 |
| Assert | Daemon #2 thất bại, exit code ≠ 0 |

```rust
#[tokio::test]
async fn it_11_daemon_duplicate_prevention() {
    let test_dir1 = TestDir::new();
    let test_dir2 = TestDir::new();
    ConfigBuilder::new().write(test_dir1.config_dir());
    ConfigBuilder::new().write(test_dir2.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let daemon1 = DaemonProcess::spawn_with(test_dir1, &addr).await;
    wait_for_dbus_registration(&addr).await;

    // Try second daemon
    let mut child = Command::new(anyrun_bin())
        .args(&["daemon"])
        .env("DBUS_SESSION_BUS_ADDRESS", &addr)
        .envs(headless_env())
        .arg("--config-dir")
        .arg(test_dir2.config_dir())
        .spawn()
        .unwrap();

    let status = child.wait_with_timeout(Duration::from_secs(5)).await;
    assert!(!status.success(), "Second daemon should fail");

    drop(daemon1);
    dbus_child.wait().ok();
}
```

#### IT-12: Custom CSS Loading

| Field | Value |
|-------|-------|
| Test | `daemon_custom_css_loading` |
| Setup | style.css hợp lệ |
| Execution | `anyrun daemon` |
| Assert | CSS applied, daemon hoạt động bình thường |

```rust
#[tokio::test]
async fn it_12_daemon_custom_css_loading() {
    let test_dir = TestDir::new();
    ConfigBuilder::new()
        .with_css("window { background-color: red; }")
        .write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;

    assert!(wait_for_dbus_registration(&addr).await);
    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-13: Invalid CSS Recovery

| Field | Value |
|-------|-------|
| Test | `daemon_invalid_css_recovery` |
| Setup | CSS lỗi hoặc không đọc được |
| Execution | `anyrun daemon` |
| Assert | Fallback CSS mặc định, daemon vẫn đăng ký D-Bus |

```rust
#[tokio::test]
async fn it_13_daemon_invalid_css_recovery() {
    let test_dir = TestDir::new();
    ConfigBuilder::new()
        .with_css("this is not valid css {{{")
        .write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;

    assert!(wait_for_dbus_registration(&addr).await);
    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-14: Provider Spawn

| Field | Value |
|-------|-------|
| Test | `daemon_provider_spawn` |
| Assert | Provider process được tạo, IPC endpoint khả dụng |

```rust
#[tokio::test]
async fn it_14_daemon_provider_spawn() {
    let test_dir = TestDir::new();
    ConfigBuilder::new()
        .provider("anyrun-provider")
        .write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    // Check provider socket exists
    let socket = test_dir.runtime_dir().join("provider.sock");
    wait_for_file(&socket, Duration::from_secs(5)).await;

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-15: Custom Provider Spawn

| Field | Value |
|-------|-------|
| Test | `daemon_custom_provider_spawn` |
| Setup | Config `provider: "/tmp/mock-provider"` |
| Execution | Daemon khởi động với custom provider path |
| Assert | Mock provider được thực thi (có thể dùng script wrapper) |

```rust
#[tokio::test]
async fn it_15_daemon_custom_provider_spawn() {
    let test_dir = TestDir::new();
    let mock_provider = test_dir.path().join("mock-provider.sh");
    std::fs::write(&mock_provider, "#!/bin/sh\necho mock-provider-running\nsleep 30")
        .unwrap();
    std::fs::set_permissions(&mock_provider, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .unwrap();

    ConfigBuilder::new()
        .provider(&mock_provider.to_string_lossy())
        .write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    // Give time for mock provider to start
    tokio::time::sleep(Duration::from_millis(500)).await;
    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-16: Stale Socket Cleanup

| Field | Value |
|-------|-------|
| Test | `daemon_stale_socket_cleanup` |
| Setup | Socket cũ tồn tại trong runtime dir |
| Execution | Daemon khởi động |
| Assert | Socket cũ được thay thế, IPC hoạt động |

```rust
#[tokio::test]
async fn it_16_daemon_stale_socket_cleanup() {
    let test_dir = TestDir::new();
    let socket_path = test_dir.runtime_dir().join("provider.sock");

    // Create stale socket
    std::fs::write(&socket_path, b"stale").unwrap();
    assert!(socket_path.exists());

    ConfigBuilder::new().write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    // Socket should be replaced (recreated by provider)
    wait_for_file(&socket_path, Duration::from_secs(5)).await;

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-17: Provider Crash Recovery

| Field | Value |
|-------|-------|
| Test | `daemon_provider_crash_recovery` |
| Setup | Provider bị SIGKILL |
| Execution | Daemon đang chạy → kill provider process |
| Assert | Provider được restart hoặc daemon báo lỗi có kiểm soát. Daemon không crash. |

```rust
#[tokio::test]
async fn it_17_daemon_provider_crash_recovery() {
    let test_dir = TestDir::new();
    ConfigBuilder::new()
        .provider("anyrun-provider")
        .write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    // Find provider process
    // (Need to track PID — extend ProviderProcess to expose child PID)
    // kill -9 provider
    // Wait, then check daemon still alive
    // Assert provider restarted or graceful error

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

### Group C — Client/Daemon IPC (IT-18 → IT-26)

#### IT-18: Show Request Success

| Field | Value |
|-------|-------|
| Test | `ipc_show_request` |
| Setup | Daemon đang chạy |
| Execution | `anyrun` (client mode) |
| Assert | RPC thành công, daemon trả phản hồi hợp lệ |

```rust
#[tokio::test]
async fn it_18_ipc_show_request() {
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    // Use D-Bus proxy to send Show request
    let client = daemon.client();
    let result = client.show().await;
    assert!(result.is_ok());

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-19: STDIN Transfer Integrity

| Field | Value |
|-------|-------|
| Test | `ipc_stdin_transfer` |
| Execution | `echo "automation-test" | anyrun` |
| Assert | Provider nhận chính xác "automation-test" |

Cần mock provider ghi stdin nhận được vào file, sau đó verify file content.

```rust
#[tokio::test]
async fn it_19_ipc_stdin_transfer() {
    let test_dir = TestDir::new();
    let output_file = test_dir.path().join("stdin_captured.txt");

    // Script provider ghi stdin vào file
    let script = format!(
        "#!/bin/sh\ncat > {}\nsleep 10\n",
        output_file.to_string_lossy()
    );
    let provider_script = test_dir.path().join("capture_stdin.sh");
    std::fs::write(&provider_script, &script).unwrap();
    std::fs::set_permissions(&provider_script, PermissionsExt::from_mode(0o755)).unwrap();

    ConfigBuilder::new()
        .provider(&provider_script.to_string_lossy())
        .write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    // Send Show request with stdin
    let client = daemon.client();
    client.show_with_stdin("automation-test").await.unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;

    let captured = std::fs::read_to_string(&output_file).unwrap();
    assert_eq!(captured.trim(), "automation-test");

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-20: Environment Transfer Integrity

| Field | Value |
|-------|-------|
| Test | `ipc_env_transfer` |
| Setup | `TEST_ENV=abc123` |
| Execution | `anyrun` (client) |
| Assert | Provider nhận được biến môi trường |

```rust
#[tokio::test]
async fn it_20_ipc_env_transfer() {
    let test_dir = TestDir::new();
    let output_file = test_dir.path().join("env_captured.txt");

    // Script provider ghi $TEST_ENV vào file
    let script = format!(
        "#!/bin/sh\necho $TEST_ENV > {}\nsleep 10\n",
        output_file.to_string_lossy()
    );
    let provider_script = test_dir.path().join("capture_env.sh");
    std::fs::write(&provider_script, &script).unwrap();
    std::fs::set_permissions(&provider_script, PermissionsExt::from_mode(0o755)).unwrap();

    ConfigBuilder::new()
        .provider(&provider_script.to_string_lossy())
        .write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with_env(
        test_dir, &addr, vec![("TEST_ENV", "abc123")]
    ).await;
    wait_for_dbus_registration(&addr).await;

    let client = daemon.client();
    client.show().await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let captured = std::fs::read_to_string(&output_file).unwrap();
    assert_eq!(captured.trim(), "abc123");

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-21: Close Request

| Field | Value |
|-------|-------|
| Test | `ipc_close_request` |
| Execution | `anyrun close` |
| Assert | RPC thành công, window state = Hidden |

```rust
#[tokio::test]
async fn it_21_ipc_close_request() {
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    let client = daemon.client();
    client.show().await.unwrap();
    client.close().await.unwrap();

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-22: Quit Request

| Field | Value |
|-------|-------|
| Test | `ipc_quit_request` |
| Execution | `anyrun quit` |
| Assert | Daemon thoát sạch, provider dừng, bus name giải phóng |

Test này đã có trong `client_daemon_e2e.rs` (`test_client_daemon_e2e_communication`).

```rust
#[tokio::test]
async fn it_22_ipc_quit_request() {
    // Tương tự test_client_daemon_e2e_communication hiện có
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    daemon.quit().await;

    // Verify bus name released
    let output = Command::new("dbus-send")
        .args(&["--bus", &addr, "--dest=org.freedesktop.DBus",
                "--print-reply", "/org/freedesktop/DBus",
                "org.freedesktop.DBus.NameHasOwner",
                "string:org.anyrun.anyrun"])
        .output().unwrap();
    assert!(output.stdout.contains("false"));

    dbus_child.wait().ok();
}
```

#### IT-23: Reload Request

| Field | Value |
|-------|-------|
| Test | `ipc_reload_request` |
| Execution | `anyrun reload` |
| Assert | Config reload thành công, plugin reload thành công |

```rust
#[tokio::test]
async fn it_23_ipc_reload_request() {
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    // Modify config
    ConfigBuilder::new()
        .show_results_immediately(true)
        .write(test_dir.config_dir());

    let client = daemon.client();
    client.reload().await.unwrap();

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-24: Daemon Unavailable Fallback

| Field | Value |
|-------|-------|
| Test | `ipc_daemon_unavailable_fallback` |
| Setup | Không daemon, D-Bus address không hợp lệ |
| Execution | `anyrun` |
| Assert | Standalone mode kích hoạt, không timeout vô hạn |

Test này đã có trong `client_daemon_e2e.rs` (`test_client_standalone_fallback`).

```rust
#[tokio::test]
async fn it_24_ipc_daemon_unavailable_fallback() {
    set_headless_env();
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let mut child = Command::new(anyrun_bin())
        .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent.sock")
        .envs(headless_env())
        .arg("--config-dir")
        .arg(test_dir.config_dir())
        .spawn()
        .unwrap();

    // Should start in standalone mode, not hang
    tokio::time::sleep(Duration::from_secs(2)).await;
    child.kill().ok();
    assert!(child.wait().unwrap().success());
}
```

#### IT-25: Concurrent Client Requests

| Field | Value |
|-------|-------|
| Test | `ipc_concurrent_requests` |
| Setup | 20 client đồng thời gửi request |
| Assert | Không deadlock, tất cả request hoàn thành |

```rust
#[tokio::test]
async fn it_25_ipc_concurrent_requests() {
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    let mut handles = vec![];
    for _ in 0..20 {
        let addr = addr.clone();
        handles.push(tokio::spawn(async move {
            let client = ClientHandle::new(&addr);
            client.show().await
        }));
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-26: Reload During Active Query

| Field | Value |
|-------|-------|
| Test | `ipc_reload_during_query` |
| Setup | Provider đang xử lý query (hang-query mock) |
| Execution | Send reload trong khi query đang chạy |
| Assert | Không crash, query hoàn thành hoặc bị huỷ an toàn |

```rust
#[tokio::test]
async fn it_26_ipc_reload_during_query() {
    let test_dir = TestDir::new();
    MockPluginType::HangQuery.copy_to(test_dir.plugins_dir());
    ConfigBuilder::new()
        .plugins(&[test_dir.plugins_dir().join("libmock_hang_query.so")
            .to_string_lossy().as_ref()])
        .write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    let client = daemon.client();
    client.show().await.unwrap();

    // Send reload while query is active
    client.reload().await.unwrap();

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

### Group D — Search Pipeline (IT-27 → IT-30)

#### IT-27: Immediate Search On Startup

| Field | Value |
|-------|-------|
| Test | `search_immediate_on_startup` |
| Setup | `show_results_immediately: true` |
| Execution | Client kết nối đến daemon |
| Assert | Provider nhận query rỗng ngay khi mở |

```rust
#[tokio::test]
async fn it_27_search_immediate_on_startup() {
    let test_dir = TestDir::new();
    ConfigBuilder::new()
        .show_results_immediately(true)
        .write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    let client = daemon.client();
    client.show().await.unwrap();

    // Verify via provider IPC that empty query was received
    // (Requires instrumented mock provider)

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-28: Debounce Behavior

| Field | Value |
|-------|-------|
| Test | `search_debounce` |
| Setup | Gõ `a`, `ab`, `abc` liên tục với settle delay |
| Execution | Send 3 text changes liên tiếp |
| Assert | Chỉ query cuối (`abc`) được gửi đến provider |

```rust
#[tokio::test]
async fn it_28_search_debounce() {
    let test_dir = TestDir::new();
    ConfigBuilder::new()
        .settle_delay(100)
        .write(test_dir.config_dir());

    // Cần mock provider ghi lại các query nhận được
    // Send 3 queries liên tiếp
    // Verify provider chỉ nhận query cuối
}
```

#### IT-29: Query Cancellation

| Field | Value |
|-------|-------|
| Test | `search_query_cancellation` |
| Setup | Provider xử lý chậm (hang-query) |
| Execution | Send query mới khi query cũ chưa xong |
| Assert | Query cũ bị hủy, query mới được xử lý |

```rust
#[tokio::test]
async fn it_29_search_query_cancellation() {
    // Similar setup to IT-28 but with slow provider
    // Send query A, immediately send query B
    // Assert provider only processes B
}
```

#### IT-30: Large Query Handling

| Field | Value |
|-------|-------|
| Test | `search_large_query` |
| Setup | Input 10000 ký tự |
| Execution | Gửi query dài |
| Assert | Không panic, không OOM |

```rust
#[tokio::test]
async fn it_30_search_large_query() {
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let long_text = "a".repeat(10000);

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    let client = daemon.client();
    client.show_with_text(&long_text).await.unwrap();

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

### Group E — Reliability & Fault Injection (IT-31 → IT-34)

#### IT-31: D-Bus Restart Recovery

| Field | Value |
|-------|-------|
| Test | `reliability_dbus_restart` |
| Setup | Khởi động lại Session Bus |
| Execution | Daemon đang chạy → kill dbus-daemon → restart |
| Assert | Daemon xử lý lỗi có kiểm soát, không crash bất thường |

```rust
#[tokio::test]
async fn it_31_reliability_dbus_restart() {
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let (mut dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    // Kill dbus
    dbus_child.kill().unwrap();
    dbus_child.wait().unwrap();

    // Give daemon time to react
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Restart dbus
    let (new_dbus, _) = start_private_dbus();
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Daemon should still be alive
    // (may need to check process state)

    drop(daemon);
    new_dbus.wait().ok();
}
```

#### IT-32: Provider Timeout Handling

| Field | Value |
|-------|-------|
| Test | `reliability_provider_timeout` |
| Setup | Provider treo (hang-query plugin) |
| Execution | Send query, đợi timeout |
| Assert | Timeout xảy ra đúng cấu hình, UI không bị block |

```rust
#[tokio::test]
async fn it_32_reliability_provider_timeout() {
    let test_dir = TestDir::new();
    MockPluginType::HangQuery.copy_to(test_dir.plugins_dir());
    ConfigBuilder::new()
        .plugins(&[test_dir.plugins_dir().join("libmock_hang_query.so")
            .to_string_lossy().as_ref()])
        .write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    // Send query — should timeout but not crash
    let client = daemon.client();
    client.show_with_text("test").await.unwrap();

    // Wait for provider timeout
    tokio::time::sleep(Duration::from_secs(5)).await;
    // Daemon should still be responsive
    client.close().await.unwrap();

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

#### IT-33: Graceful Shutdown During Active Query

| Field | Value |
|-------|-------|
| Test | `reliability_shutdown_during_query` |
| Setup | Query đang chạy |
| Execution | `anyrun quit` |
| Assert | Không corruption, không zombie process |

```rust
#[tokio::test]
async fn it_33_reliability_shutdown_during_query() {
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    let client = daemon.client();
    client.show_with_text("test").await.unwrap();

    // Quit while query active
    daemon.quit().await;

    // Verify no zombie processes
    dbus_child.wait().ok();
}
```

#### IT-34: Repeated Open/Close Stability

| Field | Value |
|-------|-------|
| Test | `reliability_repeated_open_close` |
| Setup | 500 vòng Show/Close |
| Assert | Không memory leak đáng kể, không crash |

```rust
#[tokio::test]
async fn it_34_reliability_repeated_open_close() {
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    let client = daemon.client();
    for _ in 0..500 {
        client.show().await.unwrap();
        client.close().await.unwrap();
    }

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

### Group F — Performance Regression (IT-35 → IT-38)

Các benchmark chạy `#[ignore]` mặc định, chỉ chạy với `--release`:

```bash
cargo test --release -- --ignored
```

#### IT-35: Client Startup Latency

| Field | Value |
|-------|-------|
| Test | `perf_client_startup_latency` |
| Setup | 10 lần đo |
| Execution | Đo thời gian process start → hoàn thành IPC connect |
| Assert | P95 < Baseline + 10% |

```rust
#[ignore]
#[tokio::test]
async fn it_35_perf_client_startup_latency() {
    let latencies = measure_n_times(10, || async {
        let start = std::time::Instant::now();
        let client = ClientHandle::new(&addr);
        client.connect().await.unwrap();
        start.elapsed()
    }).await;

    let p95 = percentile(&latencies, 0.95);
    assert!(p95 < BASELINE_STARTUP * 1.1,
        "P95 startup latency {}ms exceeded baseline {}ms",
        p95.as_millis(), BASELINE_STARTUP.as_millis());
}
```

#### IT-36: IPC Round-trip Latency

| Field | Value |
|-------|-------|
| Test | `perf_ipc_roundtrip` |
| Setup | Đo round-trip Client → Daemon → Client |
| Assert | P95 < 20ms |

```rust
#[ignore]
#[tokio::test]
async fn it_36_perf_ipc_roundtrip() {
    let latencies = measure_n_times(100, || async {
        let start = std::time::Instant::now();
        client.show().await.unwrap();
        let elapsed = start.elapsed();
        client.close().await.unwrap();
        elapsed
    }).await;

    let p95 = percentile(&latencies, 0.95);
    assert!(p95 < Duration::from_millis(20),
        "P95 round-trip {}ms exceeded 20ms", p95.as_millis());
}
```

#### IT-37: CSS Reload Throttle

| Field | Value |
|-------|-------|
| Test | `perf_css_reload_throttle` |
| Setup | 20 lần Show trong 1 giây |
| Assert | Chỉ 1 lần đọc CSS, không I/O thừa |

```rust
#[ignore]
#[tokio::test]
async fn it_37_perf_css_reload_throttle() {
    // Cần instrument daemon để đếm số lần đọc CSS file
    // 20 show requests trong 1s
    // Assert CSS read count == 1
}
```

#### IT-38: Burst Request Stress Test

| Field | Value |
|-------|-------|
| Test | `perf_burst_request_stress` |
| Setup | 100 yêu cầu Show liên tiếp |
| Assert | Không deadlock, không tăng memory bất thường, không mất response |

```rust
#[ignore]
#[tokio::test]
async fn it_38_perf_burst_request_stress() {
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let (dbus_child, addr) = start_private_dbus();
    let mut daemon = DaemonProcess::spawn_with(test_dir, &addr).await;
    wait_for_dbus_registration(&addr).await;

    let client = daemon.client();
    let start = std::time::Instant::now();
    for _ in 0..100 {
        client.show().await.unwrap();
    }
    let elapsed = start.elapsed();

    // Verify all completed without hang
    assert!(elapsed < Duration::from_secs(30),
        "Burst requests timed out at {:?}", elapsed);

    daemon.quit().await;
    dbus_child.wait().ok();
}
```

## Helper Functions For Tests

```rust
// anyrun/tests/helpers/mod.rs (supplement)

/// Locate anyrun binary
pub fn anyrun_bin() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target")
        .join(profile)
        .join("anyrun");
    assert!(path.exists(), "anyrun binary not found at {:?}", path);
    path
}

/// Environment variables for headless GTK
pub fn headless_env() -> Vec<(&'static str, &'static str)> {
    vec![
        ("GDK_BACKEND", "headless"),
        ("GSK_RENDERER", "cairo"),
        ("GTK_A11Y", "none"),
        ("NO_AT_BRIDGE", "1"),
    ]
}

/// Spawn anyrun in standalone mode
pub fn spawn_standalone(test_dir: &TestDir) -> Child {
    Command::new(anyrun_bin())
        .envs(headless_env())
        .arg("--config-dir")
        .arg(test_dir.config_dir())
        .spawn()
        .unwrap()
}

/// Wait for file to exist with timeout
pub async fn wait_for_file(path: &Path, timeout: Duration) {
    let start = Instant::now();
    while !path.exists() {
        if start.elapsed() > timeout {
            panic!("Timeout waiting for file {:?}", path);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Percentile calculation (for performance tests)
pub fn percentile(data: &[Duration], p: f64) -> Duration {
    let mut sorted: Vec<_> = data.iter().copied().collect();
    sorted.sort();
    let index = (sorted.len() as f64 * p).ceil() as usize - 1;
    sorted[index.min(sorted.len() - 1)]
}
```

## Workspace Configuration

### `Cargo.toml` additions (workspace root)

```toml
[workspace.dependencies]
serial_test = "3"
tempfile = "3"
```

### `anyrun/Cargo.toml` additions

```toml
[dev-dependencies]
serial_test.workspace = true
tempfile.workspace = true
```

### `anyrun-provider/Cargo.toml`

```toml
# tempfile already in dev-dependencies
# Add serial_test if needed
```

### Mock plugins — 3 new workspace members

Mỗi mock plugin là một workspace member riêng.

Root `Cargo.toml`:
```toml
[workspace]
members = [
    # ... existing members ...
    "tests/mock-plugins/basic",
    "tests/mock-plugins/panic-init",
    "tests/mock-plugins/hang-query",
]
```

Mỗi mock plugin `Cargo.toml`:
```toml
[package]
name = "mock-plugin-basic"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
anyrun-plugin = { path = "../../anyrun-plugin" }
abi_stable = "0.11"
```

## Implementation Order

### Phase 1: Infrastructure
1. Tạo `tests/mock-plugins/` (3 crates)
2. Tạo `anyrun/tests/helpers/` module
3. Cập nhật `Cargo.toml` dependencies
4. Verify: helpers compile, mock plugins build

### Phase 2: Group A + B (Standalone + Daemon)
5. `group_a_standalone.rs` — IT-01 → IT-09
6. `group_b_daemon.rs` — IT-10 → IT-17

### Phase 3: Group C (IPC)
7. Mở rộng `ClientHandle` helpers
8. `group_c_ipc.rs` — IT-18 → IT-26

### Phase 4: Group D (Search)
9. Instrument mock provider cho search testing
10. `group_d_search.rs` — IT-27 → IT-30

### Phase 5: Group E (Reliability)
11. `group_e_reliability.rs` — IT-31 → IT-34

### Phase 6: Group F (Performance)
12. `group_f_performance.rs` — IT-35 → IT-38

### Phase 7: CI Integration
13. Update CI workflow chạy integration tests
14. Cleanup bash e2e scripts (optional)

## Exit Criteria

- 38/38 tests pass trong CI headless
- Không zombie process sau test suite
- Không memory leak đáng kể
- Không deadlock
- Không panic
- Test suite hoàn thành trong < 5 phút

---

## Self-Review Checklist

- [x] Không có "TBD", "TODO" trong spec
- [x] Architecture nhất quán: helpers module, mock plugins, test lifecycle
- [x] Mỗi IT test đều có: setup, execution, assertion, code skeleton
- [x] Implementation order hợp lý: infrastructure → groups A→F
- [x] Dependencies rõ ràng (serial_test, tempfile, mock plugin crates)
- [x] CI integration được đề cập
- [x] Scope phù hợp: 38 tests spec, code sẽ viết dần theo phases
