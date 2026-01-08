mod category;

use abi_stable::std_types::ROption::{RNone, RSome};
use abi_stable::std_types::{RString, RVec};
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::fs::{self};
use std::path::PathBuf;
use std::process::Command;

use crate::category::InputCategory;

#[derive(Deserialize, Debug)]
struct Action {
    name: String,
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

    State { config }
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
        Some(stripped) => stripped.trim_start(),
        None => return RVec::new(),
    };

    let content = get_clipboard();
    let clip_type = InputCategory::detect(&content);

    let matcher = SkimMatcherV2::default();
    let query_lower = query_text.to_lowercase();

    let mut scored_matches: Vec<(i64, Match)> = state
        .config
        .actions
        .iter()
        .filter(|a| a.data_type == clip_type)
        .filter_map(|action| {
            let action_name_lower = action.name.to_lowercase();
            matcher
                .fuzzy_match(&action_name_lower, &query_lower)
                .map(|score| {
                    let m = Match {
                        title: action.name.clone().into(),
                        description: RSome(format!("Run action for {:?}", clip_type).into()),
                        icon: RSome(clip_type.get_icon().into()),
                        id: RNone,
                        use_pango: false,
                    };
                    (score, m)
                })
        })
        .take(state.config.max_entries)
        .collect();

    scored_matches.sort_by(|a, b| b.0.cmp(&a.0));

    scored_matches.into_iter().map(|(_, m)| m).collect()
}

fn get_clipboard() -> String {
    let output = Command::new("wl-paste")
        .arg("--type")
        .arg("text/plain")
        .arg("--no-newline")
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
        _ => String::new(),
    }
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    let name = selection.title.to_string();

    if let Some(action) = state.config.actions.iter().find(|a| a.name == name) {
        let content = get_clipboard();
        let cmd_script = action.command.replace("{clip}", &content);
        let _ = Command::new("sh")
            .arg("-c")
            .arg(cmd_script)
            .env("CLIP_CONTENT", content)
            .spawn();

        return HandleResult::Close;
    }

    HandleResult::Close
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let ron_str = r#"
            Config(
                show_log: true,
                prefix: ":ua ",
                actions: [
                    Action(
                        name: "Search Google",
                        command: "xdg-open 'https://google.com/search?q={clip}'",
                        data_type: Plaintext
                    )
                ]
            )
        "#;
        let config: Config = ron::from_str(ron_str).unwrap();
        assert_eq!(config.actions.len(), 1);
    }
}
