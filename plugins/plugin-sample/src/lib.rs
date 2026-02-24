use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::icon::SystemIcon;
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::fs;

const PLUGIN_NAME: &str = "";

#[derive(Deserialize)]
struct Config {
    prefix: String,
    max_entries: usize,
    show_results_immediately: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "ex ".into(),
            max_entries: 5,
            show_results_immediately: false,
        }
    }
}

pub struct State {
    config: Config,
    matcher: SkimMatcherV2,
    data: Vec<String>,
}

fn fetch_data() -> Vec<String> {
    vec![]
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = std::path::PathBuf::from(config_dir.to_string()).join(PLUGIN_NAME);

    let config: Config = fs::read_to_string(&config_path)
        .map_err(|e| format!("IO Error: {}", e))
        .and_then(|content| ron::from_str(&content).map_err(|e| format!("RON Error: {}", e)))
        .unwrap_or_else(|_err| Config::default());

    State {
        config,
        matcher: SkimMatcherV2::default(),
        data: fetch_data(),
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Generic Plugin Name".into(),
        icon: "system-search".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    if !input.starts_with(&state.config.prefix) {
        return RVec::new();
    }

    let query_str = &input[state.config.prefix.len()..];

    if query_str.is_empty() {
        if state.config.show_results_immediately {
            return state
                .data
                .iter()
                .take(state.config.max_entries)
                .map(|item| item_to_match(item))
                .collect::<Vec<_>>()
                .into();
        } else {
            return RVec::new();
        }
    }

    let query_parts: Vec<&str> = query_str.split_whitespace().collect();

    state
        .data
        .iter()
        .filter(|item| {
            query_parts
                .iter()
                .all(|part| state.matcher.fuzzy_match(item, part).is_some())
        })
        .take(state.config.max_entries)
        .map(|item| item_to_match(item))
        .collect::<Vec<_>>()
        .into()
}

fn item_to_match(item: &String) -> Match {
    Match {
        title: item.clone().into(),
        description: ROption::RSome("Plugin".into()),
        use_pango: false,
        icon: ROption::RSome(SystemIcon::Folder.as_str().into()),
        id: ROption::RNone,
    }
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    execute_action(selection.title.to_string());
    HandleResult::Close
}

fn execute_action(selection: String) {}
