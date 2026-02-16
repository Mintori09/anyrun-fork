mod action;
mod helper;
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

use crate::action::model::{ActionTarget, InputCategory, UniversalAction};
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
    filetype: InputCategory,
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

    let mut actions = get_internal_actions();
    let config_actions: Vec<UniversalAction> = config
        .actions
        .iter()
        .map(|a| UniversalAction {
            name: a.name.clone(),
            name_lowercase: a.name.to_lowercase(),
            target: ActionTarget::Shell(a.command.clone()),
            category: a.data_type.clone(),
            validator: None,
        })
        .collect();

    actions.extend(config_actions);

    State {
        filetype: InputCategory::classify_clipboard(),
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
    let query = match extract_query(&input, &state.config.prefix) {
        Some(q) => q,
        None => return RVec::new(),
    };

    let is_initial_view = query.is_empty();
    let limit = if is_initial_view {
        10
    } else {
        state.config.max_entries
    };

    let mut ranked_actions = filter_and_score_actions(state, query, is_initial_view);

    if !is_initial_view {
        ranked_actions.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    }

    create_matches(ranked_actions, state, limit)
}

fn extract_query<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    let stripped = input.strip_prefix(prefix)?;
    if stripped.is_empty() {
        Some(stripped)
    } else {
        Some(stripped.trim_start())
    }
}

fn filter_and_score_actions<'a>(
    state: &'a State,
    query: &str,
    is_initial_view: bool,
) -> Vec<(i64, &'a UniversalAction)> {
    state
        .actions
        .iter()
        .filter(|action| action.is_match(&state.clipboard, state.filetype.clone()))
        .filter_map(|action| {
            if is_initial_view {
                Some((0, action))
            } else {
                state
                    .matcher
                    .fuzzy_match(&action.name_lowercase, query)
                    .map(|score| (score, action))
            }
        })
        .collect()
}

fn create_matches(
    ranked_actions: Vec<(i64, &UniversalAction)>,
    state: &State,
    limit: usize,
) -> RVec<Match> {
    let icon = RSome(state.filetype.get_icon().as_str().into());
    let description = RSome(format!("Run action for {:?}", state.filetype).into());

    ranked_actions
        .into_iter()
        .take(limit)
        .map(|(_, action)| Match {
            title: action.name.clone().into(),
            description: description.clone(),
            icon: icon.clone(),
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
        action
            .target
            .run_action(state.filetype.clone(), &state.clipboard);
    }

    HandleResult::Close
}

#[test]
fn test_read_config() {
    let mut config_path: PathBuf =
        std::env::home_dir().expect("Could not determine home directory");
    config_path.push(".config");
    config_path.push("anyrun");
    config_path.push("universal_action.ron");

    let config: Config = fs::read_to_string(&config_path)
        .map_err(|e| format!("IO Error: {}", e))
        .and_then(|content| ron::from_str(&content).map_err(|e| format!("RON Error: {}", e)))
        .unwrap_or_else(|_err| Config {
            prefix: default_prefix(),
            actions: Vec::new(),
            max_entries: 5,
        });

    let mut actions = get_internal_actions();
    let config_actions: Vec<UniversalAction> = config
        .actions
        .iter()
        .map(|a| UniversalAction {
            name: a.name.clone(),
            name_lowercase: a.name.to_lowercase(),
            target: ActionTarget::Shell(a.command.clone()),
            category: a.data_type.clone(),
            validator: None,
        })
        .collect();

    actions.extend(config_actions);

    let state = State {
        filetype: InputCategory::classify_clipboard(),
        clipboard: get_clipboard(),
        config,
        actions,
        matcher: SkimMatcherV2::default(),
    };

    println!("{:?}", state.config);
}
