use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use serde::Deserialize;
use std::fs;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
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
    #[serde(default = "default_preview_max_chars")]
    preview_max_chars: usize,
}

fn default_prefix() -> String {
    "hf ".into()
}
fn default_max_entries() -> usize {
    10
}
fn default_preview_max_chars() -> usize {
    120
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: default_prefix(),
            max_entries: default_max_entries(),
            preview_max_chars: default_preview_max_chars(),
        }
    }
}

pub struct State {
    config: Config,
    connection: Connection,
    cached_history: RwLock<(Instant, Vec<String>)>,
    selection_map: RwLock<std::collections::HashMap<u64, String>>,
    next_selection_id: AtomicU64,
}

#[init]
fn init(config_dir: RString) -> State {
    let config: Config = fs::read_to_string(format!("{}/klipper.ron", config_dir))
        .map(|content| ron::from_str(&content).unwrap_or_default())
        .unwrap_or_default();

    let connection = Connection::session().expect("Failed to connect to D-Bus");

    let cached_history = RwLock::new((Instant::now() - Duration::from_secs(60), Vec::new()));

    State {
        config,
        connection,
        cached_history,
        selection_map: RwLock::new(std::collections::HashMap::new()),
        next_selection_id: AtomicU64::new(1),
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "KDE Klipper".into(),
        icon: "klipper".into(),
    }
}

fn format_preview(raw: &str, max_chars: usize) -> String {
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");

    if max_chars == 0 {
        return "...".into();
    }

    let mut preview = String::new();
    let mut count = 0usize;
    for ch in normalized.chars() {
        if count >= max_chars {
            break;
        }
        preview.push(ch);
        count += 1;
    }

    if normalized.chars().count() > max_chars {
        preview.push_str("...");
    }

    preview
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
        results.sort_unstable_by_key(|b| std::cmp::Reverse(b.0));
    }

    let mut selection_pairs = Vec::new();
    let matches = results
        .into_iter()
        .take(state.config.max_entries)
        .map(|(_, text)| {
            let id = state.next_selection_id.fetch_add(1, Ordering::Relaxed);
            let preview = format_preview(&text, state.config.preview_max_chars);
            selection_pairs.push((id, text));

            Match {
                title: preview.into(),
                description: ROption::RSome("Copy to clipboard".into()),
                use_pango: false,
                icon: ROption::RSome("edit-copy".into()),
                id: ROption::RSome(id),
            }
        })
        .collect::<Vec<_>>();

    {
        let mut map = state.selection_map.write().unwrap();
        map.clear();
        map.extend(selection_pairs);
    }

    matches.into()
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    let result = match selection.id {
        ROption::RSome(id) => state
            .selection_map
            .read()
            .unwrap()
            .get(&id)
            .cloned()
            .unwrap_or_else(|| selection.title.to_string()),
        ROption::RNone => selection.title.to_string(),
    };

    if let Err(why) = std::process::Command::new("wl-copy").arg(result).spawn() {
        eprintln!("[libklipper] Failed to copy: {}", why);
    }

    HandleResult::Close
}

#[cfg(test)]
mod tests {
    use super::format_preview;

    #[test]
    fn keeps_short_single_line() {
        assert_eq!(format_preview("hello world", 120), "hello world");
    }

    #[test]
    fn truncates_long_single_line() {
        assert_eq!(
            format_preview("abcdefghijklmnopqrstuvwxyz", 10),
            "abcdefghij..."
        );
    }

    #[test]
    fn collapses_multiline_then_truncates() {
        assert_eq!(format_preview("line1\nline2\t line3", 10), "line1 line...");
    }
}
