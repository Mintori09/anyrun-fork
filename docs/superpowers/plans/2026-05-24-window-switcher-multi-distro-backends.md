# Window Switcher Multi-Distro Backends Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add robust multi-distro backend support to `plugins/window-switcher`, starting with i3 and enabling easy expansion to additional Linux window managers/compositors.

**Architecture:** Introduce a deterministic backend probe chain with configurable override, backend capability checks, and testable command-runner abstraction. Keep existing `WindowBackend` contract unchanged so UI/matching logic in `lib.rs` stays stable while backend modules become independently testable. Implement i3 via `i3-msg -t get_tree` parsing and `i3-msg [con_id=ID] focus`, then extend detection for Sway using the same parser pattern.

**Tech Stack:** Rust (edition 2024), serde/serde_json, existing `anyrun_helper::window::{WindowBackend, WindowInfo}`, external CLI backends (`i3-msg`, `swaymsg`, `hyprctl`, `kdotool`, `niri-ipc`).

---

## Scope Check

This spec touches two subsystems:
1. Backend infrastructure (detection/probing/config override/testing seams)
2. Concrete WM backend implementations (i3 first, then additional WM hooks)

They are coupled in this plugin and can ship together as one plan while still producing incremental, testable commits.

## File Structure

- Modify: `plugins/window-switcher/src/config.rs`
  - Add optional backend override and backend probe order config.
- Modify: `plugins/window-switcher/src/lib.rs`
  - Pass config-aware detection; improve startup errors.
- Modify: `plugins/window-switcher/src/backends/mod.rs`
  - Add backend enum/name mapping, probe orchestration, command availability checks, and tests.
- Create: `plugins/window-switcher/src/backends/command.rs`
  - Shared command execution abstraction for runtime + test fakes.
- Create: `plugins/window-switcher/src/backends/i3.rs`
  - i3 implementation: tree parsing, focus action, workspace label extraction.
- Create: `plugins/window-switcher/src/backends/sway.rs`
  - Sway implementation mirroring i3 parser path.
- Modify: `plugins/window-switcher/src/backends/kwin.rs`
  - Switch to shared command abstraction; add capability probe helper.
- Modify: `plugins/window-switcher/src/backends/hyprland.rs`
  - Switch to shared command abstraction; add capability probe helper.
- Modify: `plugins/window-switcher/src/backends/niri.rs`
  - Add explicit capability probe function for detection chain.
- Modify: `plugins/window-switcher/README.md`
  - Document supported WMs, required binaries per distro, override config.
- Modify: `plugins/window-switcher/Cargo.toml`
  - Add any parsing/testing dependency needed (if required by implementation).

---

### Task 1: Add Config Surface for Backend Selection

