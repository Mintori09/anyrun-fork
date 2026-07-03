# Git Projects Sub-command Routing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add configurable sub-command routing to `anyrun-git-projects` so users can type `git/nvim myrepo` to open in Neovim, `git/op myrepo` for OpenCode, or `git/ myrepo` (bare) for default terminal.

**Architecture:** Replace `shell` config field with `default_command` template + `commands` map. After stripping the main prefix, extract the first word as a sub-command key; if found in the map, use its command template and icon. Store the resolved command in Match `id`. Handler executes via `sh -c` for sub-commands or `terminal::launch` for default.

**Tech Stack:** Rust, abi_stable, serde, anyrun-plugin, anyrun-helper, walkdir

---

### Task 1: Update Config and Add SubCommand Struct

**Files:**
- Modify: `plugins/anyrun-git-projects/src/lib.rs`
- Modify: `plugins/anyrun-git-projects/git-projects.ron`

- [x] **Step 1: Replace Config struct and add SubCommand**

Remove `shell` field. Add `default_command` (Option<String>) and `commands` (HashMap<String, SubCommand>).

```rust
use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::icon::SystemIcon;
use anyrun_plugin::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use walkdir::WalkDir;

const CONFIG_FILENAME: &str = "git-projects.ron";
const CACHE_FILENAME: &str = "git-projects.cache";

#[derive(Deserialize)]
struct SubCommand {
    command: String,
    icon: String,
}

#[derive(Deserialize)]
struct Config {
    prefix: String,
    max_entries: usize,
    show_results_immediately: bool,
    cache_ttl_hours: u64,
    #[serde(default)]
    default_command: Option<String>,
    #[serde(default)]
    commands: HashMap<String, SubCommand>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "git/".into(),
            max_entries: 100,
            show_results_immediately: true,
            cache_ttl_hours: 0,
            default_command: None,
            commands: HashMap::new(),
        }
    }
}
```

- [x] **Step 2: Update State struct — keep `config` and `repos`, add `resolved_default`**

Store the resolved default command so we compute it once (handles migration from shell):

```rust
pub struct State {
    config: Config,
    repos: Vec<(String, String)>,
    resolved_default: String,
}
```

- [x] **Step 3: Update `init` to compute `resolved_default`**

```rust
#[init]
fn init(config_dir: RString) -> State {
    let config_dir = config_dir.to_string();
    let config_path = PathBuf::from(&config_dir).join(CONFIG_FILENAME);
    let content = fs::read_to_string(&config_path).unwrap_or_default();
    let config: Config = ron::from_str(&content)
        .map_err(|e| { eprintln!("[git-projects] config error: {}", e); })
        .unwrap_or_default();

    let repos = load_cache(&config_dir, &config).unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".into());
        let repos = find_git_repos(&home);
        eprintln!("[git-projects] found {} repos in {}", repos.len(), home);
        save_cache(&config_dir, &repos);
        repos
    });

    let resolved_default = config.default_command.clone().unwrap_or_else(|| {
        if let Ok(raw) = fs::read_to_string(&config_path) {
            if raw.contains("shell:") {
                // Extract shell value from old config format
                let shell = raw
                    .lines()
                    .find(|l| l.contains("shell:"))
                    .and_then(|l| l.split("shell:").nth(1))
                    .and_then(|s| {
                        let s = s.trim().trim_end_matches(',');
                        Some(s.trim_matches('"').to_string())
                    })
                    .unwrap_or_else(|| "zsh".to_string());
                format!("cd {{path}} && exec {}", shell)
            } else {
                format!("cd {{path}} && exec $SHELL")
            }
        } else {
            format!("cd {{path}} && exec $SHELL")
        }
    });

    State { config, repos, resolved_default }
}
```

- [x] **Step 4: Commit**

```bash
git add plugins/anyrun-git-projects/src/lib.rs
git commit -m "feat(git-projects): add SubCommand struct and update Config"
```

### Task 2: Update `get_matches` for Sub-command Routing

**Files:**
- Modify: `plugins/anyrun-git-projects/src/lib.rs`

- [x] **Step 1: Rewrite `get_matches`**

```rust
#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    if !input.starts_with(&state.config.prefix) {
        return RVec::new();
    }

    let rest = &input[state.config.prefix.len()..];

    if rest.is_empty() && !state.config.show_results_immediately {
        return RVec::new();
    }

    // Extract optional sub-command key (first whitespace-delimited word)
    let (sub_key, query) = match rest.split_once(char::is_whitespace) {
        Some((key, remainder)) if state.config.commands.contains_key(key) => {
            (Some(key.to_string()), remainder.trim())
        }
        _ => (None, rest.trim()),
    };

    let query_parts: Vec<&str> = if query.is_empty() {
        Vec::new()
    } else {
        query.split_whitespace().collect()
    };

    let (icon, command_template) = match sub_key {
        Some(ref key) => {
            let cmd = &state.config.commands[key];
            (cmd.icon.clone(), cmd.command.clone())
        }
        None => (
            SystemIcon::Folder.as_str().to_string(),
            state.resolved_default.clone(),
        ),
    };

    state
        .repos
        .iter()
        .filter(|(name, _)| {
            query.is_empty()
                || query_parts
                    .iter()
                    .all(|part| anyrun_helper::mazzy_matcher::fuzzy_match(name, part).is_some())
        })
        .take(state.config.max_entries)
        .map(|(name, path)| Match {
            title: name.to_owned().into(),
            description: ROption::RSome(path.to_owned().into()),
            use_pango: false,
            icon: ROption::RSome(icon.clone().into()),
            id: ROption::RSome(command_template.clone().into()),
        })
        .collect::<Vec<_>>()
        .into()
}
```

