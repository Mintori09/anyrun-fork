use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::focus_to_class;
use anyrun_helper::icon::{SystemIcon, get_icon_path};
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::{env, fs, path::PathBuf};

#[derive(Deserialize, Debug)]
#[serde(default)]
struct Config {
    prefix: String,
    max_entries: usize,
    source: String,
    cache_ttl_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "tab ".into(),
            source: "~/.local/bin/brotab".into(),
            max_entries: 10,
            cache_ttl_secs: 5,
        }
    }
}

pub struct State {
    config: Config,
    full_path: String,
    matcher: SkimMatcherV2,
    cache: Mutex<Option<(Instant, Vec<Browser>)>>,
}

#[derive(Debug, Clone)]
pub struct Browser {
    title: String,
    url: String,
    id: String,
    id_numeric: u32,
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = PathBuf::from(config_dir.to_string()).join("browser.ron");

    let config: Config = fs::read_to_string(config_path)
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_default();

    let full_path = if config.source.starts_with('~') {
        let home = env::var("HOME").unwrap_or_default();
        config.source.replacen('~', &home, 1)
    } else {
        config.source.clone()
    };

    State {
        config,
        full_path,
        matcher: SkimMatcherV2::default().smart_case(),
        cache: Mutex::new(None),
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Browser Tabs".into(),
        icon: SystemIcon::WebBrowser.as_str().into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let input_str = input.to_string();

    if let Some(query) = input_str.strip_prefix(&state.config.prefix) {
        let query_parts: Vec<&str> = query.split_whitespace().collect();
        let tabs = get_tabs_with_cache(state);
        let mut scored_matches = get_scored_matches(state, tabs, query_parts);

        return scored_matches
            .drain(..std::cmp::min(scored_matches.len(), state.config.max_entries))
            .map(|browser| Match {
                title: browser.title.into(),
                description: ROption::RSome(browser.id.into()),
                id: ROption::RNone,
                icon: ROption::RSome(get_icon_path(&browser.url).into()),
                use_pango: false,
            })
            .collect::<Vec<_>>()
            .into();
    }

    RVec::new()
}

fn get_tabs_with_cache(state: &State) -> Vec<Browser> {
    let mut cache = state.cache.lock().unwrap();
    let now = Instant::now();

    if let Some((last_update, tabs)) = &*cache
        && now.duration_since(*last_update) < Duration::from_secs(state.config.cache_ttl_secs)
    {
        return tabs.clone();
    }

    let new_tabs = fetch_tab(&state.full_path);
    *cache = Some((now, new_tabs.clone()));
    new_tabs
}

fn fetch_tab(bin_path: &str) -> Vec<Browser> {
    let output = Command::new(bin_path).arg("list").output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .lines()
                .filter_map(|line| {
                    let mut parts = line.split('\t');
                    let id_str = parts.next()?;
                    let title = parts.next()?;
                    let url = parts.next()?;

                    let id_numeric = id_str
                        .split('.')
                        .next_back()
                        .and_then(|n| n.parse::<u32>().ok())
                        .unwrap_or(u32::MAX);

                    Some(Browser {
                        id: id_str.to_string(),
                        title: title.to_string(),
                        url: url.to_string(),
                        id_numeric,
                    })
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn get_scored_matches(state: &State, list: Vec<Browser>, query: Vec<&str>) -> Vec<Browser> {
    let mut scored: Vec<(i64, Browser)> = list
        .into_iter()
        .filter_map(|browser| {
            if query.is_empty() {
                return Some((0, browser));
            }

            let mut total_score = 0;
            for part in &query {
                let title_score = state.matcher.fuzzy_match(&browser.title, part);
                let url_score = state.matcher.fuzzy_match(&browser.url, part);

                match (title_score, url_score) {
                    (Some(s1), Some(s2)) => total_score += s1.max(s2),
                    (Some(s), None) | (None, Some(s)) => total_score += s,
                    (None, None) => return None,
                }
            }
            Some((total_score, browser))
        })
        .collect();

    scored.sort_unstable_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.id_numeric.cmp(&b.1.id_numeric))
    });

    scored.into_iter().map(|(_, b)| b).collect()
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    if let ROption::RSome(tab_id) = selection.description {
        focus_to_class("firefox");

        let _ = Command::new(&state.full_path)
            .arg("activate")
            .arg(tab_id.to_string())
            .spawn();
    }
    HandleResult::Close
}
