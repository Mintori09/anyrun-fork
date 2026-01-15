use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::{anyrun_interface::HandleResult, *};
use fuzzy_matcher::FuzzyMatcher;
use scrubber::DesktopEntry;
use serde::Deserialize;
use std::{collections::HashMap, env, fs, path::PathBuf, process::Command};

#[derive(Deserialize, Clone)]
pub struct Config {
    desktop_actions: bool,
    max_entries: usize,
    #[serde(default)]
    hide_description: bool,
    terminal: Option<Terminal>,
    preprocess_exec_script: Option<PathBuf>,
}

#[derive(Deserialize, Clone)]
pub struct Terminal {
    command: String,
    args: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            desktop_actions: false,
            max_entries: 5,
            hide_description: false,
            preprocess_exec_script: None,
            terminal: None,
        }
    }
}

/// Cấu trúc dữ liệu đã được tối ưu cho tìm kiếm
pub struct SearchableEntry {
    id: u64,
    name_lc: String,
    title_lc: String,
    desc_lc: Option<String>,
    keywords_lc: Vec<String>,
    offset: i64,
    is_action: bool,
}

pub struct State {
    config: Config,
    // Truy xuất O(1) khi người dùng nhấn Enter
    entry_map: HashMap<u64, DesktopEntry>,
    // Dữ liệu đã chuẩn hóa để search O(N) cực nhanh không tốn RAM
    search_entries: Vec<SearchableEntry>,
    // Lưu sẵn terminal hợp lệ để dùng ngay
    cached_terminal: Option<Terminal>,
}

mod scrubber;

#[handler]
pub fn handler(selection: Match, state: &State) -> HandleResult {
    let id = selection.id.unwrap();
    let entry = match state.entry_map.get(&id) {
        Some(e) => e,
        None => return HandleResult::Close,
    };

    // Tối ưu: Xử lý tiền thực thi (Pre-process)
    let exec = if let Some(script) = &state.config.preprocess_exec_script {
        let cmd_str = format!(
            "{} {} {}",
            script.display(),
            if entry.term { "term" } else { "no-term" },
            &entry.exec
        );
        Command::new("sh")
            .args(["-c", &cmd_str])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| entry.exec.clone())
    } else {
        entry.exec.clone()
    };

    if entry.term {
        if let Some(term) = &state.cached_terminal {
            let _ = Command::new("sh")
                .args([
                    "-c",
                    &format!("{} {}", term.command, term.args.replace("{}", &exec)),
                ])
                .spawn();
        }
    } else {
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let _ = Command::new("sh")
            .args(["-c", &entry.exec])
            .current_dir(
                entry
                    .path
                    .as_ref()
                    .filter(|p| p.exists())
                    .unwrap_or(&current_dir),
            )
            .spawn();
    }

    HandleResult::Close
}

#[init]
pub fn init(config_dir: RString) -> State {
    let config: Config = fs::read_to_string(format!("{}/applications.ron", config_dir))
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_default();

    let mut raw_entries = scrubber::scrubber(&config).unwrap_or_default();

    // Thêm các custom scripts
    let custom_actions = [
        ("Shutdown", "systemctl poweroff", "system-shutdown"),
        ("Reboot", "systemctl reboot", "system-reboot"),
        ("Lock Screen", "swaylock", "system-lock-screen"),
        ("Suspend", "systemctl suspend", "system-suspend"),
        ("Logout", "hyprctl dispatch exit", "system-log-out"),
    ];

    let mut next_id = raw_entries.iter().map(|(_, id)| *id).max().unwrap_or(0) + 1;
    for (name, exec, icon) in custom_actions {
        let entry = DesktopEntry {
            name: name.to_string(),
            exec: exec.to_string(),
            icon: icon.to_string(),
            localized_name: None, 
            desc: Some(format!("Execute {}", name)),
            term: false,
            keywords: vec!["system".to_string(), name.to_lowercase()],
            localized_keywords: None,
            is_action: true,
            offset: 0,
            path: None,
        };
        raw_entries.push((entry, next_id));
        next_id += 1;
    }

    // Tối ưu I/O: Tìm terminal khả dụng 1 lần duy nhất
    let cached_terminal = config.terminal.clone().or_else(|| {
        let candidates = [
            ("alacritty", "-e {}"),
            ("foot", "-e \"{}\""),
            ("kitty", "-e \"{}\""),
            ("wezterm", "-e \"{}\""),
            ("ghostty", "-e \"{}\""),
        ];
        candidates
            .iter()
            .find(|(cmd, _)| {
                Command::new("which")
                    .arg(cmd)
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            })
            .map(|(c, a)| Terminal {
                command: c.to_string(),
                args: a.to_string(),
            })
    });

    // Tối ưu bộ nhớ: Chuẩn bị sẵn dữ liệu tìm kiếm
    let mut search_entries = Vec::with_capacity(raw_entries.len());
    let mut entry_map = HashMap::with_capacity(raw_entries.len());

    for (entry, id) in raw_entries {
        search_entries.push(SearchableEntry {
            id,
            name_lc: entry.name.to_lowercase(),
            title_lc: entry.localized_name().to_lowercase(),
            desc_lc: entry.desc.as_ref().map(|d| d.to_lowercase()),
            keywords_lc: entry
                .keywords
                .iter()
                .map(|k| k.to_lowercase())
                .chain(
                    entry
                        .localized_keywords
                        .iter()
                        .flat_map(|ks| ks.iter().map(|k| k.to_lowercase())),
                )
                .collect(),
            offset: entry.offset,
            is_action: entry.is_action,
        });
        entry_map.insert(id, entry);
    }

    State {
        config,
        entry_map,
        search_entries,
        cached_terminal,
    }
}

