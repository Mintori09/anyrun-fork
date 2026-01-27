use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::icon::SystemIcon;
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::{fs, process::Command};

pub struct State {
    config: Config,
    matcher: SkimMatcherV2,
    zoxide: Vec<String>,
}

#[derive(Deserialize)]
struct Config {
    prefix: String,
    max_entries: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: "z ".into(),
            max_entries: 5,
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
        matcher: SkimMatcherV2::default(),
        zoxide: get_all_zoxide_paths(),
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

    let query_str = &input[state.config.prefix.len()..];
    if query_str.is_empty() {
        return RVec::new();
    }

    let query_parts: Vec<&str> = query_str.split_whitespace().collect();

    state
        .zoxide
        .iter()
        .filter(|path| {
            // "Just find": Check if all query parts exist in the path
            query_parts
                .iter()
                .all(|part| state.matcher.fuzzy_match(path, part).is_some())
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

    let Some(terminal) = anyrun_helper::terminal::get_available_terminal() else {
        eprintln!("[Libzoxide] Error: No terminal available");
        return HandleResult::Close;
    };

    if let Err(why) = Command::new(&terminal)
        .arg("--working-directory")
        .arg(path.as_str())
        .spawn()
    {
        eprintln!("[Libzoxide] Error: {}", why);
    }

    HandleResult::Close
}
