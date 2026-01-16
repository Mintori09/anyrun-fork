mod actions;
mod category;
mod registry;
mod validate;

use abi_stable::std_types::ROption::{RNone, RSome};
use abi_stable::std_types::{RString, RVec};
use anyrun_helper::get_clipboard;
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::fs::{self};
use std::path::PathBuf;

use crate::actions::UniversalAction;
use crate::category::InputCategory;
use crate::registry::get_internal_actions;

#[derive(Deserialize, Debug)]
struct Action {
    name: String,
    command: String,
    data_type: InputCategory,
}

#[derive(Deserialize, Debug, Default)]
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
    actions: Vec<UniversalAction>,
    matcher: SkimMatcherV2,
    clipboard: String,
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = PathBuf::from(config_dir.to_string()).join("universal_action.ron");

    let config: Config = fs::read_to_string(&config_path)
        .map_err(|e| format!("IO Error: {}", e))
        .and_then(|content| ron::from_str(&content).map_err(|e| format!("RON Error: {}", e)))
        .unwrap_or_else(|_err| Config {
            prefix: default_prefix(),
            actions: Vec::new(),
            max_entries: 5,
        });

    let actions: Vec<UniversalAction> = config
        .actions
        .iter()
        .map(|a| UniversalAction {
            name: a.name.clone(),
            name_lowercase: a.name.to_lowercase(),
            target: actions::ActionTarget::Shell(a.command.clone()),
            category: a.data_type,
            validator: None,
        })
        .chain(get_internal_actions())
        .collect();

    State {
        clipboard: get_clipboard(),
        config,
        actions,
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
        Some(stripped) => stripped,
        _ => return RVec::new(),
    };

    let clip_type = InputCategory::detect(&state.clipboard);
    let query_trimmed = query_text.trim();
    let is_empty_query = query_trimmed.is_empty();

    let common_icon = RSome(clip_type.get_icon().into());
    let common_desc = RSome(format!("Run action for {:?}", clip_type).into());

    let limit = if is_empty_query {
        10
    } else {
        state.config.max_entries
    };

    let mut scores: Vec<(i64, &UniversalAction)> = state
        .actions
        .iter()
        .filter(|a| a.is_match(&state.clipboard, clip_type) || a.category == InputCategory::All)
        .filter_map(|action| {
            if is_empty_query {
                Some((0, action))
            } else {
                state
                    .matcher
                    .fuzzy_match(&action.name_lowercase, query_trimmed)
                    .map(|score| (score, action))
            }
        })
        .collect();

    if !is_empty_query {
        scores.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    }

    scores
        .into_iter()
        .take(limit)
        .map(|(_, action)| Match {
            title: action.name.clone().into(),
            description: common_desc.clone(),
            icon: common_icon.clone(),
            id: RNone,
            use_pango: false,
        })
        .collect()
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    if let Some(action) = state
        .actions
        .iter()
        .find(|a| a.name == selection.title.to_string())
    {
        action.target.run_action(&state.clipboard);
    }

    HandleResult::Close
}

#[test]
fn check_url() {
    let url = "https://www.youtube.com/watch?v=CRLEfo_4X0M";
    let input = InputCategory::detect(url);
    assert_eq!(input, InputCategory::Url);
}
