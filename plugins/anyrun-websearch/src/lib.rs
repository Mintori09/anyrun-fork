use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::focus_to_class;
use anyrun_helper::icon::get_icon_path;
use anyrun_plugin::*;
use serde::Deserialize;
use std::{fs, process::Command, thread, time::Duration};

#[derive(Deserialize)]
struct SearchEngine {
    name: String,
    prefix: String,
    url: String,
}

#[derive(Deserialize)]
#[serde(default)]
struct Config {
    engines: Vec<SearchEngine>,
    focus_class: Option<String>,
    focus_delay_ms: u64,
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
            focus_class: None,
            focus_delay_ms: 120,
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
fn handler(selection: Match, config: &Config) -> HandleResult {
    let url = selection.description.unwrap();

    if let Err(why) = anyrun_plugin::spawn_detached(Command::new("xdg-open").arg(url.as_str())) {
        eprintln!("[browser-search] Failed to open browser: {}", why);
    }

    if let Some(class) = config.focus_class.as_deref().map(str::trim)
        && !class.is_empty()
    {
        thread::sleep(Duration::from_millis(config.focus_delay_ms));
        focus_to_class(class);
    }

    HandleResult::Close
}
