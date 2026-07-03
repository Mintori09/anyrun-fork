mod helpers;

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use helpers::config::ConfigBuilder;
use helpers::mock::MockPluginType;
use helpers::temp::TestDir;
use helpers::anyrun_bin;

fn apply_standalone_env(cmd: &mut Command) -> &mut Command {
    cmd.env("GSK_RENDERER", "cairo")
        .env("GTK_A11Y", "none")
        .env("NO_AT_BRIDGE", "1")
        .env("GTK_USE_PORTAL", "0")
        .env("GIO_USE_PORTALS", "0")
}

fn spawn_standalone(config_dir: &std::path::Path) -> std::process::Child {
    let mut cmd = Command::new(anyrun_bin());
    cmd.arg("--config-dir").arg(config_dir);
    apply_standalone_env(&mut cmd);
    cmd.spawn().unwrap()
}

/// Verify the child stays alive at least 500ms, then kill it.
/// If it exited early, it must have been successful.
async fn assert_alive_or_success(child: &mut std::process::Child) {
    tokio::time::sleep(Duration::from_millis(500)).await;
    match child.try_wait().unwrap() {
        Some(status) => assert!(status.success(), "Process exited prematurely with failure"),
        None => {
            child.kill().ok();
            child.wait().ok();
        }
    }
}

/// IT-01: Standalone startup with default configuration.
#[tokio::test]
async fn it_01_standalone_default_config() {
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());

    let mut child = spawn_standalone(test_dir.config_dir());
    assert_alive_or_success(&mut child).await;
}

/// IT-02: Standalone with custom config dir.
#[tokio::test]
async fn it_02_standalone_custom_config_dir() {
    let test_dir = TestDir::new();
    let plugin_path = MockPluginType::Basic
        .so_path()
        .to_string_lossy()
        .to_string();
    ConfigBuilder::new()
        .plugins(&[plugin_path])
        .write(test_dir.config_dir());

    let mut child = spawn_standalone(test_dir.config_dir());
    assert_alive_or_success(&mut child).await;
}

/// IT-03: Standalone with explicit plugins via --plugins CLI.
#[tokio::test]
async fn it_03_standalone_explicit_plugins() {
    let test_dir = TestDir::new();
    ConfigBuilder::new().write(test_dir.config_dir());
    let plugin_path = MockPluginType::Basic.so_path();

    let mut cmd = Command::new(anyrun_bin());
    cmd.args(["--plugins", &plugin_path.to_string_lossy()])
        .arg("--config-dir")
        .arg(test_dir.config_dir());
    apply_standalone_env(&mut cmd);
    let mut child = cmd.spawn().unwrap();

    assert_alive_or_success(&mut child).await;
}

/// IT-04: Missing config directory falls back to defaults.
#[tokio::test]
async fn it_04_standalone_missing_config_fallback() {
    let empty_dir = TestDir::new();

    let mut cmd = Command::new(anyrun_bin());
    cmd.env("XDG_CONFIG_HOME", empty_dir.path());
    apply_standalone_env(&mut cmd);
    let mut child = cmd.spawn().unwrap();

    assert_alive_or_success(&mut child).await;
}

/// IT-05: Invalid RON in config.ron recovers with defaults.
#[tokio::test]
async fn it_05_standalone_invalid_config_recovery() {
    let test_dir = TestDir::new();
    std::fs::write(test_dir.config_dir().join("config.ron"), b"invalid {").unwrap();
    std::fs::write(test_dir.config_dir().join("style.css"), b"").unwrap();

    let mut child = spawn_standalone(test_dir.config_dir());
    assert_alive_or_success(&mut child).await;
}

/// IT-06: Home expansion (~/...) in plugin paths.
#[tokio::test]
async fn it_06_standalone_home_expansion() {
    let test_dir = TestDir::new();
    let home = std::env::var("HOME").unwrap();
    let fake_plugin_dir = PathBuf::from(&home).join(".local/share/anyrun/plugins");
    std::fs::create_dir_all(&fake_plugin_dir).unwrap();

    let src = MockPluginType::Basic.so_path();
    let dest = fake_plugin_dir.join("basic.so");
    std::fs::copy(&src, &dest).unwrap();

    ConfigBuilder::new()
        .plugins(&["~/.local/share/anyrun/plugins/basic.so".to_string()])
        .write(test_dir.config_dir());

    let mut child = spawn_standalone(test_dir.config_dir());
    assert_alive_or_success(&mut child).await;

    std::fs::remove_file(&dest).ok();
}

/// IT-07: Missing plugin path doesn't crash app.
#[tokio::test]
async fn it_07_standalone_missing_plugin_recovery() {
    let test_dir = TestDir::new();
    ConfigBuilder::new()
        .plugins(&["/fake/plugin.so".to_string()])
        .write(test_dir.config_dir());

    let mut child = spawn_standalone(test_dir.config_dir());
    assert_alive_or_success(&mut child).await;
}

/// IT-08: Match selection output integrity — requires GTK event simulation.
#[tokio::test]
#[ignore = "Requires GTK event simulation"]
async fn it_08_standalone_match_output_integrity() {
    // Placeholder: needs GTK event simulation or D-Bus based selection
}

/// IT-09: Plugin panic in init doesn't crash the app.
#[tokio::test]
async fn it_09_standalone_plugin_init_failure() {
    let test_dir = TestDir::new();
    MockPluginType::PanicInit.copy_to(test_dir.plugins_dir());
    let plugin_path = test_dir
        .plugins_dir()
        .join("libmock_plugin_panic_init.so")
        .to_string_lossy()
        .to_string();
    ConfigBuilder::new()
        .plugins(&[plugin_path])
        .write(test_dir.config_dir());

    let mut child = spawn_standalone(test_dir.config_dir());
    assert_alive_or_success(&mut child).await;
}
