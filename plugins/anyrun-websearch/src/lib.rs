use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::{focus_to_class, icon::get_icon_path};
use anyrun_plugin::*;
use serde::Deserialize;
use std::{fs, process::Command};

#[derive(Deserialize)]
struct SearchEngine {
    name: String,
    prefix: String,
    url: String,
}

#[derive(Deserialize)]
struct Config {
    engines: Vec<SearchEngine>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            engines: vec![
                SearchEngine {
                    name: "Google".into(),
                    prefix: "gg ".into(),
                    url: "https://www.google.com/search?q={}".into(),
                },
                SearchEngine {
                    name: "Github".into(),
                    prefix: "gh ".into(),
                    url: "https://github.com/search?q={}".into(),
                },
            ],
        }
    }
}

#[init]
fn init(config_dir: RString) -> Config {
    match fs::read_to_string(format!("{}/websearchs.ron", config_dir)) {
        Ok(content) => ron::from_str(&content).unwrap_or_else(|_| Config::default()),
        Err(_) => Config::default(),
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Web Search".into(),
        icon: "network-wired".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, config: &Config) -> RVec<Match> {
    let mut matches = Vec::new();

    for engine in &config.engines {
        if input.starts_with(&engine.prefix) {
            let query = &input[engine.prefix.len()..];
            if !query.is_empty() {
                let full_url = engine.url.replace("{}", query);

                matches.push(Match {
                    title: format!("Search {} for: {}", engine.name, query).into(),
                    description: ROption::RSome(full_url.into()),
                    use_pango: false,
                    icon: ROption::RSome(get_icon_path(&engine.url).into()),
                    id: ROption::RNone,
                });
            }
        }
    }

    matches.into()
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    let url = selection.description.unwrap();

    if let Err(why) = Command::new("xdg-open").arg(url.as_str()).spawn() {
        eprintln!("[browser-search] Failed to open browser: {}", why);
    }

    focus_to_class("firefox");

    HandleResult::Close
}
