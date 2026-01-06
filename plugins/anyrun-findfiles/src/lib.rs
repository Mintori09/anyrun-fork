use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self};
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchScope {
    path: String,
    prefix: String,
    excludes: Vec<String>,
    command: Option<String>,
}

impl Default for SearchScope {
    fn default() -> Self {
        Self {
            path: env::var("HOME").unwrap_or_else(|_| "/".into()),
            prefix: "".into(),
            excludes: vec![],
            command: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct FilterRule {
    hidden: bool,
    patterns: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
struct Config {
    prefix: String,
    default_command: String,
    scopes: Vec<SearchScope>,
    options: FilterRule,
    max_entries: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: ":f".into(),
            default_command: "xdg-open {}".into(),
            scopes: vec![],
            options: FilterRule::default(),
            max_entries: 10,
        }
    }
}

// --- CORE ENGINE ---

struct SearchEngine<'a> {
    config: &'a Config,
}

impl<'a> SearchEngine<'a> {
    fn new(config: &'a Config) -> Self {
        Self { config }
    }

    fn build_regex(&self, query: &str) -> String {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            return String::new();
        }

        trimmed_query
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

    fn execute_search(
        &self,
        query: &str,
        path: &str,
        excludes: &[String],
        scope_idx: Option<usize>,
    ) -> Vec<Match> {
        let regex = self.build_regex(query);

        let mut cmd = Command::new("fd");
        cmd.args(["--color", "never", "--full-path"]);

        cmd.arg("--max-results")
            .arg((self.config.max_entries).to_string());

        if self.config.options.hidden {
            cmd.arg("--hidden");
        }

        for exc in excludes {
            cmd.arg("--exclude").arg(exc);
        }

        if regex.is_empty() {
            cmd.arg(".").arg(path);
        } else {
            cmd.arg(&regex).arg(path);
        }

        let match_id = scope_idx.map(|i| i as u64).unwrap_or(u64::MAX);

        cmd.output()
            .map(|out| self.to_matches(&String::from_utf8_lossy(&out.stdout), match_id))
            .unwrap_or_default()
    }

    fn to_matches(&self, stdout: &str, match_id: u64) -> Vec<Match> {
        stdout
            .lines()
            .filter_map(|line| {
                let path = Path::new(line);
                let name = path.file_name()?.to_str()?;

                Some(Match {
                    title: name.into(),
                    description: ROption::RSome(line.to_string().into()),
                    icon: ROption::RSome(
                        (if path.is_dir() {
                            "folder"
                        } else {
                            "text-x-generic"
                        })
                        .into(),
                    ),
                    use_pango: false,
                    id: ROption::RSome(match_id),
                })
            })
            .collect()
    }
}

// --- PLUGIN HOOKS ---

#[init]
fn init(config_dir: RString) -> Config {
    let path = Path::new(config_dir.as_str()).join("findfiles.ron");
    let config: Config = fs::read_to_string(path)
        .ok()
        .and_then(|c| ron::from_str(&c).ok())
        .unwrap_or_default();

    // Store config globally so handler can access custom commands
    let _ = CONFIG.set(config.clone());

    config
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

    let engine = SearchEngine::new(config);
    let home = env::var("HOME").unwrap_or_else(|_| "/".into());
    for (idx, scope) in config.scopes.iter().enumerate() {
        if input_str.starts_with(&scope.prefix) {
            let query = input_str.trim_start_matches(&scope.prefix).trim();
            return engine
                .execute_search(query, &scope.path, &scope.excludes, Some(idx))
                .into();
        }
    }
    if !config.prefix.is_empty() && input_str.starts_with(&config.prefix) {
        let query = input_str.trim_start_matches(&config.prefix).trim();
        return engine.execute_search(query, &home, &[], None).into();
    }

    RVec::new()
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    let path = match selection.description {
        ROption::RSome(p) => p,
        ROption::RNone => return HandleResult::Close,
    };

    let config = CONFIG.get();

    let command_template = match (config, selection.id) {
        (Some(cfg), ROption::RSome(id)) if id != u64::MAX => cfg
            .scopes
            .get(id as usize)
            .and_then(|s| s.command.as_ref())
            .unwrap_or(&cfg.default_command),
        (Some(cfg), _) => &cfg.default_command,
        (None, _) => "xdg-open {}",
    };

    let final_command = command_template.replace("{}", &path);

    let _ = Command::new("sh").arg("-c").arg(final_command).spawn();

    HandleResult::Close
}
