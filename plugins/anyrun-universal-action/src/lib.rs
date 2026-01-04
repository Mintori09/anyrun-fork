use abi_stable::std_types::ROption::RNone;
use abi_stable::std_types::{RString, RVec};
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize, Debug)]
struct Action {
    name: String,
    command: String,
    data_type: String,
}

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(default)]
    show_log: bool,
    prefix: String,
    actions: Vec<Action>,
}

pub struct State {
    config: Config,
}

fn get_type(content: &str) -> &str {
    if content.starts_with("http://") || content.starts_with("https://") {
        "Url"
    } else if Path::new(content).exists() || content.starts_with("file:///") {
        "File"
    } else if content.trim().is_empty() {
        "Nothing"
    } else {
        "Text"
    }
}

fn logger(msg: &str, state: &State) {
    if !state.config.show_log {
        return;
    }

    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let log_path = PathBuf::from(home).join("Desktop/universal-action.log");

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = PathBuf::from(config_dir.to_string()).join("universal-action.ron");

    let config: Config = fs::read_to_string(config_path)
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_else(|| Config {
            show_log: false,
            prefix: ":ua ".into(),
            actions: Vec::new(),
        });

    let state = State { config };
    logger("Plugin initialized", &state);
    state
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Universal Action".into(),
        icon: "edit-paste-symbolic".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    if !input.starts_with(&state.config.prefix) {
        return RVec::new();
    }

    let content = get_clipboard();
    let clip_type = get_type(&content);
    logger(&content, state);
    logger(clip_type, state);

    let query = &input[state.config.prefix.len()..];
    let matcher = SkimMatcherV2::default();

    let mut matches: RVec<_> = state
        .config
        .actions
        .iter()
        .filter(|a| a.data_type == clip_type || a.data_type == "Any")
        .filter_map(|action| {
            matcher
                .fuzzy_match(&action.name.to_lowercase(), &query.to_lowercase())
                .map(|score| {
                    let m = Match {
                        title: action.name.clone().into(),
                        description: RNone,
                        icon: RNone,
                        id: RNone,
                        use_pango: false,
                    };
                    (score, m)
                })
        })
        .collect();

    matches.sort_by(|a, b| b.0.cmp(&a.0));
    matches.into_iter().map(|(_, m)| m).collect()
}

fn get_clipboard() -> String {
    let output = Command::new("wl-paste")
        .arg("--type")
        .arg("text/plain")
        .arg("--no-newline")
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => String::new(),
    }
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    let name = &selection.title.to_string();
    for action in &state.config.actions {
        if name == &action.name {
            let content = get_clipboard();
            let cmd: String = action.command.replace("{clip}", &content);

            let _ = Command::new("sh").arg("-c").arg(cmd).spawn();
            return HandleResult::Close;
        }
    }

    HandleResult::Close
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs, path::PathBuf};

    #[test]
    fn test_read_real_config() {
        let home_dir = env::var("HOME").expect("Không tìm thấy biến môi trường HOME");

        let config_path = PathBuf::from(home_dir)
            .join(".config")
            .join("anyrun")
            .join("universal-action.ron");

        assert!(
            config_path.exists(),
            "File cấu hình không tồn tại tại: {:?}",
            config_path
        );

        let content = fs::read_to_string(&config_path).expect("Không thể đọc file cấu hình");

        let result: Result<Config, _> = ron::from_str(&content);

        match result {
            Ok(config) => {
                println!("Đọc config thành công! Số lượng scopes: {:?}", config);
            }
            Err(e) => panic!("File RON tồn tại nhưng sai cấu trúc: {}", e),
        }
    }
}
