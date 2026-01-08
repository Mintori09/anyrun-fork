use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::focus_to_class;
use anyrun_helper::icon::SystemIcon;
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::process::Command;
use std::{env, fs, path::PathBuf};

#[derive(Deserialize, Debug)]
#[serde(default)]
struct Config {
    prefix: String,
    max_entries: usize,
    source: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "tab ".into(),
            source: "~/.local/bin/brotab".into(),
            max_entries: 10,
        }
    }
}

pub struct State {
    config: Config,
}

#[derive(Debug)]
pub struct Browser {
    title: String,
    url: String,
    id: String,
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = PathBuf::from(config_dir.to_string()).join("browser.ron");

    let config: Config = fs::read_to_string(config_path)
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_default();

    State { config }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Browser Tabs".into(),
        icon: SystemIcon::WebBrowser.as_str().into(),
    }
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(target_family = "windows")]
    {
        if let Some(v) = env::var_os("USERPROFILE") {
            return Some(PathBuf::from(v));
        }
        let drive = env::var_os("HOMEDRIVE");
        let path = env::var_os("HOMEPATH");
        if let (Some(d), Some(p)) = (drive, path) {
            return Some(PathBuf::from(PathBuf::from(d).join(p)));
        }
    }

    #[cfg(target_family = "unix")]
    {
        if let Some(home) = env::var_os("HOME") {
            return Some(PathBuf::from(home));
        }
    }

    None
}
fn get_icon_path(url_str: &str) -> String {
    // Trích xuất domain sạch (ví dụ: laravel.com)
    let domain = url_str
        .replace("https://", "")
        .replace("http://", "")
        .split('/')
        .next()
        .unwrap_or("default")
        .to_string();

    let cache_dir = format!(
        "{}/.config/anyrun/anyrun-favicons",
        home_dir().unwrap().to_string_lossy()
    );
    let icon_path = format!("{}/{}.png", cache_dir, domain);

    if std::path::Path::new(&icon_path).exists() {
        return icon_path;
    }

    // Tạo thư mục và tải ngầm
    let _ = std::fs::create_dir_all(cache_dir);
    let dest = icon_path.clone();

    // Sử dụng URL gstatic mới
    let download_url = format!(
        "https://t3.gstatic.com/faviconV2?client=SOCIAL&type=FAVICON&fallback_opts=TYPE,SIZE,URL&url=http://{}&size=64",
        domain
    );

    std::thread::spawn(move || {
        // Flag -L để xử lý 301/302 redirect
        let _ = std::process::Command::new("curl")
            .arg("-L")
            .arg("-s")
            .arg("-o")
            .arg(dest)
            .arg(download_url)
            .output();
    });

    // Trả về icon mặc định của hệ thống trong khi chờ tải
    "web-browser".to_string()
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let input_str = input.to_string();

    if let Some(query) = input_str.strip_prefix(&state.config.prefix) {
        let query_parts: Vec<&str> = query.split_whitespace().collect();

        let full_path = if state.config.source.starts_with('~') {
            let home = env::var("HOME").unwrap_or_default();
            state.config.source.replacen('~', &home, 1)
        } else {
            state.config.source.clone()
        };

        let tabs = fetch_tab(&full_path);
        let matches = get_matches_fuzzy_finder(tabs, query_parts);

        return matches
            .into_iter()
            .take(state.config.max_entries)
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

fn fetch_tab(bin_path: &str) -> Vec<Browser> {
    let output = Command::new(bin_path).arg("list").output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split('\t').collect();

                    if parts.len() >= 3 {
                        Some(Browser {
                            id: parts[0].to_string(),
                            title: parts[1].to_string(),
                            url: parts[2].to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn get_matches_fuzzy_finder(list: Vec<Browser>, query: Vec<&str>) -> Vec<Browser> {
    let matcher = SkimMatcherV2::default();

    let mut matches: Vec<(i64, Browser)> = list
        .into_iter()
        .filter_map(|browser| {
            if query.is_empty() {
                return Some((0, browser));
            }

            let mut total_score = 0;
            let title_lower = browser.title.to_lowercase();
            let url_lower = browser.url.to_lowercase();

            for part in &query {
                let part_lower = part.to_lowercase();

                let title_score = matcher.fuzzy_match(&title_lower, &part_lower);

                let url_score = matcher.fuzzy_match(&url_lower, &part_lower);

                match (title_score, url_score) {
                    (Some(s1), Some(s2)) => total_score += s1.max(s2),
                    (Some(s), None) | (None, Some(s)) => total_score += s,
                    (None, None) => return None,
                }
            }

            Some((total_score, browser))
        })
        .collect();

    matches.sort_by(|a, b| b.0.cmp(&a.0));

    matches.into_iter().map(|(_, browser)| browser).collect()
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    if let ROption::RSome(tab_id) = selection.description {
        let full_path = if state.config.source.starts_with('~') {
            let home = env::var("HOME").unwrap_or_default();
            state.config.source.replacen('~', &home, 1)
        } else {
            state.config.source.clone()
        };
        focus_to_class("firefox");

        let _ = Command::new(full_path)
            .arg("activate")
            .arg(tab_id.to_string())
            .spawn();
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
            .join("browser.ron");

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
    fn test_fetch() {
        let results: Vec<Browser> = fetch_tab("/home/mintori/.local/bin/brotab");
        results.iter().for_each(|result| {
            println!("{:?}", result);
        });
    }
}
