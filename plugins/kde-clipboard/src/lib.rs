use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use serde::Deserialize;
use std::fs;
use std::time::{Duration, Instant};
use zbus::blocking::Connection;
use zbus::proxy;

#[proxy(
    interface = "org.kde.klipper.klipper",
    default_service = "org.kde.klipper",
    default_path = "/klipper",
    blocking_name = "KlipperProxy",
    async_name = "KlipperProxyAsync"
)]
trait Klipper {
    #[zbus(name = "getClipboardHistoryMenu")]
    fn get_clipboard_history_menu(&self) -> zbus::Result<Vec<String>>;
    #[zbus(name = "setClipboardContents")]
    fn set_clipboard_contents(&self, data: &str) -> zbus::Result<()>;
}

#[derive(Deserialize)]
struct Config {
    #[serde(default = "default_prefix")]
    prefix: String,
    #[serde(default = "default_max_entries")]
    max_entries: usize,
}

fn default_prefix() -> String {
    "hf ".into()
}
fn default_max_entries() -> usize {
    10
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: default_prefix(),
            max_entries: default_max_entries(),
        }
    }
}

use std::sync::RwLock;

pub struct State {
    config: Config,
    connection: Connection,
    cached_history: RwLock<(Instant, Vec<String>)>,
}

#[init]
fn init(config_dir: RString) -> State {
    let config: Config = fs::read_to_string(format!("{}/klipper.ron", config_dir))
        .map(|content| ron::from_str(&content).unwrap_or_default())
        .unwrap_or_default();

    let connection = Connection::session().expect("Failed to connect to D-Bus");

    let cached_history =
        RwLock::new((Instant::now() - Duration::from_secs(60), Vec::new()));

    State {
        config,
        connection,
        cached_history,
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "KDE Klipper".into(),
        icon: "klipper".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let input_str = input.as_str();
    let query = match input_str.strip_prefix(&state.config.prefix) {
        Some(q) => q.trim(),
        None => return RVec::new(),
    };

    let needs_update = {
        let cache = state.cached_history.read().unwrap();
        cache.0.elapsed() > Duration::from_millis(1000)
    };

    if needs_update {
        let new_data = KlipperProxy::new(&state.connection)
            .ok()
            .and_then(|proxy| proxy.get_clipboard_history_menu().ok());

        if let Some(data) = new_data {
            let mut cache = state.cached_history.write().unwrap();
            *cache = (Instant::now(), data);
        }
    }

    let history = {
        let cache = state.cached_history.read().unwrap();
        cache.1.clone()
    };

    let mut results: Vec<(i64, String)> = history
        .into_iter()
        .filter_map(|item| {
            if item.trim().is_empty() {
                return None;
            }

            let clean_item = if item.contains('&') {
                item.replace('&', "")
            } else {
                item
            };

            if query.is_empty() {
                Some((0, clean_item))
            } else {
                anyrun_helper::mazzy_matcher::fuzzy_match(&clean_item, query)
                    .map(|score| (score, clean_item))
            }
        })
        .collect();

    if !query.is_empty() {
        results.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    }

    results
        .into_iter()
        .take(state.config.max_entries)
        .map(|(_, text)| Match {
            title: text.into(),
            description: ROption::RSome("Copy to clipboard".into()),
            use_pango: false,
            icon: ROption::RSome("edit-copy".into()),
            id: ROption::RNone,
        })
        .collect::<Vec<_>>()
        .into()
}

#[handler]
fn handler(selection: Match, _state: &State) -> HandleResult {
    let result = selection.title;

    if let Err(why) = std::process::Command::new("wl-copy")
        .arg(result.as_str())
        .spawn()
    {
        eprintln!("[libklipper] Failed to copy: {}", why);
    }

    HandleResult::Close
}
