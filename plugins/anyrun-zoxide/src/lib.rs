use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::icon::SystemIcon;
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::{fs, process::Command};

#[derive(Deserialize)]
struct Config {
    prefix: String,
    max_entries: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: "zo ".into(),
            max_entries: 5,
        }
    }
}

#[init]
fn init(config_dir: RString) -> Config {
    match fs::read_to_string(format!("{}/zoxide.ron", config_dir)) {
        Ok(content) => ron::from_str(&content).unwrap_or_else(|_| Config::default()),
        Err(_) => Config::default(),
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
fn get_matches(input: RString, config: &Config) -> RVec<Match> {
    if !input.starts_with(&config.prefix) {
        return RVec::new();
    }

    let query_str = &input[config.prefix.len()..];
    if query_str.is_empty() {
        return RVec::new();
    }

    let query_parts: Vec<&str> = query_str.split_whitespace().collect();

    let matcher = SkimMatcherV2::default();
    let all_paths = get_all_zoxide_paths();

    let mut matches: Vec<(i64, String)> = all_paths
        .into_iter()
        .filter_map(|path| {
            let mut total_score = 0;

            for part in &query_parts {
                if let Some(score) = matcher.fuzzy_match(&path, part) {
                    total_score += score;
                } else {
                    return None;
                }
            }
            Some((total_score, path))
        })
        .collect();

    matches.sort_by(|a, b| b.0.cmp(&a.0));

    matches
        .into_iter()
        .take(config.max_entries)
        .map(|(_score, path)| Match {
            title: path.into(),
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

    if let Err(why) = Command::new("kitty")
        .arg("--working-directory")
        .arg(path.as_str())
        .spawn()
    {
        eprintln!("[Libzoxide] Error: {}", why);
    }

    HandleResult::Close
}