#[get_matches]
pub fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let input_lc = input.to_lowercase();
    let input_trimmed = input_lc.trim();

    if input_trimmed.is_empty() {
        return state
            .search_entries
            .iter()
            .take(state.config.max_entries)
            .map(|se| {
                let entry = &state.entry_map[&se.id];
                make_match(entry, se.id, &state.config)
            })
            .collect();
    }

    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
    let tokens: Vec<&str> = input_trimmed.split_whitespace().collect();
    const ACTION_VERBS: &[&str] = &["quit", "close", "exit", "kill", "stop", "restart"];
    let has_action_verb = tokens.iter().any(|t| ACTION_VERBS.contains(t));

    let mut scored_results: Vec<(u64, i64)> = state
        .search_entries
        .iter()
        .filter_map(|se| {
            let mut score = 0;
            for token in &tokens {
                let s_title = matcher.fuzzy_match(&se.title_lc, token).unwrap_or(0);
                let s_name = matcher.fuzzy_match(&se.name_lc, token).unwrap_or(0);
                let s_desc = se
                    .desc_lc
                    .as_ref()
                    .and_then(|d| matcher.fuzzy_match(d, token))
                    .unwrap_or(0);
                let s_key = se
                    .keywords_lc
                    .iter()
                    .filter_map(|k| matcher.fuzzy_match(k, token))
                    .max()
                    .unwrap_or(0);

                let best = s_title.max(s_name).max(s_desc).max(s_key);
                if best == 0 {
                    return None;
                }

                score += s_title * 10 + s_name * 8 + s_desc * 5 + s_key * 3;
            }

            score -= se.offset;
            if se.is_action {
                score = if has_action_verb {
                    score * 3
                } else {
                    score / 2
                };
            }

            if score > 0 {
                Some((se.id, score))
            } else {
                None
            }
        })
        .collect();

    // Sắp xếp theo score giảm dần
    scored_results.sort_unstable_by(|a, b| b.1.cmp(&a.1));

    scored_results
        .into_iter()
        .take(state.config.max_entries)
        .map(|(id, _)| {
            let entry = &state.entry_map[&id];
            make_match(entry, id, &state.config)
        })
        .collect()
}

// Hàm helper để tránh lặp code và giảm clone
fn make_match(entry: &DesktopEntry, id: u64, config: &Config) -> Match {
    Match {
        title: entry.localized_name().into(),
        description: if config.hide_description {
            ROption::RNone
        } else {
            entry.desc.clone().map(|d| d.into()).into()
        },
        use_pango: false,
        icon: ROption::RSome(entry.icon.clone().into()),
        id: ROption::RSome(id),
    }
}

#[info]
pub fn info() -> PluginInfo {
    PluginInfo {
        name: "Applications".into(),
        icon: "application-x-executable".into(),
    }
}
