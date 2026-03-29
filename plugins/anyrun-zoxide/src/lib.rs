use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::{icon::SystemIcon, terminal};
use anyrun_plugin::*;
use serde::Deserialize;
use std::{fs, process::Command};

use std::sync::RwLock;
use std::time::{Duration, Instant};

pub struct State {
    config: Config,
    zoxide: RwLock<(Instant, Vec<String>)>,
}

#[derive(Deserialize)]
struct Config {
    prefix: String,
    max_entries: usize,
    #[serde(default = "default_cache_ttl")]
    cache_ttl_secs: u64,
}

fn default_cache_ttl() -> u64 {
    30
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: "z ".into(),
            max_entries: 5,
            cache_ttl_secs: default_cache_ttl(),
        }
    }
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = std::path::PathBuf::from(config_dir.to_string()).join("zoxide.ron");

    let config: Config = fs::read_to_string(&config_path)
        .map_err(|e| format!("IO Error: {}", e))
        .and_then(|content| ron::from_str(&content).map_err(|e| format!("RON Error: {}", e)))
        .unwrap_or_else(|_err| Config::default());

    State {
        config,
        zoxide: RwLock::new((Instant::now(), get_all_zoxide_paths())),
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Zoxide Fuzzy".into(),
        icon: "folder-open".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    if !input.starts_with(&state.config.prefix) {
        return RVec::new();
    }

    let needs_update = {
        let cache = state.zoxide.read().unwrap();
        cache.0.elapsed() > Duration::from_secs(state.config.cache_ttl_secs)
    };

    if needs_update {
        let paths = get_all_zoxide_paths();
        let mut cache = state.zoxide.write().unwrap();
        *cache = (Instant::now(), paths);
    }

    let query_str = &input[state.config.prefix.len()..];
    if query_str.is_empty() {
        return RVec::new();
    }

    let query_parts: Vec<&str> = query_str.split_whitespace().collect();

    let cache = state.zoxide.read().unwrap();
    cache
        .1
        .iter()
        .filter(|path| {
            // "Just find": Check if all query parts exist in the path
            query_parts
                .iter()
                .all(|part| anyrun_helper::mazzy_matcher::fuzzy_match(path, part).is_some())
        })
        .take(state.config.max_entries) // Stop looking once we hit the limit
        .map(|path| Match {
            title: path.clone().into(),
            description: ROption::RSome("Zoxide directory".into()),
            use_pango: false,
            icon: ROption::RSome(SystemIcon::Folder.as_str().into()),
            id: ROption::RNone,
        })
        .collect::<Vec<_>>()
        .into()
}

fn get_all_zoxide_paths() -> Vec<String> {
    let output = Command::new("zoxide").arg("query").arg("--list").output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect()
        }
        Err(_) => Vec::new(),
    }
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    let path = selection.title;
    launch_terminal_at_path(path.as_str());
    HandleResult::Close
}

pub fn launch_terminal_at_path(path: &str) {
    let terminal = match terminal::get_available_terminal() {
        Some(t) => t,
        None => {
            eprintln!("[Zoxide] Error: No supported terminal emulator found in PATH.");
            return;
        }
    };

    let mut cmd = Command::new(&terminal);
    terminal::configure_terminal_environment(&mut cmd);

    let result = cmd.arg("--working-directory").arg(path).spawn();

    if let Err(error) = result {
        eprintln!("[Zoxide] Failed to spawn {}: {}", terminal, error);
    }
}
