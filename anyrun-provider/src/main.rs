use anyrun_interface::{Match, PluginInfo, PluginRef, abi_stable};
use anyrun_provider_ipc::{CONFIG_DIRS, PLUGIN_PATHS, Request, Response, Socket};
use clap::{Parser, Subcommand};
use futures::stream::{FuturesUnordered, StreamExt};
use std::{collections::HashMap, env, io, path::PathBuf, sync::Arc};
use tokio::{
    net::{UnixListener, UnixStream},
    task::{AbortHandle, JoinHandle},
};

// Định nghĩa alias để code gọn gàng hơn
type PluginQueryResult = (abi_stable::std_types::RVec<Match>, usize);

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

    let mut plugin_dirs = vec![user_dir.join("plugins")];
    if let Ok(path) = env::var("ANYRUN_PLUGINS") {
        plugin_dirs.push(PathBuf::from(path));
    }
    plugin_dirs.extend(PLUGIN_PATHS.iter().map(PathBuf::from));

    let mut state = State {
        plugins: Vec::with_capacity(args.plugins.len()),
        plugin_map: HashMap::with_capacity(args.plugins.len()),
        config_dir,
    };

    for plugin_path in &args.plugins {
        if let Some(path) = find_plugin(plugin_path, &plugin_dirs) {
            if let Ok(header) = abi_stable::library::lib_header_from_path(&path) {
                if let Ok(plugin) = header.init_root_module::<PluginRef>() {
                    plugin.init()(state.config_dir.as_ref().into());
                    let info = plugin.info()();
                    let idx = state.plugins.len();
                    state.plugin_map.insert(info.name.to_string(), idx);
                    state.plugins.push(PluginState { plugin, info });
                }
            }
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
                if let Ok((matches, idx)) = join_result {
                    if let Some(p_state) = state.plugins.get(idx) {
                        socket.send(&Response::Matches {
                            plugin: p_state.info.clone(),
                            matches,
                        }).await?;
                    }
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
                        if let Some(&idx) = state.plugin_map.get(&plugin.name.to_string()) {
                            let p = &state.plugins[idx];
                            let result = p.plugin.handle_selection()(selection);
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

fn find_plugin(name: &PathBuf, dirs: &[PathBuf]) -> Option<PathBuf> {
    if name.is_absolute() && name.exists() {
        return Some(name.clone());
    }
    for dir in dirs {
        let p = dir.join(name);
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
