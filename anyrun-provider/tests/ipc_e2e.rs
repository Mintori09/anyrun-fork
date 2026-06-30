use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;

use anyrun_provider_ipc::{QueryPhase, Request, Response, Socket};

fn provider_bin() -> String {
    // CARGO_MANIFEST_DIR = <workspace>/anyrun-provider
    // Workspace root = <workspace>
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let ws = PathBuf::from(&dir).parent().unwrap().to_path_buf();
        for sub in &["release", "debug"] {
            let candidate = ws.join("target").join(sub).join("anyrun-provider");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }
    // Try cwd-relative fallback
    for p in &[
        "target/release/anyrun-provider",
        "target/debug/anyrun-provider",
        "../target/release/anyrun-provider",
        "../target/debug/anyrun-provider",
    ] {
        let path = PathBuf::from(p);
        if path.exists() {
            return path.canonicalize().unwrap().to_string_lossy().to_string();
        }
    }
    panic!("anyrun-provider binary not found");
}

fn temp_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "anyrun-provider-e2e-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn make_config(root: &PathBuf, provider: &str) {
    fs::create_dir_all(root.join("plugins")).unwrap();
    fs::write(
        root.join("config.ron"),
        format!("(provider: \"{provider}\", plugins: [])"),
    )
    .unwrap();
}

fn start_provider(socket_path: &PathBuf, config_dir: &PathBuf) -> Child {
    let bin = provider_bin();
    let mut child = Command::new(&bin)
        .arg("-c")
        .arg(config_dir)
        .arg("socket")
        .arg(socket_path)
        .spawn()
        .expect("failed to start anyrun-provider");

    for _ in 0..50 {
        if socket_path.exists() {
            return child;
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                panic!("provider exited early with {status}");
            }
            Ok(None) => {}
            Err(e) => panic!("error checking provider: {e}"),
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("provider did not create socket at {:?}", socket_path);
}

async fn connect_and_handshake(
    socket_path: &PathBuf,
) -> (Socket, Vec<anyrun_interface::PluginInfo>) {
    let stream = tokio::net::UnixStream::connect(socket_path)
        .await
        .expect("failed to connect to provider socket");
    let mut socket = Socket::new(stream);

    let resp: Response = socket
        .recv()
        .await
        .expect("failed to receive Ready response");
    match resp {
        Response::Ready { info } => (socket, info),
        _ => panic!("expected Response::Ready, got {:?}", resp),
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn provider_accepts_connection_and_sends_ready() {
    let root = temp_dir();
    make_config(&root, &provider_bin());
    let socket_path = root.join("provider.sock");

    let mut provider = start_provider(&socket_path, &root);
    let (_socket, info) = connect_and_handshake(&socket_path).await;

    assert!(
        info.is_empty(),
        "expected empty plugin info, got {} plugins",
        info.len()
    );

    provider.kill().expect("failed to kill provider");
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn provider_responds_to_query_with_no_plugins() {
    let root = temp_dir();
    make_config(&root, &provider_bin());
    let socket_path = root.join("provider.sock");

    let mut provider = start_provider(&socket_path, &root);
    let (mut socket, _info) = connect_and_handshake(&socket_path).await;

    socket
        .send(&Request::Query {
            text: "test".into(),
            phase: QueryPhase::Settling,
            plugins: vec![],
            timeout_ms: 500,
            slow_ms: 200,
        })
        .await
        .expect("failed to send Query request");

    let result = tokio::time::timeout(Duration::from_millis(300), socket.recv::<Response>()).await;
    match result {
        Ok(Ok(resp)) => match resp {
            Response::Health { .. } => { /* acceptable */ }
            other => panic!("unexpected response: {:?}", other),
        },
        Ok(Err(e)) => panic!("error receiving response: {e}"),
        Err(_timeout) => { /* timeout expected — no plugins = no matches */ }
    }

    provider.kill().expect("failed to kill provider");
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn provider_handles_quit_gracefully() {
    let root = temp_dir();
    make_config(&root, &provider_bin());
    let socket_path = root.join("provider.sock");

    let mut provider = start_provider(&socket_path, &root);
    let (mut socket, _info) = connect_and_handshake(&socket_path).await;

    socket
        .send(&Request::Quit)
        .await
        .expect("failed to send Quit");

    let status = provider.wait().expect("provider failed to exit after Quit");
    assert!(
        status.success(),
        "provider exited with non-zero status: {status}"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn provider_refuses_second_connection() {
    let root = temp_dir();
    make_config(&root, &provider_bin());
    let socket_path = root.join("provider.sock");

    let mut provider = start_provider(&socket_path, &root);
    let (_socket, _info) = connect_and_handshake(&socket_path).await;

    let second = tokio::time::timeout(Duration::from_secs(2), async {
        tokio::net::UnixStream::connect(&socket_path).await
    })
    .await;
    assert!(
        second.is_ok(),
        "second connection should succeed at socket level"
    );

    provider.kill().expect("failed to kill provider");
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn provider_rejects_invalid_request() {
    let root = temp_dir();
    make_config(&root, &provider_bin());
    let socket_path = root.join("provider.sock");

    let mut provider = start_provider(&socket_path, &root);
    let (mut socket, _info) = connect_and_handshake(&socket_path).await;

    use tokio::io::AsyncWriteExt;
    socket
        .inner
        .get_mut()
        .write_all(b"\xff\xff\xff\xffgarbage")
        .await
        .expect("failed to send garbage");

    let result = socket.recv::<Response>().await;
    assert!(
        result.is_err(),
        "expected error from garbage data, got {:?}",
        result
    );

    provider.kill().expect("failed to kill provider");
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn provider_query_fast_lane_plugins() {
    let root = temp_dir();
    make_config(&root, &provider_bin());
    let socket_path = root.join("provider.sock");

    let mut provider = start_provider(&socket_path, &root);
    let (mut socket, _info) = connect_and_handshake(&socket_path).await;

    socket
        .send(&Request::Query {
            text: "search".into(),
            phase: QueryPhase::Typing,
            plugins: vec!["nonexistent".into()],
            timeout_ms: 500,
            slow_ms: 200,
        })
        .await
        .expect("failed to send Query");

    let result = tokio::time::timeout(Duration::from_millis(300), socket.recv::<Response>()).await;
    assert!(result.is_err(), "expected timeout but got response");

    provider.kill().expect("failed to kill provider");
    let _ = std::fs::remove_dir_all(root);
}
