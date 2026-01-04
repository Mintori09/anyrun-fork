use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use serde::Deserialize;
use std::{fs, process::Command};

#[derive(Deserialize)]
struct Config {
    prefix: String,
}

impl Default for Config {
    fn default() -> Self {
        Config { prefix: "=".into() }
    }
}

#[init]
fn init(config_dir: RString) -> Config {
    match fs::read_to_string(format!("{}/calc.ron", config_dir)) {
        Ok(content) => ron::from_str(&content).unwrap_or_else(|_| Config::default()),
        Err(_) => Config::default(),
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Calc".into(),
        icon: "network-wired".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, config: &Config) -> RVec<Match> {
    let mut matches = Vec::new();

    if input.starts_with(&config.prefix) {
        let query = &input[config.prefix.len()..];
        if !query.is_empty() {
            matches.push(Match {
                title: calc(query).into(),
                description: ROption::RSome(query.into()),
                use_pango: false,
                icon: ROption::RNone,
                id: ROption::RNone,
            });
        }
    }

    matches.into()
}

fn calc(formula: &str) -> String {
    let output = Command::new("qalc")
        .arg("-t")
        .arg(formula)
        .output()
        .expect("[libqalc] Failed to run qalc!");

    let result = String::from_utf8_lossy(&output.stdout).to_string();

    result.trim().to_string()
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    let result = selection.title;

    if let Err(why) = Command::new("wl-copy").arg(result.as_str()).spawn() {
        eprintln!("[libqalc] Failed to copy: {}", why);
    }

    HandleResult::Close
}
