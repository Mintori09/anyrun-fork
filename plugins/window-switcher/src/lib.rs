mod backends;
mod config;

use std::sync::RwLock;
use std::{fs, path::PathBuf};

use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::{
    mazzy_matcher::fuzzy_match,
    window::{CachedBackend, WindowBackend, WindowInfo},
};
use anyrun_plugin::*;
use config::Config;

pub struct State {
    config: Config,
    backend: Box<dyn WindowBackend>,
    id_map: RwLock<Vec<String>>,
}

#[init]
fn init(config_dir: RString) -> Option<State> {
    let config_path = PathBuf::from(config_dir.to_string()).join("window_switcher.ron");
    let config: Config = fs::read_to_string(config_path)
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_default();

    let backend = match backends::detect_backend() {
        Some(be) => be,
        None => {
            eprintln!("[window-switcher] No supported compositor detected");
            return None;
        }
    };

    let cached = CachedBackend::new(backend, config.cache_ttl_secs);

    Some(State {
        config,
        backend: Box::new(cached),
        id_map: RwLock::new(Vec::new()),
    })
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Window Switcher".into(),
        icon: "preferences-system-windows".into(),
    }
}

fn score_window(win: &WindowInfo, query: &str) -> i64 {
    if query.is_empty() {
        return 1;
    }

    let mut score = 0i64;
    if let Some(ref name) = win.app_name {
        score += fuzzy_match(name, query).unwrap_or(0) * 3;
    }
    if let Some(ref id) = win.app_id {
        score += fuzzy_match(id, query).unwrap_or(0) * 2;
    }
    score += fuzzy_match(&win.title, query).unwrap_or(0);
    score
}

#[get_matches]
fn get_matches(input: RString, state: &Option<State>) -> RVec<Match> {
    let state = match state {
        Some(s) => s,
        None => return RVec::new(),
    };

    let input_str = input.to_string();
    let query = match input_str.strip_prefix(&state.config.prefix) {
        Some(q) => q.trim_start().to_string(),
        None => return RVec::new(),
    };

    let windows = state.backend.list_windows();

    let mut scored: Vec<(i64, &WindowInfo)> = windows
        .iter()
        .filter(|win| {
            !state.config.exclude_classes.iter().any(|excl| {
                win.app_id.as_deref() == Some(excl.as_str())
                    || win.app_name.as_deref() == Some(excl.as_str())
            })
        })
        .filter_map(|win| {
            let score = score_window(win, &query);
            if score > 0 { Some((score, win)) } else { None }
        })
        .collect();

    scored.sort_unstable_by_key(|b| std::cmp::Reverse(b.0));
    scored.truncate(state.config.max_entries);

    let mut id_map = Vec::with_capacity(scored.len());
    let matches: RVec<Match> = scored
        .into_iter()
        .map(|(_, win)| {
            let idx = id_map.len() as u64;
            id_map.push(win.id.clone());

            let display_name = win
                .app_name
                .clone()
                .or_else(|| win.app_id.clone())
                .unwrap_or_else(|| win.title.clone());

            let desc = if win.title.is_empty() {
                win.app_id.clone().unwrap_or_else(|| "Window".into())
            } else {
                let mut d = win.title.clone();
                if let Some(ref app_id) = win.app_id
                    && win.app_name.as_deref() != Some(app_id.as_str())
                {
                    d.push_str(" · ");
                    d.push_str(app_id);
                }
                if let Some(ref ws) = win.workspace {
                    d.push_str(" · ");
                    d.push_str(ws);
                }
                d
            };

            Match {
                title: display_name.into(),
                description: ROption::RSome(desc.into()),
                use_pango: false,
                icon: win.icon.clone().map(|i| i.into()).into(),
                id: ROption::RSome(idx),
            }
        })
        .collect();

    *state.id_map.write().unwrap() = id_map;
    matches
}

#[handler]
fn handler(selection: Match, state: &Option<State>) -> HandleResult {
    let state = match state {
        Some(s) => s,
        None => return HandleResult::Close,
    };

    if let ROption::RSome(idx) = selection.id {
        let id_map = state.id_map.read().unwrap();
        if let Some(window_id) = id_map.get(idx as usize)
            && let Err(e) = state.backend.focus_window(window_id)
        {
            eprintln!("[window-switcher] Failed to focus window: {e}");
        }
    }
    HandleResult::Close
}
