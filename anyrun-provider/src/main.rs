use anyrun_interface::{HandleResult, Match, PluginInfo, PluginRef, abi_stable};
use anyrun_provider_ipc::{CONFIG_DIRS, PLUGIN_PATHS, Request, Response, Socket};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, io, path::PathBuf, sync::Arc};
use tokio::{
    net::{UnixListener, UnixStream},
    sync::Mutex,
    task::{AbortHandle, JoinHandle},
};

// Định nghĩa alias để code gọn gàng hơn
type PluginQueryResult = (abi_stable::std_types::RVec<Match>, usize);

#[derive(Serialize, Deserialize, Default)]
struct FrecencyData {
    // (plugin_name, match_title) -> timestamps
    pub usage: HashMap<(String, String), Vec<DateTime<Utc>>>,
}

impl FrecencyData {
    pub fn load(config_dir: &str) -> Self {
        let path = PathBuf::from(config_dir).join("frecency.json");
        fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, config_dir: &str) {
        let path = PathBuf::from(config_dir).join("frecency.json");
        if let Ok(content) = serde_json::to_string(self) {
            let _ = fs::write(path, content);
        }
    }

    pub fn cleanup(&mut self) {
        let now = Utc::now();
        let max_age = chrono::Duration::days(30);
        for usages in self.usage.values_mut() {
            usages.retain(|&time| now.signed_duration_since(time) <= max_age);
        }
        self.usage.retain(|_, usages| !usages.is_empty());
    }

    pub fn get_score(&self, plugin: &str, title: &str, half_life_days: f64) -> f64 {
        let now = Utc::now();
        let mut total_score = 0.0;
        if let Some(usages) = self.usage.get(&(plugin.to_string(), title.to_string())) {
            for &time in usages {
                let duration = now.signed_duration_since(time);
                let days = duration.num_seconds() as f64 / 86400.0;
                total_score += 0.5f64.powf(days / half_life_days);
            }
        }
        total_score
    }
}

#[derive(Parser)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Command,
    #[arg(short, long)]
    plugins: Vec<PathBuf>,
    #[arg(short, long)]
    config_dir: Option<String>,
}

#[derive(Clone, Subcommand)]
enum Command {
    Socket { path: PathBuf },
    ConnectTo { path: PathBuf },
}

enum WorkerResult {
    Quit,
    Continue,
}

struct PluginState {
    plugin: PluginRef,
    info: PluginInfo,
}

struct State {
    plugins: Vec<PluginState>,
    plugin_map: HashMap<String, usize>,
    config_dir: Arc<str>,
    frecency: Arc<Mutex<FrecencyData>>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Args::parse();

    let user_dir = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut p = PathBuf::from(env::var("HOME").unwrap_or_else(|_| ".".into()));
            p.push(".config");
            p
        })
        .join("anyrun");

    let config_dir: Arc<str> = args.config_dir.map(Into::into).unwrap_or_else(|| {
        if user_dir.exists() {
            user_dir.to_string_lossy().into()
        } else {
            CONFIG_DIRS
                .iter()
                .find(|p| PathBuf::from(p).exists())
                .map(|&p| p.into())
                .unwrap_or_else(|| CONFIG_DIRS[0].into())
        }
    });

    let mut frecency = FrecencyData::load(&config_dir);
    frecency.cleanup();
    let frecency = Arc::new(Mutex::new(frecency));

    let mut plugin_dirs = vec![user_dir.join("plugins")];
    if let Ok(path) = env::var("ANYRUN_PLUGINS") {
        plugin_dirs.push(PathBuf::from(path));
    }
    plugin_dirs.extend(PLUGIN_PATHS.iter().map(PathBuf::from));

    let mut state = State {
        plugins: Vec::with_capacity(args.plugins.len()),
        plugin_map: HashMap::with_capacity(args.plugins.len()),
        config_dir,
        frecency,
    };

    for plugin_path in &args.plugins {
        if let Some(path) = find_plugin(plugin_path, &plugin_dirs)
            && let Ok(header) = abi_stable::library::lib_header_from_path(&path)
            && let Ok(plugin) = header.init_root_module::<PluginRef>()
        {
            plugin.init()(state.config_dir.as_ref().into());
            let info = plugin.info()();
            let idx = state.plugins.len();
            state.plugin_map.insert(info.name.to_string(), idx);
            state.plugins.push(PluginState { plugin, info });
        }
    }

    match args.command {
        Command::Socket { path } => {
            let _ = std::fs::remove_file(&path);
            let listener = UnixListener::bind(path)?;
            loop {
                let (stream, _) = listener.accept().await?;
                if let WorkerResult::Quit = worker(stream, &mut state).await? {
                    break;
                }
            }
        }
        Command::ConnectTo { path } => {
            let stream = UnixStream::connect(path).await?;
            worker(stream, &mut state).await?;
        }
    }
    Ok(())
}

