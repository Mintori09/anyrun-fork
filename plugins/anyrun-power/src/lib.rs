use abi_stable::std_types::ROption::{RNone, RSome};
use abi_stable::std_types::{RString, RVec};
use anyrun_helper::icon::SystemIcon;
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{self};
use std::path::PathBuf;
use std::process::Command;

#[derive(Deserialize, Debug)]
struct Action {
    name: String,
    command: String,
    icon: SystemIcon,
}

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(default)]
    prefix: String,
    max_entries: usize,
}

pub struct State {
    config: Config,
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = PathBuf::from(config_dir.to_string()).join("system_power.ron");

    let config: Config = fs::read_to_string(&config_path)
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_else(|| Config {
            prefix: "pow ".into(),
            max_entries: 5,
        });

    State { config }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "System".into(),
        icon: SystemIcon::Settings.as_str().into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let query_text = match input.strip_prefix(&state.config.prefix) {
        Some(stripped) => stripped.trim_start(),
        None => return RVec::new(),
    };

    let mut actions: Vec<Action> = Vec::new();

    let mut scripts = HashMap::new();
    scripts.insert("Shutdown", "systemctl poweroff");
    scripts.insert("Reboot", "systemctl reboot");
    scripts.insert("Lock Screen", "swaylock");
    scripts.insert("Suspend", "systemctl suspend");
    scripts.insert("Logout", "hyprctl dispatch exit");

    for (name, command) in scripts {
        let icon = match name {
            "Shutdown" => SystemIcon::SystemRun,
            "Reboot" => SystemIcon::ViewRefresh,
            "Lock Screen" => SystemIcon::UserPassword,
            "Suspend" => SystemIcon::Battery,
            "Logout" => SystemIcon::GoBack,
            _ => SystemIcon::Settings,
        };

        actions.push(Action {
            name: name.to_string(),
            command: command.to_string(),
            icon,
        });
    }

    let matcher = SkimMatcherV2::default();
    let query_lower = query_text.to_lowercase();

    let mut scored_matches: Vec<(i64, Match)> = actions
        .iter()
        .filter_map(|action| {
            let action_name_lower = action.name.to_lowercase();
            matcher
                .fuzzy_match(&action_name_lower, &query_lower)
                .map(|score| {
                    let m = Match {
                        title: action.name.clone().into(),
                        description: RSome(action.command.clone().into()),
                        icon: RSome(action.icon.as_str().into()),
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

#[handler]
fn handler(selection: Match, _state: &State) -> HandleResult {
    if let RSome(action) = selection.description {
        let _ = Command::new("sh").arg("-c").arg(action.to_string()).spawn();

        return HandleResult::Close;
    }

    HandleResult::Close
}

#[cfg(test)]
mod tests {}
