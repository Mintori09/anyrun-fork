use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::icon::SystemIcon;
use anyrun_plugin::*;
use serde::Deserialize;
use std::process::Command;
use std::{env, fs, path::PathBuf};

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
struct Config {
    prefix: String,
    max_entries: usize,
    scopes: Vec<SyncManager>,
}

#[derive(Deserialize, Debug, Clone)]
struct SyncManager {
    name: String,
    source: String,
    icon: SystemIcon,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "sy ".into(),
            max_entries: 10,
            scopes: Vec::new(),
        }
    }
}

pub struct State {
    config: Config,
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = PathBuf::from(config_dir.to_string()).join("sync_manager.ron");

    let config: Config = fs::read_to_string(config_path)
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_default();

    State { config }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Sync Manager".into(),
        icon: SystemIcon::WebBrowser.as_str().into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let input_str = input.to_string();

    if let Some(query) = input_str.strip_prefix(&state.config.prefix) {
        let query_parts: Vec<&str> = query.split_whitespace().collect();

        let matches = get_matches_fuzzy_finder(state.config.scopes.clone(), query_parts);

        return matches
            .into_iter()
            .take(state.config.max_entries)
            .map(|sync| Match {
                title: sync.name.into(),
                description: ROption::RSome(sync.source.into()),
                id: ROption::RNone,
                icon: ROption::RSome(sync.icon.as_str().into()),
                use_pango: false,
            })
            .collect::<Vec<_>>()
            .into();
    }

    RVec::new()
}

fn get_matches_fuzzy_finder(list: Vec<SyncManager>, query: Vec<&str>) -> Vec<SyncManager> {
    let mut matches: Vec<(i64, SyncManager)> = list
        .into_iter()
        .filter_map(|sync| {
            if query.is_empty() {
                return Some((0, sync));
            }

            let mut total_score = 0;
            let source = sync.source.to_lowercase();
            let name = sync.name.to_lowercase();

            for part in &query {
                let part_lower = part.to_lowercase();

                let title_score = anyrun_helper::mazzy_matcher::fuzzy_match(&source, &part_lower);

                let url_score = anyrun_helper::mazzy_matcher::fuzzy_match(&name, &part_lower);

                match (title_score, url_score) {
                    (Some(s1), Some(s2)) => total_score += s1.max(s2),
                    (Some(s), None) | (None, Some(s)) => total_score += s,
                    (None, None) => return None,
                }
            }

            Some((total_score, sync))
        })
        .collect();

    matches.sort_by_key(|b| std::cmp::Reverse(b.0));

    matches.into_iter().map(|(_, sync)| sync).collect()
}

#[handler]
fn handler(selection: Match, _state: &State) -> HandleResult {
    if let ROption::RSome(path) = selection.description {
        let expanded_path = if path.starts_with('~') {
            let home = env::var("HOME").unwrap_or_default();
            path.replacen('~', &home, 1)
        } else {
            path.into()
        };

        let _ = anyrun_plugin::spawn_detached(Command::new("sh").arg(expanded_path));
    }
    HandleResult::Close
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::fs;

    #[test]
    fn test_read_sample_config() {
        let config_path = std::env::temp_dir().join(format!(
            "anyrun-sync-manager-{}.ron",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &config_path,
            r#"(
                prefix: "sy ",
                max_entries: 5,
                scopes: [
                    (name: "Backup", source: "~/bin/backup.sh", icon: Search),
                ],
            )"#,
        )
        .unwrap();

        let content = fs::read_to_string(&config_path).expect("Không thể đọc file cấu hình mẫu");

        let result: Result<Config, _> = ron::from_str(&content);

        match result {
            Ok(config) => {
                assert_eq!(config.prefix, "sy ");
                assert_eq!(config.max_entries, 5);
                assert_eq!(config.scopes.len(), 1);
            }
            Err(e) => panic!("File RON tồn tại nhưng sai cấu trúc: {}", e),
        }

        let _ = fs::remove_file(config_path);
    }
}
