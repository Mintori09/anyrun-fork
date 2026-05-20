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
    #[serde(default = "default_terminal")]
    terminal: bool,
}

const fn default_terminal() -> bool {
    true
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

pub struct State {
    config: Config,
    repos: Vec<(String, String)>,
    resolved_default: String,
}

fn load_cache(config_dir: &str, config: &Config) -> Option<Vec<(String, String)>> {
    let cache_path = PathBuf::from(config_dir).join(CACHE_FILENAME);
    let meta = fs::metadata(&cache_path).ok()?;
    let modified = meta.modified().ok()?;
    let elapsed = modified.elapsed().ok()?;

    if elapsed >= Duration::from_secs(config.cache_ttl_hours.saturating_mul(3600)) {
        return None;
    }

    let content = fs::read_to_string(&cache_path).ok()?;
    let repos: Vec<(String, String)> = ron::from_str(&content).ok()?;
    eprintln!("[git-projects] loaded {} repos from cache", repos.len());
    Some(repos)
}

fn save_cache(config_dir: &str, repos: &[(String, String)]) {
    let cache_path = PathBuf::from(config_dir).join(CACHE_FILENAME);
    if let Ok(content) = ron::to_string(repos) {
        let _ = fs::write(&cache_path, content);
    }
}

fn find_git_repos(root: &str) -> Vec<(String, String)> {
    let mut repos = Vec::new();
    for entry in WalkDir::new(root)
        .max_depth(10)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') || name == ".git"
        })
        .flatten()
    {
        if entry.file_type().is_dir() && entry.file_name() == ".git" {
            if let Some(parent) = entry.path().parent() {
                let name = parent
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let path = parent.to_string_lossy().to_string();
                repos.push((name, path));
            }
        }
    }
    repos.sort_by_key(|a| a.0.to_lowercase());
    repos
}

#[init]
fn init(config_dir: RString) -> State {
    let config_dir = config_dir.to_string();
    let config_path = PathBuf::from(&config_dir).join(CONFIG_FILENAME);
    let config: Config = fs::read_to_string(&config_path)
        .map_err(|e| format!("IO Error: {}", e))
        .and_then(|content| ron::from_str(&content).map_err(|e| format!("RON Error: {}", e)))
        .unwrap_or_else(|_err| Config::default());

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
                let shell = raw
                    .lines()
                    .find(|l| l.contains("shell:"))
                    .and_then(|l| l.split("shell:").nth(1))
                    .map(|s| s.trim().trim_end_matches(',').trim_matches('"').to_string())
                    .unwrap_or_else(|| "zsh".to_string());
                format!("cd {{path}} && exec {}", shell)
            } else {
                "cd {path} && exec $SHELL".to_string()
            }
        } else {
            "cd {path} && exec $SHELL".to_string()
        }
    });

    State {
        config,
        repos,
        resolved_default,
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Git Projects".into(),
        icon: "folder".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    if !input.starts_with(&state.config.prefix) {
        return RVec::new();
    }

    let rest = &input[state.config.prefix.len()..];

    if rest.is_empty() && !state.config.show_results_immediately {
        return RVec::new();
    }

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

    let (icon, command_template, needs_terminal) = match sub_key {
        Some(ref key) => {
            let cmd = &state.config.commands[key];
            (cmd.icon.clone(), cmd.command.clone(), cmd.terminal)
        }
        None => (
            SystemIcon::Folder.as_str().to_string(),
            state.resolved_default.clone(),
            false,
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
            description: ROption::RSome(
                format!("{}\0{}\0{}", path, command_template, needs_terminal as u8).into(),
            ),
            use_pango: false,
            icon: ROption::RSome(icon.clone().into()),
            id: ROption::RNone,
        })
        .collect::<Vec<_>>()
        .into()
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    let desc = match selection.description {
        ROption::RSome(ref d) => d.to_string(),
        _ => return HandleResult::Close,
    };

    let mut parts = desc.splitn(3, '\0');
    let path = parts.next().unwrap_or(&desc).to_string();
    let cmd_template = parts.next().unwrap_or(&state.resolved_default).to_string();
    let needs_terminal = parts.next().is_none_or(|s| s == "1");

    let escaped_path = shell_escape_single_arg(&path);
    let resolved = cmd_template.replace("{path}", &escaped_path);

    if needs_terminal {
        anyrun_helper::terminal::launch(&resolved);
    } else if let Err(e) = std::process::Command::new("sh")
        .arg("-c")
        .arg(&resolved)
        .spawn()
    {
        eprintln!("[git-projects] failed to execute command: {}", e);
    }

    HandleResult::Close
}

fn shell_escape_single_arg(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    let mut escaped = String::from("'");

    for ch in value.chars() {
        if ch == '\'' {
            escaped.push_str("'\\''");
        } else {
            escaped.push(ch);
        }
    }

    escaped.push('\'');
    escaped
}

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
                "nvim": (command: "nvim {path}", icon: "nvim", terminal: true),
                "code": (command: "code {path}", icon: "code", terminal: false),
            },
        )"#;
        let config: Config = ron::from_str(content).expect("valid config");
        assert_eq!(config.prefix, "git/");
        assert_eq!(config.max_entries, 5);
        assert_eq!(
            config.default_command,
            Some("kitty --directory {path}".into())
        );
        assert_eq!(config.commands.len(), 2);
        assert_eq!(config.commands["nvim"].command, "nvim {path}");
        assert_eq!(config.commands["code"].icon, "code");
        assert!(config.commands["nvim"].terminal);
        assert!(!config.commands["code"].terminal);
    }

    #[test]
    fn command_template_replaces_path() {
        let template = "nvim {path}".to_string();
        let path = "/home/user/my repo";
        let escaped = shell_escape_single_arg(path);
        let resolved = template.replace("{path}", &escaped);
        assert_eq!(resolved, "nvim '/home/user/my repo'");
    }

    #[test]
    fn terminal_defaults_to_true() {
        let content = r#"SubCommand(command: "nvim {path}", icon: "nvim")"#;
        let cmd: SubCommand = ron::from_str(content).expect("valid subcommand");
        assert!(cmd.terminal);
    }

    #[test]
    fn terminal_can_be_false() {
        let content = r#"SubCommand(command: "code {path}", icon: "code", terminal: false)"#;
        let cmd: SubCommand = ron::from_str(content).expect("valid subcommand");
        assert!(!cmd.terminal);
    }
}
