mod category;

use abi_stable::std_types::ROption::{RNone, RSome};
use abi_stable::std_types::{RString, RVec};
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::category::InputCategory;

#[derive(Deserialize, Debug)]
struct Action {
    name: String,
    command: String,
    data_type: InputCategory,
}

struct OptimizedAction {
    original_name: String,
    lowercase_name: String,
    command: String,
    data_type: InputCategory,
}

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(default = "default_prefix")]
    prefix: String,
    actions: Vec<Action>,
    #[serde(default = "default_max_entries")]
    max_entries: usize,
}

fn default_prefix() -> String {
    ":ua ".into()
}
fn default_max_entries() -> usize {
    5
}

pub struct State {
    config: Config,
    optimized_actions: Vec<OptimizedAction>,
    matcher: SkimMatcherV2,
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = PathBuf::from(config_dir.to_string()).join("universal-action.ron");

    let config: Config = fs::read_to_string(&config_path)
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_else(|| Config {
            prefix: default_prefix(),
            actions: Vec::new(),
            max_entries: 5,
        });

    let optimized_actions = config
        .actions
        .iter()
        .map(|a| OptimizedAction {
            original_name: a.name.clone(),
            lowercase_name: a.name.to_lowercase(),
            command: a.command.clone(),
            data_type: a.data_type,
        })
        .collect();

    State {
        config,
        optimized_actions,
        matcher: SkimMatcherV2::default(),
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Universal Action".into(),
        icon: "edit-paste-symbolic".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let query_text = match input.strip_prefix(&state.config.prefix) {
        Some(stripped) if !stripped.is_empty() => stripped.trim_start(),
        _ => return RVec::new(),
    };

    let content = get_clipboard();
    let clip_type = InputCategory::detect(&content);
    let query_lower = query_text.to_lowercase();

    let mut scored_matches: Vec<(i64, Match)> = state
        .optimized_actions
        .iter()
        .filter(|a| a.data_type == clip_type || a.data_type == InputCategory::All)
        .filter_map(|action| {
            state
                .matcher
                .fuzzy_match(&action.lowercase_name, &query_lower)
                .map(|score| {
                    (
                        score,
                        Match {
                            title: action.original_name.clone().into(),
                            description: RSome(format!("Run action for {:?}", clip_type).into()),
                            icon: RSome(clip_type.get_icon().into()),
                            id: RNone,
                            use_pango: false,
                        },
                    )
                })
        })
        .collect();

    scored_matches.sort_unstable_by(|a, b| b.0.cmp(&a.0));

    scored_matches
        .into_iter()
        .take(state.config.max_entries)
        .map(|(_, m)| m)
        .collect()
}

fn get_clipboard() -> String {
    Command::new("wl-paste")
        .args(["--type", "text/plain", "--no-newline"])
        .output()
        .map(|out| {
            if out.status.success() {
                String::from_utf8_lossy(&out.stdout).into_owned()
            } else {
                String::new()
            }
        })
        .unwrap_or_default()
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    let name = selection.title.to_string();

    if let Some(action) = state
        .optimized_actions
        .iter()
        .find(|a| a.original_name == name)
    {
        let content = get_clipboard();
        let cmd_script = action.command.replace("{clip}", &content);

        let _ = Command::new("sh").arg("-c").arg(cmd_script).spawn();
    }

    HandleResult::Close
}
