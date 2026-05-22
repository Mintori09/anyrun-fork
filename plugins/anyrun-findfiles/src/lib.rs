use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_OPEN_COMMAND: &str = "xdg-open {}";
const GLOBAL_SCOPE_ID: u64 = u64::MAX;
const CONFIG_FILE_NAME: &str = "findfiles.ron";
const FOLDER_ICON: &str = "folder";
const FILE_ICON: &str = "text-x-generic";
const PLUGIN_ICON: &str = "folder-saved-search";
const PLUGIN_NAME: &str = "Find Files";

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

struct SearchEngine<'a> {
    config: &'a Config,
}

impl<'a> SearchEngine<'a> {
    fn new(config: &'a Config) -> Self {
        Self { config }
    }

    fn build_regex_pattern(&self, query: &str) -> String {
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

    fn execute_fd(
        &self,
        query: &str,
        search_root: &str,
        excludes: &[String],
        id: u64,
    ) -> Vec<Match> {
        let pattern = self.build_regex_pattern(query);
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
            cmd.arg(".").arg(search_root);
        } else {
            cmd.arg(&pattern).arg(search_root);
        }

        cmd.output()
            .map(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                self.create_matches_from_output(&stdout, id)
            })
            .unwrap_or_default()
    }

    fn create_matches_from_output(&self, stdout: &str, scope_id: u64) -> Vec<Match> {
        stdout
            .lines()
            .filter_map(|line| {
                let path = Path::new(line);
                let filename = path.file_name()?.to_str()?;
                let icon = if path.is_dir() {
                    FOLDER_ICON
                } else {
                    FILE_ICON
                };

                Some(Match {
                    title: filename.into(),
                    description: ROption::RSome(line.to_string().into()),
                    icon: ROption::RSome(icon.into()),
                    use_pango: false,
                    id: ROption::RSome(scope_id),
                })
            })
            .collect()
    }
}

#[init]
fn init(config_dir: RString) -> Config {
    let config_path = PathBuf::from(config_dir.to_string()).join(CONFIG_FILE_NAME);
    load_config(config_path)
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
        name: PLUGIN_NAME.into(),
        icon: PLUGIN_ICON.into(),
    }
}

#[get_matches]
fn get_matches(input: RString, config: &Config) -> RVec<Match> {
    let input_str = input.trim();
    if input_str.is_empty() {
        return RVec::new();
    }

    let engine = SearchEngine::new(config);

    for (index, scope) in config.scopes.iter().enumerate() {
        if input_str.starts_with(&scope.prefix) {
            let query = input_str.trim_start_matches(&scope.prefix).trim();
            return engine
                .execute_fd(query, &scope.path, &scope.excludes, index as u64)
                .into();
        }
    }

    if !config.prefix.is_empty() && input_str.starts_with(&config.prefix) {
        let query = input_str.trim_start_matches(&config.prefix).trim();
        let home_dir = env::var("HOME").unwrap_or_else(|_| "/".into());
        return engine
            .execute_fd(query, &home_dir, &[], GLOBAL_SCOPE_ID)
            .into();
    }

    RVec::new()
}

#[handler]
fn handler(selection: Match, config: &Config) -> HandleResult {
    let entry_path = match selection.description {
        ROption::RSome(path) => path,
        ROption::RNone => return HandleResult::Close,
    };

    let template = get_execution_template(selection.id, config);
    let command_string = template.replace("{}", &entry_path);

    let _ = Command::new("sh").arg("-c").arg(command_string).spawn();

    HandleResult::Close
}

fn get_execution_template(scope_id: ROption<u64>, config: &Config) -> String {
    match scope_id {
        ROption::RSome(id) if id != GLOBAL_SCOPE_ID => config
            .scopes
            .get(id as usize)
            .and_then(|scope| scope.command.as_ref())
            .cloned()
            .unwrap_or_else(|| config.default_command.clone()),
        _ => config.default_command.clone(),
    }
}
