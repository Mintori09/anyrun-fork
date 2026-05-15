use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::Arc,
};

use anyrun_provider_ipc as ipc;
use relm4::Sender;
use tokio::{net::UnixListener, sync::mpsc::Receiver};

use crate::config::Config;

pub fn worker_spawn(
    config: Arc<Config>,
    config_dir: Option<String>,
    mut rx: Receiver<anyrun_provider_ipc::Request>,
    sender: Sender<anyrun_provider_ipc::Response>,
    // The stdin received by the launching command
    stdin: Vec<u8>,
    // The environment of the launching command
    env: Vec<(String, String)>,
) -> io::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let socket_path = format!(
                "{}/anyrun.sock",
                env::var("XDG_RUNTIME_DIR").unwrap_or("/tmp".to_string())
            );
            // Make sure that it does not exist already
            let _ = fs::remove_file(&socket_path);
            let listener = UnixListener::bind(&socket_path).unwrap();

            let mut child = match Command::new(&config.provider)
                .stdin(Stdio::piped())
                .arg("--config-dir")
                .arg(config_dir.unwrap_or(ipc::CONFIG_DIRS[0].to_string()))
                .args(
                    config
                        .plugins
                        .iter()
                        .flat_map(|plugin| [PathBuf::from("-p"), plugin.to_owned()]),
                )
                .arg("connect-to")
                .arg(&socket_path)
                .envs(env)
                .spawn()
            {
                Ok(child) => child,
                Err(why) => match why.kind() {
                    io::ErrorKind::NotFound => {
                        eprintln!("[anyrun] `{}` Not found, make sure `anyrun-provider` is installed and available in $PATH, \
                             or configure an alternative path via the `provider` config option.", config.provider.display());
                        return Ok(());
                    }
                    _ => return Err(why),
                },
            };

            if let Some(mut child_stdin) = child.stdin.take() {
                child_stdin.write_all(&stdin).unwrap();
            };

            let (stream, _) = listener.accept().await?;
            let mut socket = ipc::Socket::new(stream);

            relay_loop(&mut socket, &mut rx, &sender).await?;

            // Remove it after we are done with it
            let _ = fs::remove_file(&socket_path);
            // Make sure it exits properly and doesn't leave a zombie process
            let _ = child.wait();

            Ok(())
        }
    )
}

pub fn worker_connect(
    socket_path: PathBuf,
    mut rx: Receiver<anyrun_provider_ipc::Request>,
    sender: Sender<anyrun_provider_ipc::Response>,
) -> io::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut attempts = 0;
            let stream = loop {
                match tokio::net::UnixStream::connect(&socket_path).await {
                    Ok(stream) => break stream,
                    Err(_e) if attempts < 10 => {
                        attempts += 1;
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    }
                    Err(e) => return Err(e),
                }
            };
            let mut socket = ipc::Socket::new(stream);
            relay_loop(&mut socket, &mut rx, &sender).await
        })
}

async fn relay_loop(
    socket: &mut ipc::Socket,
    rx: &mut Receiver<anyrun_provider_ipc::Request>,
    sender: &Sender<anyrun_provider_ipc::Response>,
) -> io::Result<()> {
    loop {
        tokio::select! {
            req = rx.recv() => {
                if let Some(req) = req {
                    socket.send(&req).await?;
                    if matches!(req, ipc::Request::Quit) {
                        break;
                    }
                }
            }
            res = socket.recv() => {
                match res {
                    Ok(response) => sender.emit(response),
                    Err(why) => {
                        eprintln!("[anyrun] Error reading from IPC: {why}");
                        break;
                    },
                }
            }
        }
    }
    Ok(())
}
