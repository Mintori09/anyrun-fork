use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;
use std::fs;
use zbus::blocking::Connection;
use zbus::proxy;

// --- Định nghĩa Proxy D-Bus ---
// Giải pháp: Đặt tên khác nhau cho blocking và async để tránh lỗi E0428/E0592
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

pub struct State {
    config: Config,
    connection: Connection,
}

#[init]
fn init(config_dir: RString) -> State {
    let config: Config = fs::read_to_string(format!("{}/klipper.ron", config_dir))
        .map(|content| ron::from_str(&content).unwrap_or_default())
        .unwrap_or_default();

    // Kết nối D-Bus (Blocking)
    let connection = Connection::session().expect("Failed to connect to D-Bus");

    State { config, connection }
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
    let prefix = &state.config.prefix;

    if let Some(query) = input_str.strip_prefix(prefix) {
        let query = query.trim();

        // Khởi tạo proxy từ connection có sẵn
        let proxy = match KlipperProxy::new(&state.connection) {
            Ok(p) => p,
            Err(_) => return RVec::new(),
        };

        let history = match proxy.get_clipboard_history_menu() {
            Ok(h) => h,
            Err(_) => return RVec::new(),
        };

        let matcher = SkimMatcherV2::default();

        // Thêm type annotation rõ ràng để sửa lỗi E0282
        let mut results: Vec<(i64, String)> = history
            .into_iter()
            .filter(|item| !item.trim().is_empty())
            .filter_map(|item: String| {
                let clean_item = item.replace('&', "");
                if query.is_empty() {
                    Some((0, clean_item))
                } else {
                    matcher
                        .fuzzy_match(&clean_item, query)
                        .map(|score| (score, clean_item))
                }
            })
            .collect();

        if !query.is_empty() {
            results.sort_by(|a, b| b.0.cmp(&a.0));
        }

        results
            .into_iter()
            .take(state.config.max_entries)
            .map(|(_score, text): (i64, String)| Match {
                title: text.clone().into(),
                description: ROption::RSome("Copy to clipboard".into()),
                use_pango: false,
                icon: ROption::RSome("edit-copy".into()),
                id: ROption::RNone,
            })
            .collect::<Vec<Match>>()
            .into()
    } else {
        RVec::new()
    }
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    let result = selection.title;

    if let Err(why) = std::process::Command::new("wl-copy")
        .arg(result.as_str())
        .spawn()
    {
        eprintln!("[libklipper] Failed to copy: {}", why);
    }

    HandleResult::Close
}
