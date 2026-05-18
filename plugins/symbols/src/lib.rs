use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Clone, Debug)]
struct Symbol {
    chr: String,
    name: String,
}

#[derive(Deserialize, Debug)]
struct Config {
    prefix: String,
    #[serde(default)]
    symbols: HashMap<String, String>,
    #[serde(default = "default_max_entries")]
    max_entries: usize,
}

fn default_max_entries() -> usize {
    10
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "icon".to_string(),
            symbols: HashMap::new(),
            max_entries: 5,
        }
    }
}

pub struct State {
    config: Config,
    symbols: Vec<Symbol>,
}

#[init]
fn init(config_dir: RString) -> State {
    let config: Config = fs::read_to_string(format!("{}/symbols.ron", config_dir))
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_default();

    let mut symbols = Vec::new();

    // 1. Add icons from icons.ron
    let icons_str = include_str!("../res/icons.ron");
    if let Ok(icons) = ron::from_str::<HashMap<String, String>>(icons_str) {
        for (name, chr) in icons {
            symbols.push(Symbol { name, chr });
        }
    }

    // 2. Add unicode characters from UnicodeData.txt
    let unicode_data = include_str!("../res/UnicodeData.txt");
    for line in unicode_data.lines() {
        let mut fields = line.split(';');
        let hex = fields.next().unwrap_or_default();
        let name = fields.next().unwrap_or_default();

        if !name.is_empty() && name != "<control>" {
            if let Ok(code) = u32::from_str_radix(hex, 16) {
                if let Some(chr) = std::char::from_u32(code) {
                    symbols.push(Symbol {
                        name: name.to_string(),
                        chr: chr.to_string(),
                    });
                }
            }
        }
    }

    // 3. Add custom symbols from config
    for (name, chr) in config.symbols.clone() {
        symbols.push(Symbol { name, chr });
    }

    State { config, symbols }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Symbols".into(),
        icon: "accessories-character-map".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let input = if state.config.prefix.is_empty() {
        input.as_str()
    } else if let Some(stripped) = input.strip_prefix(&state.config.prefix) {
        stripped.trim()
    } else {
        return RVec::new();
    };

    if input.is_empty() {
        return RVec::new();
    }

    let mut matches: Vec<(&Symbol, i64)> = state
        .symbols
        .iter()
        .filter_map(|s| {
            anyrun_helper::mazzy_matcher::fuzzy_match(&s.name, input).map(|score| (s, score))
        })
        .collect();

    matches.sort_by_key(|b| std::cmp::Reverse(b.1));

    matches
        .into_iter()
        .take(state.config.max_entries)
        .map(|(s, _)| Match {
            title: s.chr.clone().into(),
            description: ROption::RSome(s.name.clone().into()),
            use_pango: false,
            icon: ROption::RNone,
            id: ROption::RNone,
        })
        .collect()
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    HandleResult::Copy(selection.title.into_bytes())
}
