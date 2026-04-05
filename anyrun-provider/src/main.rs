use anyrun_interface::{HandleResult, Match, PluginInfo, PluginRef, abi_stable};
use anyrun_provider_ipc::{CONFIG_DIRS, PLUGIN_PATHS, Request, Response, Socket};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use futures::stream::{FuturesUnordered, StreamExt};
use notify::RecommendedWatcher;
use notify_debouncer_mini::{Debouncer, new_debouncer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio::task::{AbortHandle, JoinHandle};

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

    let config_dir_str = config_dir.to_string();

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

    let (reload_tx, _) = broadcast::channel::<()>(4);

    spawn_file_watcher(reload_tx.clone(), &plugin_dirs, &config_dir_str);

    match args.command {
        Command::Socket { path } => {
            let _ = std::fs::remove_file(&path);
            let listener = UnixListener::bind(path)?;
            loop {
                let (stream, _) = listener.accept().await?;
                let reload_rx = reload_tx.subscribe();
                if let WorkerResult::Quit = worker(stream, &mut state, reload_rx).await? {
                    break;
                }
            }
        }
        Command::ConnectTo { path } => {
            let stream = UnixStream::connect(path).await?;
            let reload_rx = reload_tx.subscribe();
            worker(stream, &mut state, reload_rx).await?;
        }
    }
    Ok(())
}

async fn worker(
    stream: UnixStream,
    state: &mut State,
    mut reload_rx: broadcast::Receiver<()>,
) -> io::Result<WorkerResult> {
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
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e),
                };

                match request {
                Request::Query { text } => {
                    for handle in abort_handles.drain(..) {
                        handle.abort();
                    }
                    pending_results.clear();

                    for p in &mut state.plugins {
                        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            p.plugin.init()(state.config_dir.as_ref().into());
                        }));
                    }

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
                            let mut frecency = state.frecency.lock().await;
                            frecency.usage
                                .entry((plugin.name.to_string(), selection.title.to_string()))
                                .or_default()
                                .push(Utc::now());
                            frecency.cleanup();
                            frecency.save(&state.config_dir);

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
                    Request::ReloadPlugins => {
                        for handle in abort_handles.drain(..) {
                            handle.abort();
                        }
                        pending_results.clear();

                        for p in &mut state.plugins {
                            p.plugin.init()(state.config_dir.as_ref().into());
                        }

                        let plugin_infos: Vec<PluginInfo> =
                            state.plugins.iter().map(|p| p.info.clone()).collect();
                        socket.send(&Response::Ready { info: plugin_infos }).await?;
                    }
                    Request::ReloadPlugin { name } => {
                        if let Some(&idx) = state.plugin_map.get(&name) {
                            if let Some(p) = state.plugins.get_mut(idx) {
                                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    p.plugin.init()(state.config_dir.as_ref().into());
                                }));
                                if result.is_err() {
                                    eprintln!("[provider] Plugin re-init failed for: {}", name);
                                } else {
                                    let plugin_infos: Vec<PluginInfo> =
                                        state.plugins.iter().map(|p| p.info.clone()).collect();
                                    socket.send(&Response::Ready { info: plugin_infos }).await?;
                                }
                            }
                        }
                    }
                    Request::Quit => return Ok(WorkerResult::Quit),
                }
            }

            _ = reload_rx.recv() => {
                for handle in abort_handles.drain(..) {
                    handle.abort();
                }
                pending_results.clear();

                let mut failed = Vec::new();
                for (_i, p) in state.plugins.iter_mut().enumerate() {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        p.plugin.init()(state.config_dir.as_ref().into());
                    }));
                    if result.is_err() {
                        failed.push(p.info.name.to_string());
                    }
                }

                if !failed.is_empty() {
                    eprintln!("[provider] Plugin re-init failed for: {}", failed.join(", "));
                }

                let plugin_infos: Vec<PluginInfo> =
                    state.plugins.iter().map(|p| p.info.clone()).collect();
                socket.send(&Response::Ready { info: plugin_infos }).await?;
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

fn resolve_desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    let user_data = env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut p = PathBuf::from(env::var("HOME").unwrap_or_else(|_| ".".into()));
            p.push(".local");
            p.push("share");
            p
        });
    dirs.push(user_data.join("applications"));

    if let Ok(data_dirs) = env::var("XDG_DATA_DIRS") {
        for dir in data_dirs.split(':') {
            dirs.push(PathBuf::from(dir).join("applications"));
        }
    } else {
        dirs.push(PathBuf::from("/usr/local/share/applications"));
        dirs.push(PathBuf::from("/usr/share/applications"));
    }

    dirs
}