**Files:**
- Modify: `plugins/window-switcher/src/config.rs`
- Test: `plugins/window-switcher/src/config.rs` (inline unit tests)

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_backend_config_is_auto_and_has_probe_order() {
        let cfg = Config::default();
        assert_eq!(cfg.backend.as_deref(), None);
        assert!(!cfg.backend_probe_order.is_empty());
        assert!(cfg.backend_probe_order.iter().any(|b| b == "i3"));
    }

    #[test]
    fn parses_backend_override_from_ron() {
        let ron = r#"Config(backend: Some(\"i3\"), backend_probe_order: [\"i3\", \"sway\"])"#;
        let cfg: Config = ron::from_str(ron).expect("config parse");
        assert_eq!(cfg.backend.as_deref(), Some("i3"));
        assert_eq!(cfg.backend_probe_order, vec!["i3", "sway"]);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p window-switcher default_backend_config_is_auto_and_has_probe_order -- --nocapture`
Expected: FAIL with unknown fields `backend` and `backend_probe_order`.

- [ ] **Step 3: Write minimal implementation**

```rust
#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    pub prefix: String,
    pub max_entries: usize,
    pub show_results_immediately: bool,
    pub cache_ttl_secs: u64,
    pub exclude_classes: Vec<String>,
    pub backend: Option<String>,
    pub backend_probe_order: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "w ".into(),
            max_entries: 15,
            show_results_immediately: false,
            cache_ttl_secs: 5,
            exclude_classes: vec!["plasmashell".into()],
            backend: None,
            backend_probe_order: vec![
                "kwin".into(),
                "niri".into(),
                "hyprland".into(),
                "sway".into(),
                "i3".into(),
            ],
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p window-switcher config::tests -- --nocapture`
Expected: PASS for the two new tests.

- [ ] **Step 5: Commit**

```bash
git add plugins/window-switcher/src/config.rs
git commit -m "feat(window-switcher): add backend override and probe order config"
```

### Task 2: Introduce Testable Command Runner for Backends

**Files:**
- Create: `plugins/window-switcher/src/backends/command.rs`
- Modify: `plugins/window-switcher/src/backends/mod.rs`
- Test: `plugins/window-switcher/src/backends/command.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_runner_reports_missing_binary() {
        let runner = FakeRunner::new();
        assert!(!runner.command_exists("i3-msg"));
    }

    #[test]
    fn fake_runner_returns_registered_stdout() {
        let mut runner = FakeRunner::new();
        runner.register_success("i3-msg", &["-t", "get_tree"], "{\"nodes\":[]}");
        let out = runner.run("i3-msg", &["-t", "get_tree"]).expect("stdout");
        assert_eq!(out, "{\"nodes\":[]}");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p window-switcher backends::command::tests -- --nocapture`
Expected: FAIL because `command` module and fake runner do not exist.

- [ ] **Step 3: Write minimal implementation**

```rust
pub trait CommandRunner: Send + Sync {
    fn command_exists(&self, bin: &str) -> bool;
    fn run(&self, bin: &str, args: &[&str]) -> Result<String, String>;
}

pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn command_exists(&self, bin: &str) -> bool {
        std::process::Command::new(bin).arg("--help").output().is_ok()
    }

    fn run(&self, bin: &str, args: &[&str]) -> Result<String, String> {
        let output = std::process::Command::new(bin)
            .args(args)
            .output()
            .map_err(|e| format!("run {bin} failed: {e}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        String::from_utf8(output.stdout).map_err(|e| format!("utf8 decode failed: {e}"))
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p window-switcher backends::command::tests -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add plugins/window-switcher/src/backends/command.rs plugins/window-switcher/src/backends/mod.rs
git commit -m "refactor(window-switcher): add shared command runner abstraction"
```

### Task 3: Refactor Backend Detection to Configurable Probe Chain

**Files:**
- Modify: `plugins/window-switcher/src/backends/mod.rs`
- Modify: `plugins/window-switcher/src/lib.rs`
- Test: `plugins/window-switcher/src/backends/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn detects_i3_when_i3_msg_present_and_env_matches() {
    let mut runner = FakeRunner::new();
    runner.add_binary("i3-msg");

    let env = ProbeEnv {
        wayland_display: None,
        xdg_current_desktop: Some("i3".into()),
        hypr_sig: None,
        niri_socket: None,
    };

    let backend = detect_backend_with(&runner, &env, None, &["i3".into()]);
    assert_eq!(backend.map(|b| b.name()), Some("i3"));
}

#[test]
fn honors_backend_override_before_auto_probe() {
    let mut runner = FakeRunner::new();
    runner.add_binary("swaymsg");

    let env = ProbeEnv::default();
    let backend = detect_backend_with(&runner, &env, Some("sway"), &["i3".into()]);
    assert_eq!(backend.map(|b| b.name()), Some("sway"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p window-switcher backends::tests::detects_i3_when_i3_msg_present_and_env_matches -- --nocapture`
Expected: FAIL because `detect_backend_with` and `ProbeEnv` do not exist.

- [ ] **Step 3: Write minimal implementation**

```rust
#[derive(Default, Clone)]
pub struct ProbeEnv {
    pub wayland_display: Option<String>,
    pub xdg_current_desktop: Option<String>,
    pub hypr_sig: Option<String>,
    pub niri_socket: Option<String>,
}

pub fn detect_backend(
    cfg_backend: Option<&str>,
    probe_order: &[String],
) -> Option<Box<dyn WindowBackend>> {
    let env = ProbeEnv {
        wayland_display: std::env::var("WAYLAND_DISPLAY").ok(),
        xdg_current_desktop: std::env::var("XDG_CURRENT_DESKTOP").ok(),
        hypr_sig: std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok(),
        niri_socket: std::env::var("NIRI_SOCKET").ok(),
    };
    detect_backend_with(&command::SystemRunner, &env, cfg_backend, probe_order)
}
```

Add `detect_backend_with(...)` to centralize override + probe order logic and return first backend whose `is_available(...)` returns true.

Update `lib.rs` `init`:

```rust
let backend = match backends::detect_backend(
    config.backend.as_deref(),
    &config.backend_probe_order,
) {
    Some(be) => be,
    None => {
        eprintln!("[window-switcher] No supported compositor/window manager detected");
        return None;
    }
};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p window-switcher backends::tests -- --nocapture`
Expected: PASS for detection tests.

- [ ] **Step 5: Commit**

```bash
git add plugins/window-switcher/src/backends/mod.rs plugins/window-switcher/src/lib.rs
git commit -m "feat(window-switcher): add configurable backend probe chain"
```

### Task 4: Implement i3 Backend

**Files:**
- Create: `plugins/window-switcher/src/backends/i3.rs`
- Modify: `plugins/window-switcher/src/backends/mod.rs`
- Test: `plugins/window-switcher/src/backends/i3.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn parses_i3_tree_into_windows() {
    let json = r#"{
      \"nodes\":[{
        \"type\":\"workspace\",\"name\":\"2:web\",\"nodes\":[{
          \"id\":123,\"type\":\"con\",\"name\":\"ChatGPT - Firefox\",\"window_properties\":{\"class\":\"firefox\"},\"nodes\":[],\"floating_nodes\":[]
        }],\"floating_nodes\":[]
      }],
      \"floating_nodes\":[]
    }"#;

    let windows = parse_windows_from_tree(json).expect("parse");
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].id, "123");
    assert_eq!(windows[0].workspace.as_deref(), Some("2:web"));
    assert_eq!(windows[0].app_id.as_deref(), Some("firefox"));
}

#[test]
fn focus_command_targets_con_id() {
    let cmd = focus_command("123");
    assert_eq!(cmd, vec!["[con_id=123]", "focus"]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p window-switcher backends::i3::tests -- --nocapture`
Expected: FAIL because i3 backend not implemented.

- [ ] **Step 3: Write minimal implementation**

```rust
pub struct I3Backend {
    runner: Arc<dyn CommandRunner>,
}

impl I3Backend {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self { Self { runner } }

    pub fn is_available(runner: &dyn CommandRunner, env: &ProbeEnv) -> bool {
        let desktop = env.xdg_current_desktop.as_deref().unwrap_or_default().to_ascii_lowercase();
        runner.command_exists("i3-msg") && (desktop.contains("i3") || env.wayland_display.is_none())
    }
}

impl WindowBackend for I3Backend {
    fn list_windows(&self) -> Vec<WindowInfo> {
        let Ok(tree_json) = self.runner.run("i3-msg", &["-t", "get_tree"]) else {
            return Vec::new();
        };
        parse_windows_from_tree(&tree_json).unwrap_or_default()
    }

    fn focus_window(&self, id: &str) -> Result<(), String> {
        let args = focus_command(id);
        self.runner.run("i3-msg", &[args[0], args[1]])?;
        Ok(())
    }

    fn name(&self) -> &'static str { "i3" }
}
```

Implement DFS tree walk extracting:
- container ID (`id`) as window ID
- title from `name`
- class from `window_properties.class`
- workspace name inherited from parent workspace node

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p window-switcher backends::i3::tests -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add plugins/window-switcher/src/backends/i3.rs plugins/window-switcher/src/backends/mod.rs
git commit -m "feat(window-switcher): add i3 backend"
```

### Task 5: Implement Sway Backend Using Same Tree Parser Pattern

**Files:**
- Create: `plugins/window-switcher/src/backends/sway.rs`
- Modify: `plugins/window-switcher/src/backends/mod.rs`
- Test: `plugins/window-switcher/src/backends/sway.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn sway_availability_depends_on_swaymsg() {
    let mut runner = FakeRunner::new();
    let env = ProbeEnv {
        wayland_display: Some("wayland-0".into()),
        xdg_current_desktop: Some("sway".into()),
        hypr_sig: None,
        niri_socket: None,
    };

    assert!(!SwayBackend::is_available(&runner, &env));
    runner.add_binary("swaymsg");
    assert!(SwayBackend::is_available(&runner, &env));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p window-switcher backends::sway::tests -- --nocapture`
Expected: FAIL because Sway backend does not exist.

- [ ] **Step 3: Write minimal implementation**

```rust
pub struct SwayBackend {
    runner: Arc<dyn CommandRunner>,
}

impl SwayBackend {
    pub fn is_available(runner: &dyn CommandRunner, env: &ProbeEnv) -> bool {
        let desktop = env.xdg_current_desktop.as_deref().unwrap_or_default().to_ascii_lowercase();
        runner.command_exists("swaymsg") && desktop.contains("sway")
    }
}

impl WindowBackend for SwayBackend {
    fn list_windows(&self) -> Vec<WindowInfo> {
        let Ok(tree_json) = self.runner.run("swaymsg", &["-t", "get_tree"]) else {
            return Vec::new();
        };
        i3::parse_windows_from_tree(&tree_json).unwrap_or_default()
    }

    fn focus_window(&self, id: &str) -> Result<(), String> {
        self.runner.run("swaymsg", &[&format!("[con_id={id}]"), "focus"])?;
        Ok(())
    }

    fn name(&self) -> &'static str { "sway" }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p window-switcher backends::sway::tests -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add plugins/window-switcher/src/backends/sway.rs plugins/window-switcher/src/backends/mod.rs
git commit -m "feat(window-switcher): add sway backend"
```

### Task 6: Adapt Existing Backends to Shared Availability API

**Files:**
- Modify: `plugins/window-switcher/src/backends/hyprland.rs`
- Modify: `plugins/window-switcher/src/backends/kwin.rs`
- Modify: `plugins/window-switcher/src/backends/niri.rs`
- Modify: `plugins/window-switcher/src/backends/mod.rs`
- Test: `plugins/window-switcher/src/backends/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn probe_order_prefers_first_available_backend() {
    let mut runner = FakeRunner::new();
    runner.add_binary("i3-msg");
    runner.add_binary("hyprctl");

    let env = ProbeEnv {
        wayland_display: Some("wayland-0".into()),
        xdg_current_desktop: Some("hyprland".into()),
        hypr_sig: Some("abc".into()),
        niri_socket: None,
    };

    let order = vec!["i3".into(), "hyprland".into()];
    let backend = detect_backend_with(&runner, &env, None, &order);
    assert_eq!(backend.map(|b| b.name()), Some("i3"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p window-switcher backends::tests::probe_order_prefers_first_available_backend -- --nocapture`
Expected: FAIL until backend constructors expose uniform probe path.

- [ ] **Step 3: Write minimal implementation**

Add per-backend API shape:

```rust
impl HyprlandBackend {
    pub fn is_available(runner: &dyn CommandRunner, env: &ProbeEnv) -> bool {
        env.hypr_sig.is_some() && runner.command_exists("hyprctl")
    }
}

impl KWinBackend {
    pub fn is_available(runner: &dyn CommandRunner, _env: &ProbeEnv) -> bool {
        runner.command_exists("kdotool")
    }
}

impl NiriBackend {
    pub fn is_available(_runner: &dyn CommandRunner, env: &ProbeEnv) -> bool {
        env.niri_socket.is_some() && Self::connect().is_some()
    }
}
```

Ensure `detect_backend_with` resolves backend names from `backend_probe_order`, skips unknown names with warning, and returns first available.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p window-switcher backends::tests -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add plugins/window-switcher/src/backends/mod.rs plugins/window-switcher/src/backends/hyprland.rs plugins/window-switcher/src/backends/kwin.rs plugins/window-switcher/src/backends/niri.rs
git commit -m "refactor(window-switcher): unify backend availability probing"
```

### Task 7: Docs and Distro Setup Guidance

**Files:**
- Modify: `plugins/window-switcher/README.md`

- [ ] **Step 1: Write the failing docs expectation test (manual check list)**

```text
README must include:
- Supported backends list: KWin, Hyprland, niri, Sway, i3
- Required binary per backend
- Distro package examples (Arch, Fedora, Debian/Ubuntu)
- Config examples for backend override/probe order
```

- [ ] **Step 2: Run validation to verify missing docs fail checklist**

Run: `rg -n "i3|sway|backend_probe_order|backend:" plugins/window-switcher/README.md`
Expected: Missing one or more required lines before edit.

- [ ] **Step 3: Write minimal implementation**

Add README sections with exact snippets:

```ron
Config(
  prefix: "w ",
  backend: Some("i3"),
  backend_probe_order: ["i3", "sway", "hyprland", "niri", "kwin"],
)
```

Add distro package hints:
- Arch: `pacman -S i3-wm sway hyprland niri`
- Fedora: `dnf install i3 sway hyprland niri`
- Debian/Ubuntu: `apt install i3-wm sway`

- [ ] **Step 4: Run validation to verify docs pass checklist**

Run: `rg -n "Supported backends|backend_probe_order|backend: Some\(\"i3\"\)|i3-msg|swaymsg" plugins/window-switcher/README.md`
Expected: Matches all required docs entries.

- [ ] **Step 5: Commit**

```bash
git add plugins/window-switcher/README.md
git commit -m "docs(window-switcher): add multi-distro backend setup and override examples"
```

### Task 8: Final Verification and Integration

**Files:**
- Modify: `plugins/window-switcher/Cargo.toml` (only if dependency additions were required)
- Verify workspace health

- [ ] **Step 1: Write final verification matrix**

```text
Required checks:
1) Unit tests for config and backend detection
2) i3 parser unit tests
3) sway backend unit tests
4) Plugin package build
```

- [ ] **Step 2: Run targeted tests**

Run: `cargo test -p window-switcher -- --nocapture`
Expected: PASS.

- [ ] **Step 3: Run compile check for full workspace impact**

Run: `just check`
Expected: PASS for workspace checks.

- [ ] **Step 4: Run format check**

Run: `cargo fmt --all -- --check`
Expected: PASS.

- [ ] **Step 5: Commit integration adjustments**

```bash
git add plugins/window-switcher/Cargo.toml plugins/window-switcher/src plugins/window-switcher/README.md
git commit -m "chore(window-switcher): finalize multi-backend support verification"
```

---

## Self-Review

- Spec coverage: i3 support added (Task 4), multi-distro extensibility via probe chain + override (Tasks 1-3,6), documentation for distro usage (Task 7), verification path (Task 8).
- Placeholder scan: no TODO/TBD placeholders, all tasks include concrete paths/commands/code.
- Type consistency: `backend`/`backend_probe_order`, `ProbeEnv`, `detect_backend_with`, backend `is_available` naming consistent across tasks.

Plan complete and saved to `docs/superpowers/plans/2026-05-24-window-switcher-multi-distro-backends.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
