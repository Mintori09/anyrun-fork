use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::icon::SystemIcon;
use anyrun_plugin::*;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use walkdir::WalkDir;

const CONFIG_FILENAME: &str = "git-projects.ron";
const CACHE_FILENAME: &str = "git-projects.cache";

#[derive(Deserialize)]
struct Config {
    prefix: String,
    max_entries: usize,
    show_results_immediately: bool,
    cache_ttl_hours: u64,
    shell: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "git/".into(),
            max_entries: 100,
            show_results_immediately: true,
            cache_ttl_hours: 0,
            shell: "zsh".into(),
        }
    }
}

pub struct State {
    config: Config,
    repos: Vec<(String, String)>, // (repo_name, full_path)
}

fn load_cache(config_dir: &str, config: &Config) -> Option<Vec<(String, String)>> {
    let cache_path = PathBuf::from(config_dir).join(CACHE_FILENAME);
    let meta = fs::metadata(&cache_path).ok()?;
    let modified = meta.modified().ok()?;
    let elapsed = modified.elapsed().ok()?;

    if elapsed >= Duration::from_secs(config.cache_ttl_hours * 3600) {
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

    State { config, repos }
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

    let query = &input[state.config.prefix.len()..];

    if query.is_empty() && !state.config.show_results_immediately {
        return RVec::new();
    }

    let query_parts: Vec<&str> = query.split_whitespace().collect();

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
            icon: ROption::RSome(SystemIcon::Folder.as_str().into()),
            id: ROption::RNone,
        })
        .collect::<Vec<_>>()
        .into()
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    let path = selection.description.unwrap_or_default().to_string();
    if !path.is_empty() {
        anyrun_helper::terminal::launch(&format!("cd {} && exec {}", path, state.config.shell));
    }
    HandleResult::Close
}
