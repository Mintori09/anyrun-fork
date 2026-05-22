use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
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
            let stream = connect_with_retry(
                &socket_path,
                Duration::from_secs(12),
                Duration::from_millis(100),
            )
            .await?;
            let mut socket = ipc::Socket::new(stream);
            relay_loop(&mut socket, &mut rx, &sender).await
        })
}

async fn connect_with_retry(
    socket_path: &Path,
    timeout: Duration,
    interval: Duration,
) -> io::Result<tokio::net::UnixStream> {
    let started = Instant::now();
    let mut last_error = None;

    while started.elapsed() < timeout {
        match tokio::net::UnixStream::connect(socket_path).await {
            Ok(stream) => return Ok(stream),
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                ) =>
            {
                last_error = Some(err);
                tokio::time::sleep(interval).await;
            }
            Err(err) => return Err(err),
        }
    }

    let cause = last_error
        .map(|e| e.to_string())
        .unwrap_or_else(|| "timeout".to_string());
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!(
            "failed to connect to IPC socket `{}` within {:?}: {cause}",
            socket_path.display(),
            timeout
        ),
    ))
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

#[cfg(test)]
mod tests {
    use super::connect_with_retry;
    use std::io;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tokio::net::UnixListener;

    fn test_socket_path(name: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("anyrun-{name}-{ts}.sock"))
    }

    #[tokio::test(flavor = "current_thread")]
    async fn connect_with_retry_succeeds_when_socket_appears_later() {
        let socket_path = test_socket_path("delayed-connect");
        let path_for_server = socket_path.clone();

        let server = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let _ = std::fs::remove_file(&path_for_server);
            let listener = UnixListener::bind(&path_for_server).unwrap();
            let _ = listener.accept().await;
        });

        let stream = connect_with_retry(
            &socket_path,
            Duration::from_secs(2),
            Duration::from_millis(50),
        )
        .await
        .expect("socket should become available");
        drop(stream);
        let _ = server.await;
        let _ = std::fs::remove_file(socket_path);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn connect_with_retry_times_out_for_missing_socket() {
        let socket_path = test_socket_path("missing-socket");
        let err = connect_with_retry(
            &socket_path,
            Duration::from_millis(250),
            Duration::from_millis(50),
        )
        .await
        .expect_err("missing socket should time out");

        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
    }
}