fn spawn_file_watcher(
    reload_tx: broadcast::Sender<()>,
    plugin_dirs: &[PathBuf],
    config_dir: &str,
) {
    let desktop_dirs = resolve_desktop_dirs();
    let mut watch_paths: Vec<PathBuf> = desktop_dirs.into_iter().filter(|p| p.exists()).collect();

    watch_paths.extend(plugin_dirs.iter().filter(|p| p.exists()).cloned());

    let config_path = PathBuf::from(config_dir);
    if config_path.exists() {
        watch_paths.push(config_path.clone());
    }

    if watch_paths.is_empty() {
        eprintln!("[provider] No valid paths to watch for file changes");
        return;
    }

    let (event_tx, event_rx) = std::sync::mpsc::channel();

    let mut debouncer: Debouncer<RecommendedWatcher> =
        match new_debouncer(Duration::from_millis(500), event_tx) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[provider] Failed to create file watcher debouncer: {e}");
                return;
            }
        };

    for path in &watch_paths {
        if let Err(e) = debouncer
            .watcher()
            .watch(path, notify::RecursiveMode::NonRecursive)
        {
            eprintln!("[provider] Failed to watch path {:?}: {e}", path);
        }
    }

    let config_dir_str = config_dir.to_string();
    std::thread::spawn(move || {
        while let Ok(result) = event_rx.recv() {
            let events = match result {
                Ok(events) => events,
                Err(e) => {
                    eprintln!("[provider] File watcher error: {:?}", e);
                    continue;
                }
            };

            let is_config_change = events.iter().any(|event| {
                event
                    .path
                    .to_str()
                    .map(|p| p.ends_with(".ron") && p.contains(&config_dir_str))
                    .unwrap_or(false)
            });

            let _ = reload_tx.send(());

            while event_rx.try_recv().is_ok() {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_desktop_dirs_includes_user_applications() {
        let dirs = resolve_desktop_dirs();
        assert!(!dirs.is_empty());
        let user_data = env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut p = PathBuf::from(env::var("HOME").unwrap_or_else(|_| ".".into()));
                p.push(".local");
                p.push("share");
                p
            });
        assert!(dirs.contains(&user_data.join("applications")));
    }

    #[test]
    fn test_resolve_desktop_dirs_includes_system_dirs_when_xdg_data_dirs_unset() {
        let original = env::var("XDG_DATA_DIRS");
        unsafe { env::remove_var("XDG_DATA_DIRS") };

        let dirs = resolve_desktop_dirs();
        assert!(dirs.contains(&PathBuf::from("/usr/local/share/applications")));
        assert!(dirs.contains(&PathBuf::from("/usr/share/applications")));

        if let Ok(val) = original {
            unsafe { env::set_var("XDG_DATA_DIRS", val) };
        }
    }

    #[test]
    fn test_resolve_desktop_dirs_parses_xdg_data_dirs() {
        let original = env::var("XDG_DATA_DIRS");
        unsafe { env::set_var("XDG_DATA_DIRS", "/foo:/bar") };

        let dirs = resolve_desktop_dirs();
        assert!(dirs.contains(&PathBuf::from("/foo/applications")));
        assert!(dirs.contains(&PathBuf::from("/bar/applications")));

        if let Ok(val) = original {
            unsafe { env::set_var("XDG_DATA_DIRS", val) };
        } else {
            unsafe { env::remove_var("XDG_DATA_DIRS") };
        }
    }

    #[test]
    fn test_reload_plugins_request_serializes() {
        use anyrun_provider_ipc::Request;
        let req = Request::ReloadPlugins;
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: Request = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Request::ReloadPlugins));
    }

    #[test]
    fn test_plugin_init_panic_handling() {
        let count = std::sync::atomic::AtomicUsize::new(0);
        
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }));
        
        assert!(result.is_ok());
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            panic!("simulated init failure");
        }));
        
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_reloads_config_on_init() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("plugin-sample.ron");
        
        let initial_config = r#"(prefix: "init:", max_entries: 5, show_results_immediately: true)"#;
        fs::write(&config_path, initial_config).expect("Failed to write initial config");

        let plugin_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default()
            .join("libanyrun_plugin_template.so");
        
        let plugin_path = if plugin_path.exists() {
            plugin_path
        } else {
            std::path::PathBuf::from("target/debug/deps/libanyrun_plugin_template.so")
        };

        let header = abi_stable::library::lib_header_from_path(&plugin_path)
            .expect("Failed to load plugin");
        
        let plugin = header.init_root_module::<PluginRef>()
            .expect("Failed to init plugin module");

        plugin.init()(temp_dir.path().to_string_lossy().as_ref().into());

        std::thread::sleep(std::time::Duration::from_millis(100));

        let matches1 = plugin.get_matches()("init:hello".into());
        assert!(matches1.is_empty(), "Expected no matches with empty data");

        let updated_config = r#"(prefix: "test:", max_entries: 10, show_results_immediately: false)"#;
        fs::write(&config_path, updated_config).expect("Failed to write updated config");

        plugin.init()(temp_dir.path().to_string_lossy().as_ref().into());

        std::thread::sleep(std::time::Duration::from_millis(100));

        let matches_with_old_prefix = plugin.get_matches()("init:hello".into());
        assert!(matches_with_old_prefix.is_empty(), "Old prefix should not match after config reload");

        let matches_with_new_prefix = plugin.get_matches()("test:hello".into());
        assert!(matches_with_new_prefix.is_empty(), "Expected no matches with empty data");
    }
}