- [x] **Step 2: Commit**

```bash
git add plugins/anyrun-git-projects/src/lib.rs
git commit -m "feat(git-projects): add sub-command routing in get_matches"
```

### Task 3: Update Handler for Command Template Execution

**Files:**
- Modify: `plugins/anyrun-git-projects/src/lib.rs`

- [x] **Step 1: Rewrite `handler`**

Replace the old handler that always used `terminal::launch` with one that checks `selection.id` for a command template or falls back to `terminal::launch`.

```rust
#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    let path = match selection.description {
        ROption::RSome(ref p) => p.to_string(),
        _ => return HandleResult::Close,
    };

    let cmd_template = match selection.id {
        ROption::RSome(ref t) => t.to_string(),
        ROption::RNone => state.resolved_default.clone(),
    };

    let escaped_path = shell_escape_single_arg(&path);
    let resolved = cmd_template.replace("{path}", &escaped_path);

    if let Err(e) = std::process::Command::new("sh")
        .arg("-c")
        .arg(&resolved)
        .spawn()
    {
        eprintln!("[git-projects] failed to execute command: {}", e);
    }

    HandleResult::Close
}
```

Remove the old `shell_escape_single_arg` function... wait, we still need it. Keep it.

- [x] **Step 2: Update `info` to show dynamic name**

```rust
#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Git Projects".into(),
        icon: "folder".into(),
    }
}
```
No changes needed here.

- [x] **Step 3: Commit**

```bash
git add plugins/anyrun-git-projects/src/lib.rs
git commit -m "feat(git-projects): use command templates in handler"
```

### Task 4: Update Tests

**Files:**
- Modify: `plugins/anyrun-git-projects/src/lib.rs` (test module)

- [x] **Step 1: Add tests for sub-command parsing and command resolution**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_path_with_spaces() {
        let path = "/home/mintori/My Projects/repo";
        assert_eq!(
            shell_escape_single_arg(path),
            "'/home/mintori/My Projects/repo'"
        );
    }

    #[test]
    fn escape_path_with_single_quote() {
        let path = "/home/mintori/O'Hara/repo";
        assert_eq!(
            shell_escape_single_arg(path),
            "'/home/mintori/O'\\''Hara/repo'"
        );
    }

    #[test]
    fn cache_ttl_duration_uses_saturating_mul() {
        let secs = u64::MAX.saturating_mul(3600);
        assert_eq!(secs, u64::MAX);
    }

    #[test]
    fn config_defaults() {
        let config = Config::default();
        assert_eq!(config.prefix, "git/");
        assert_eq!(config.max_entries, 100);
        assert!(config.default_command.is_none());
        assert!(config.commands.is_empty());
    }

    #[test]
    fn config_parses_new_format() {
        let content = r#"Config(
            prefix: "git/",
            max_entries: 5,
            show_results_immediately: true,
            cache_ttl_hours: 12,
            default_command: Some("kitty --directory {path}"),
            commands: {
                "nvim": (command: "nvim {path}", icon: "nvim"),
                "code": (command: "code {path}", icon: "code"),
            },
        )"#;
        let config: Config = ron::from_str(content).expect("valid config");
        assert_eq!(config.prefix, "git/");
        assert_eq!(config.max_entries, 5);
        assert_eq!(config.default_command, Some("kitty --directory {path}".into()));
        assert_eq!(config.commands.len(), 2);
        assert_eq!(config.commands["nvim"].command, "nvim {path}");
        assert_eq!(config.commands["code"].icon, "code");
    }

    #[test]
    fn command_template_replaces_path() {
        let template = "nvim {path}".to_string();
        let path = "/home/user/my repo";
        let escaped = shell_escape_single_arg(path);
        let resolved = template.replace("{path}", &escaped);
        assert_eq!(resolved, "nvim '/home/user/my repo'");
    }
}
```

- [x] **Step 2: Run tests**

Run: `cargo test -p anyrun-git-projects`
Expected: All tests pass

- [x] **Step 3: Commit**

```bash
git add plugins/anyrun-git-projects/src/lib.rs
git commit -m "test(git-projects): add tests for new config and command resolution"
```

### Task 5: Update Default Config File

**Files:**
- Modify: `plugins/anyrun-git-projects/git-projects.ron`

- [x] **Step 1: Write new config**

```ron
Config(
    prefix: "git/",
    max_entries: 10,
    show_results_immediately: true,
    cache_ttl_hours: 0,
    default_command: Some("kitty --directory {path}"),
    commands: {
        "nvim": (
            command: "nvim {path}",
            icon: "nvim",
        ),
        "code": (
            command: "code {path}",
            icon: "visual-studio-code",
        ),
        "op": (
            command: "opencode {path}",
            icon: "system-search",
        ),
    },
)
```

- [x] **Step 2: Commit**

```bash
git add plugins/anyrun-git-projects/git-projects.ron
git commit -m "feat(git-projects): update default config with sub-commands"
```

### Task 6: Verify

- [x] **Step 1: cargo check**

Run: `cargo check -p anyrun-git-projects`
Expected: Compiles without errors

- [x] **Step 2: cargo fmt**

Run: `cargo fmt --all --check`
Expected: No formatting issues

- [x] **Step 3: cargo clippy**

Run: `cargo clippy -p anyrun-git-projects -- -D warnings`
Expected: No warnings

- [x] **Step 4: cargo test**

Run: `cargo test -p anyrun-git-projects`
Expected: All tests pass

- [x] **Step 5: Final commit if needed**

```bash
git add -A
git commit -m "chore(git-projects): fix formatting and clippy issues" || true
```
