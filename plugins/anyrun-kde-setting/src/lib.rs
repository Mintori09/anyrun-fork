use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::{fs, process::Command};

#[derive(Deserialize, Clone)]
pub struct KDESetting {
    pub name: String,
    pub description: String,
}

#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    prefix: String,
    #[serde(default)]
    custom_settings: Vec<KDESetting>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "set".into(),
            custom_settings: Vec::new(),
        }
    }
}

pub struct State {
    config: Config,
    all_settings: Vec<KDESetting>,
}

#[init]
fn init(config_dir: RString) -> State {
    let config: Config = fs::read_to_string(format!("{}/kde_setting.ron", config_dir))
        .map(|content| ron::from_str(&content).unwrap_or_default())
        .unwrap_or_default();

    let mut settings = get_kde_settings();

    settings.extend(config.custom_settings.clone());

    State {
        config,
        all_settings: settings,
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "KDE Settings".into(),
        icon: "preferences-system".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let input_str = input.as_str();
    let prefix = &state.config.prefix;

    if input_str.starts_with(prefix) {
        let query = input_str[prefix.len()..].trim();

        if query.is_empty() {
            return RVec::new();
        }

        let results = search_settings(state.all_settings.clone(), query);

        results
            .into_iter()
            .map(|s| Match {
                title: s.name.into(),
                description: ROption::RSome(s.description.into()),
                use_pango: false,
                icon: ROption::RSome("preferences-system".into()),
                id: ROption::RNone,
            })
            .collect::<Vec<_>>()
            .into()
    } else {
        RVec::new()
    }
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    if let ROption::RSome(id) = selection.description {
        let _ = Command::new("kcmshell6")
            .arg(format!("kcm_{}", id.to_string()))
            .spawn();
    }
    HandleResult::Close
}

fn get_kde_settings() -> Vec<KDESetting> {
    let output = Command::new("kcmshell6").arg("--list").output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .lines()
                .skip(1)
                .filter_map(|line| {
                    line.rsplit_once('-').map(|(id_part, name_part)| {
                        let id = id_part.trim().to_string();
                        let name_from_list = name_part.trim().to_string();

                        let display_name = id.replace("kcm_", "").replace('_', " ");

                        KDESetting {
                            name: name_from_list,
                            description: display_name,
                        }
                    })
                })
                .collect()
        }
        Err(_) => Vec::new(),
    }
}

pub fn search_settings(settings: Vec<KDESetting>, query: &str) -> Vec<KDESetting> {
    let matcher = SkimMatcherV2::default();

    let mut ranked_results: Vec<(i64, KDESetting)> = settings
        .into_iter()
        .filter_map(|setting| {
            let target_text = format!("{} {}", setting.name, setting.description);
            matcher
                .fuzzy_match(&target_text, query)
                .map(|score| (score, setting))
        })
        .collect();

    ranked_results.sort_by(|a, b| b.0.cmp(&a.0));

    ranked_results
        .into_iter()
        .map(|(_, setting)| setting)
        .collect()
}
