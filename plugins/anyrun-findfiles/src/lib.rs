use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

static CONFIG_STORE: OnceLock<Config> = OnceLock::new();

const DEFAULT_OPEN_COMMAND: &str = "xdg-open {}";
const GLOBAL_SCOPE_ID: u64 = u64::MAX;
const CONFIG_FILE_NAME: &str = "findfiles.ron";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchScope {
    pub path: String,
    pub prefix: String,
    pub excludes: Vec<String>,
    pub command: Option<String>,
}

impl Default for SearchScope {
    fn default() -> Self {
        Self {
            path: env::var("HOME").unwrap_or_else(|_| "/".into()),
            prefix: String::new(),
            excludes: Vec::new(),
            command: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterRule {
    pub hidden: bool,
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub prefix: String,
    pub default_command: String,
    pub scopes: Vec<SearchScope>,
    pub options: FilterRule,
    pub max_entries: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "fd ".into(),
            default_command: DEFAULT_OPEN_COMMAND.into(),
            scopes: Vec::new(),
            options: FilterRule::default(),
            max_entries: 10,
        }
    }
}

struct SearchRunner<'a> {
    config: &'a Config,
}

impl<'a> SearchRunner<'a> {
    fn new(config: &'a Config) -> Self {
        Self { config }
    }

    fn format_query_to_regex(&self, query: &str) -> String {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        trimmed
            .split_whitespace()
            .map(|word| {
                word.chars()
                    .map(|c| {
                        if ".+*?()|[]{}^$\\".contains(c) {
                            format!("\\{}", c)
                        } else {
                            c.to_string()
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(".*")
    }

    fn run_fd_search(&self, query: &str, path: &str, excludes: &[String], id: u64) -> Vec<Match> {
        let pattern = self.format_query_to_regex(query);
        let mut cmd = Command::new("fd");

        cmd.args(["--color", "never", "--full-path"]);
        cmd.arg("--max-results")
            .arg(self.config.max_entries.to_string());

        if self.config.options.hidden {
            cmd.arg("--hidden");
        }

        for exclusion in excludes {
            cmd.arg("--exclude").arg(exclusion);
        }

        if pattern.is_empty() {
            cmd.arg(".").arg(path);
        } else {
            cmd.arg(&pattern).arg(path);
        }

        cmd.output()
            .map(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                self.parse_fd_output(&stdout, id)
            })
            .unwrap_or_default()
    }

    fn parse_fd_output(&self, stdout: &str, id: u64) -> Vec<Match> {
        stdout
            .lines()
            .filter_map(|line| {
                let path = Path::new(line);
                let filename = path.file_name()?.to_str()?;
                let icon = if path.is_dir() {
                    "folder"
                } else {
                    "text-x-generic"
                };

                Some(Match {
                    title: filename.into(),
                    description: ROption::RSome(line.to_string().into()),
                    icon: ROption::RSome(icon.into()),
                    use_pango: false,
                    id: ROption::RSome(id),
                })
            })
            .collect()
    }
}

#[init]
fn init(config_dir: RString) -> Config {
    let config_path = PathBuf::from(config_dir.to_string()).join(CONFIG_FILE_NAME);
    let config = load_config(config_path);
    config
}

fn load_config(path: PathBuf) -> Config {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_default()
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Find Files".into(),
        icon: "folder-saved-search".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, config: &Config) -> RVec<Match> {
    let input_str = input.trim();
    if input_str.is_empty() {
        return RVec::new();
    }

    let runner = SearchRunner::new(config);

    for (idx, scope) in config.scopes.iter().enumerate() {
        if input_str.starts_with(&scope.prefix) {
            let query = input_str.trim_start_matches(&scope.prefix).trim();
            return runner
                .run_fd_search(query, &scope.path, &scope.excludes, idx as u64)
                .into();
        }
    }

    if !config.prefix.is_empty() && input_str.starts_with(&config.prefix) {
        let query = input_str.trim_start_matches(&config.prefix).trim();
        let home = env::var("HOME").unwrap_or_else(|_| "/".into());
        return runner
            .run_fd_search(query, &home, &[], GLOBAL_SCOPE_ID)
            .into();
    }

    RVec::new()
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    let path = match selection.description {
        ROption::RSome(p) => p,
        ROption::RNone => return HandleResult::Close,
    };

    let template = resolve_execution_template(selection.id);
    let final_command = template.replace("{}", &path);

    let _ = Command::new("sh").arg("-c").arg(final_command).spawn();

    HandleResult::Close
}

fn resolve_execution_template(match_id: ROption<u64>) -> String {
    let config = match CONFIG_STORE.get() {
        Some(cfg) => cfg,
        None => return DEFAULT_OPEN_COMMAND.to_string(),
    };

    match match_id {
        ROption::RSome(id) if id != GLOBAL_SCOPE_ID => config
            .scopes
            .get(id as usize)
            .and_then(|s| s.command.as_ref())
            .cloned()
            .unwrap_or_else(|| config.default_command.clone()),
        _ => config.default_command.clone(),
    }
}
