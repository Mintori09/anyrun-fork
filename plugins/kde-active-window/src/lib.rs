use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::focus_to_window_by_id;
use anyrun_helper::icon::SystemIcon;
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::{fs, path::PathBuf};

#[derive(Deserialize, Debug)]
#[serde(default)]
struct Config {
    prefix: String,
    max_entries: usize,
    cache_ttl_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "focus ".into(),
            max_entries: 10,
            cache_ttl_secs: 2,
        }
    }
}

pub struct State {
    config: Config,
    matcher: SkimMatcherV2,
    cache: Mutex<Option<(Instant, Vec<KdeWindow>)>>,
}

#[derive(Debug, Clone)]
pub struct KdeWindow {
    pub id: String,
    pub class: String,
}

fn get_kde_windows() -> Vec<KdeWindow> {
    let mut window_list = Vec::new();
    let output = Command::new("kdotool").arg("search").arg(".").output();

    let Ok(out) = output else { return window_list };
    if !out.status.success() {
        return window_list;
    };

    let stdout = String::from_utf8_lossy(&out.stdout);

    for id in stdout.lines() {
        let id = id.trim();
        if id.is_empty() {
            continue;
        }

        // Lấy Class Name
        let class_out = Command::new("kdotool")
            .arg("getwindowclassname")
            .arg(id)
            .output();
        let Ok(c_out) = class_out else { continue };
        if !c_out.status.success() {
            continue;
        };
        let class_name = String::from_utf8_lossy(&c_out.stdout).trim().to_string();

        window_list.push(KdeWindow {
            id: id.to_string(),
            class: class_name,
        });
    }
    window_list
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = PathBuf::from(config_dir.to_string()).join("window_switcher.ron");

    let config: Config = fs::read_to_string(config_path)
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_default();

    State {
        config,
        matcher: SkimMatcherV2::default().smart_case(),
        cache: Mutex::new(None),
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "KDE Window Switcher".into(),
        icon: SystemIcon::Settings.as_str().into(),
    }
}

fn get_windows_with_cache(state: &State) -> Vec<KdeWindow> {
    let mut cache = state.cache.lock().unwrap();

    if let Some((_, windows)) = cache
        .as_ref()
        .filter(|(inst, _)| inst.elapsed() < Duration::from_secs(state.config.cache_ttl_secs))
    {
        return windows.clone();
    }

    let windows = get_kde_windows();
    *cache = Some((Instant::now(), windows.clone()));
    windows
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let input_str = input.to_string();

    if let Some(query) = input_str.strip_prefix(&state.config.prefix) {
        let windows = get_windows_with_cache(state);

        let mut scored_matches: Vec<(i64, KdeWindow)> = windows
            .into_iter()
            .filter_map(|win| {
                if query.is_empty() {
                    return Some((0, win));
                }

                let score = state.matcher.fuzzy_match(&win.class, query.trim_end());
                score.map(|s| (s, win))
            })
            .collect();

        scored_matches.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        return scored_matches
            .into_iter()
            .take(state.config.max_entries)
            .map(|(_, win)| Match {
                title: win.class.clone().into(),
                description: ROption::RSome(win.id.clone().into()), // Hiện class ở dòng dưới
                id: ROption::RNone,                                 // Lưu ID vào metadata
                icon: ROption::RSome(win.class.into()),             // Thử lấy icon theo classname
                use_pango: false,
            })
            .collect::<Vec<_>>()
            .into();
    }

    RVec::new()
}

#[handler]
fn handler(selection: Match, _state: &State) -> HandleResult {
    if let ROption::RSome(window_id) = selection.description {
        focus_to_window_by_id(&window_id.to_string());
    }
    HandleResult::Close
}