async fn worker(stream: UnixStream, state: &mut State) -> io::Result<WorkerResult> {
    let mut socket = Socket::new(stream);

    let plugin_infos: Vec<PluginInfo> = state.plugins.iter().map(|p| p.info.clone()).collect();
    socket.send(&Response::Ready { info: plugin_infos }).await?;

    let mut pending_results: FuturesUnordered<JoinHandle<PluginQueryResult>> =
        FuturesUnordered::new();
    let mut abort_handles: Vec<AbortHandle> = Vec::new();

    loop {
        tokio::select! {
            Some(join_result) = pending_results.next() => {
                if let Ok((mut matches, idx)) = join_result
                    && let Some(p_state) = state.plugins.get(idx) {
                        // Apply frecency re-sorting while preserving match quality
                        let mut indexed_matches: Vec<_> = matches.into_iter().enumerate().collect();
                        {
                            let frecency = state.frecency.lock().await;
                            let plugin_name = p_state.info.name.as_str();

                            indexed_matches.sort_by(|(idx_a, a), (idx_b, b)| {
                                let f_score_a = frecency.get_score(plugin_name, a.title.as_str(), 7.0);
                                let f_score_b = frecency.get_score(plugin_name, b.title.as_str(), 7.0);

                                // Balance original rank and frecency
                                // Higher score is better
                                let score_a = (1.0 / (1.0 + *idx_a as f64)) + f_score_a * 0.1;
                                let score_b = (1.0 / (1.0 + *idx_b as f64)) + f_score_b * 0.1;

                                score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
                            });
                        }
                        matches = indexed_matches.into_iter().map(|(_, m)| m).collect();

                        socket.send(&Response::Matches {
                            plugin: p_state.info.clone(),
                            matches,
                        }).await?;
                    }
            }

            req_result = socket.recv() => {
                let request = match req_result {
                    Ok(req) => req,
                    // Chỉ định rõ kiểu io::Error để Rust không bị nhầm lẫn
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e),
                };

                match request {
                    Request::Query { text } => {
                        for handle in abort_handles.drain(..) {
                            handle.abort();
                        }
                        pending_results.clear();

                        let query: Arc<str> = text.into();
                        for (idx, p_state) in state.plugins.iter().enumerate() {
                            let plugin_fn = p_state.plugin.get_matches();
                            let q = Arc::clone(&query);

                            let handle = tokio::task::spawn_blocking(move || {
                                (plugin_fn(q.as_ref().into()), idx)
                            });

                            abort_handles.push(handle.abort_handle());
                            pending_results.push(handle);
                        }
                    }
                    Request::Handle { plugin, selection } => {
                        if let Some(&idx) = state.plugin_map.get(plugin.name.as_str()) {
                            // Update frecency
                            {
                                let mut frecency = state.frecency.lock().await;
                                frecency.usage
                                    .entry((plugin.name.to_string(), selection.title.to_string()))
                                    .or_default()
                                    .push(Utc::now());
                                frecency.cleanup();
                                frecency.save(&state.config_dir);
                            }

                            let handle_fn = state.plugins[idx].plugin.handle_selection();
                            let result = tokio::task::spawn_blocking(move || {
                                handle_fn(selection)
                            })
                            .await
                            .unwrap_or(HandleResult::Close);
                            socket.send(&Response::Handled { plugin, result }).await?;
                        }
                    }
                    Request::Reset => {
                        pending_results.clear();
                        for p in &mut state.plugins {
                            p.plugin.init()(state.config_dir.as_ref().into());
                        }
                    }
                    Request::Quit => return Ok(WorkerResult::Quit),
                }
            }
        }
    }
    Ok(WorkerResult::Continue)
}

fn find_plugin(name: &std::path::Path, dirs: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    let name = expand_tilde(name);
    if name.is_absolute() && name.exists() {
        return Some(name.clone());
    }
    for dir in dirs {
        let p = dir.join(&name);
        if p.exists() {
            return Some(p);
        }

        let lib_name = format!("lib{}.so", name.to_string_lossy().replace('-', "_"));
        let p = dir.join(lib_name);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn expand_tilde(path: &std::path::Path) -> std::path::PathBuf {
    if let Some(path_str) = path.to_str()
        && path_str.starts_with("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return std::path::PathBuf::from(path_str.replacen('~', &home, 1));
    }
    path.to_path_buf()
}
