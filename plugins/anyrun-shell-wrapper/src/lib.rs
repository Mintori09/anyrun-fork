use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::icon::SystemIcon;
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;
use std::{env, fs, path::PathBuf};

#[derive(Deserialize, Debug)]
struct Scope {
    prefix: String,
    source: String,
    on_select: String,
}

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(default)]
    max_entries: usize,
    show_log: bool,
    scopes: Vec<Scope>,
}

pub struct State {
    config: Config,
}

fn logger(msg: &str, state: &State) {
    if !state.config.show_log {
        return;
    }

    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let log_path = PathBuf::from(home).join("Desktop/shell_wrapper.log");

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = PathBuf::from(config_dir.to_string()).join("shell_wrapper.ron");

    let config: Config = fs::read_to_string(config_path)
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_else(|| Config {
            show_log: false,
            scopes: Vec::new(),
            max_entries: 10,
        });

    let state = State { config };
    logger("Plugin initialized", &state);
    state
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Shell Wrapper".into(),
        icon: "utilities-terminal-symbolic".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let input_str = input.to_string().to_lowercase();

    for scope in &state.config.scopes {
        if let Some(query) = input_str.strip_prefix(&scope.prefix) {
            let query_str = &input_str[scope.prefix.len()..];
            let query_parts: Vec<&str> = query_str.split_whitespace().collect();
            logger(
                &format!("Searching with prefix: {} | Query: {}", scope.prefix, query),
                state,
            );

            let output = get_list_output(&scope.source);
            let matches = get_matches_fuzzy_finder(output, query_parts);

            return matches
                .into_iter()
                .take(state.config.max_entries)
                .map(|line| Match {
                    title: line.trim().into(),
                    description: ROption::RSome(format!("Execute via {}", scope.prefix).into()),
                    id: ROption::RNone,
                    icon: ROption::RSome(SystemIcon::from_ext(&line).as_str().into()),
                    use_pango: false,
                })
                .collect::<Vec<_>>()
                .into();
        }
    }

    RVec::new()
}

fn get_list_output(source: &str) -> Vec<String> {
    let output = Command::new("sh").arg("-c").arg(source).output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        return stdout
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
    }

    Vec::new()
}

fn get_matches_fuzzy_finder(list: Vec<String>, query: Vec<&str>) -> Vec<String> {
    let matcher = SkimMatcherV2::default();
    let mut matches: Vec<(i64, String)> = list
        .into_iter()
        .filter_map(|path| {
            let mut total_score = 0;
            for part in &query {
                if let Some(score) = matcher.fuzzy_match(&path.to_lowercase(), &part.to_lowercase())
                {
                    total_score += score;
                } else {
                    return None;
                }
            }
            Some((total_score, path))
        })
        .collect();

    matches.sort_by(|a, b| b.0.cmp(&a.0));
    matches.into_iter().map(|(_, path)| path).collect()
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    if let ROption::RSome(desc) = selection.description {
        let desc_str = desc.to_string();
        for scope in &state.config.scopes {
            if desc_str.contains(&scope.prefix) {
                let selection_title = selection.title.to_string();
                let cmd = scope.on_select.replace("{}", &selection_title);

                logger(&format!("Executing: {}", cmd), state);

                let _ = Command::new("sh").arg("-c").arg(cmd).spawn();
                return HandleResult::Close;
            }
        }
    }
    HandleResult::Close
}

#[cfg(test)]
mod tests {
    use abi_stable::std_types::ROption::RNone;

    use super::*;
    use std::{env, fs, path::PathBuf};

    #[test]
    fn test_read_real_config() {
        let home_dir = env::var("HOME").expect("Không tìm thấy biến môi trường HOME");

        let config_path = PathBuf::from(home_dir)
            .join(".config")
            .join("anyrun")
            .join("shell_wrapper.ron");

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

    #[test]
    fn read_brotab() {
        let results = get_list_output("sh /home/mintori/Desktop/test.sh");

        println!("--- Raw Results ---");
        results.iter().for_each(|result| {
            println!("{}", result);
        });

        println!("\n--- Matches ---");
        let matches: Vec<Match> = results
            .into_iter()
            .map(|result| Match {
                title: result.into(),
                description: RNone,
                id: RNone,
                use_pango: false,
                icon: RNone,
            })
            .collect();

        matches.iter().for_each(|m| {
            println!("{:?}", m);
        });
    }
}
