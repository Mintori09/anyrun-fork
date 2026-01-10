use abi_stable::std_types::{ROption, RString, RVec};
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
            prefix: "port ".into(),
            max_entries: 10,
            cache_ttl_secs: 1,
        }
    }
}

pub struct State {
    config: Config,
    matcher: SkimMatcherV2,
    cache: Mutex<Option<(Instant, Vec<ActivePort>)>>,
}

#[derive(Debug, Clone)]
pub struct ActivePort {
    pub port: String,
    pub proto: String,
    pub process: String,
    pub pid: String,
}

pub fn get_active_ports() -> Vec<ActivePort> {
    let mut active_ports = Vec::new();

    // -t (tcp), -u (udp), -l (listening), -p (processes), -n (numeric)
    let output = Command::new("ss").args(["-tulpn"]).output();

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);

        for line in text.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                continue;
            }

            let proto = parts[0].to_string();

            // Handle Local Address (Column 4)
            // Works for 0.0.0.0:80 and [::]:80
            let local_addr = parts[4];
            let port = local_addr.rsplit(':').next().unwrap_or("").to_string();

            // Handle Process Info (Column 6)
            // Format: users:(("name",pid=123,fd=4))
            let mut process_name = "unknown".to_string();
            let mut pid = "unknown".to_string();

            if parts.len() >= 7 {
                let process_part = parts[6]; // Sometimes ss puts users in index 6 if index 5 is empty
                if let Some(start) = process_part.find('"') {
                    let remaining = &process_part[start + 1..];
                    if let Some(end) = remaining.find('"') {
                        process_name = remaining[..end].to_string();
                    }
                }

                if let Some(pid_start) = process_part.find("pid=") {
                    let pid_section = &process_part[pid_start + 4..];
                    pid = pid_section
                        .split(',')
                        .next()
                        .unwrap_or("unknown")
                        .to_string();
                }
            }

            active_ports.push(ActivePort {
                port,
                proto,
                process: process_name,
                pid,
            });
        }
    }

    active_ports
}

/// Parses the "users:(("name",pid=123,fd=4))" string from ss output
fn parse_ss_process(raw: &str) -> (String, String) {
    // Basic extraction logic without regex for speed
    let name = raw.split('"').nth(1).unwrap_or("unknown").to_string();
    let pid = raw
        .split("pid=")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .unwrap_or("unknown")
        .to_string();

    (name, pid)
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = PathBuf::from(config_dir.to_string()).join("port_killer.ron");

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
        name: "Port Killer".into(),
        icon: SystemIcon::SystemRun.as_str().into(),
    }
}

fn get_ports_with_cache(state: &State) -> Vec<ActivePort> {
    let mut cache = state.cache.lock().unwrap();

    if let Some((inst, ports)) = cache.as_ref() {
        if inst.elapsed() < Duration::from_secs(state.config.cache_ttl_secs) {
            return ports.clone();
        }
    }

    let ports = get_active_ports();
    *cache = Some((Instant::now(), ports.clone()));
    ports
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let input_str = input.to_string();

    if let Some(query) = input_str.strip_prefix(&state.config.prefix) {
        let ports = get_ports_with_cache(state);

        let mut scored_matches: Vec<(i64, ActivePort)> = ports
            .into_iter()
            .filter_map(|p| {
                if query.is_empty() {
                    return Some((0, p));
                }

                // Tìm kiếm theo số port hoặc tên tiến trình
                let search_text = format!("{} {}", p.port, p.process);
                let score = state.matcher.fuzzy_match(&search_text, query.trim());
                score.map(|s| (s, p))
            })
            .collect();

        scored_matches.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        return scored_matches
            .into_iter()
            .take(state.config.max_entries)
            .map(|(_, p)| Match {
                title: format!("Port {}: {}", p.port, p.process).into(),
                description: ROption::RSome(p.pid.into()),
                id: ROption::RNone,
                icon: ROption::RSome(SystemIcon::SystemRun.as_str().into()),
                use_pango: false,
            })
            .collect::<Vec<_>>()
            .into();
    }

    RVec::new()
}

#[handler]
fn handler(selection: Match, _state: &State) -> HandleResult {
    if let ROption::RSome(pid) = selection.id {
        let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
    }
    HandleResult::Close
}
