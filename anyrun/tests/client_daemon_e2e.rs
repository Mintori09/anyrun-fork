use gtk4::{gio, glib, prelude::*};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

fn anyrun_bin() -> String {
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let ws = PathBuf::from(&dir).parent().unwrap().to_path_buf();
        for sub in &["release", "debug"] {
            let candidate = ws.join("target").join(sub).join("anyrun");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }
    for p in &[
        "target/release/anyrun",
        "target/debug/anyrun",
        "../target/release/anyrun",
        "../target/debug/anyrun",
    ] {
        let path = PathBuf::from(p);
        if path.exists() {
            return path.canonicalize().unwrap().to_string_lossy().to_string();
        }
    }
    panic!("anyrun binary not found");
}

fn temp_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "anyrun-client-daemon-e2e-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn make_config(root: &Path) {
    fs::create_dir_all(root.join("plugins")).unwrap();
    fs::write(
        root.join("config.ron"),
        "(
            plugins: [],
            provider: \"anyrun-provider\",
        )",
    )
    .unwrap();
    fs::write(root.join("style.css"), "").unwrap();
}

#[tokio::test]
async fn test_client_daemon_e2e_communication() {
    // 1. Spawn a private dbus-daemon
    let mut dbus_child = match Command::new("dbus-daemon")
        .arg("--session")
        .arg("--print-address=1")
        .arg("--nofork")
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            println!("dbus-daemon not found, skipping integration test");
            return;
        }
    };

    let mut dbus_address = String::new();
    if let Some(stdout) = dbus_child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let l = line.unwrap();
            if l.starts_with("unix:path=") || l.starts_with("unix:abstract=") {
                dbus_address = l;
                break;
            }
        }
    }

    assert!(!dbus_address.is_empty(), "Failed to get D-Bus session address");

    // 2. Set up config directories
    let config_dir = temp_dir();
    make_config(&config_dir);

    let bin = anyrun_bin();

    // 3. Start the daemon under the private DBus address
    let mut daemon_child = Command::new(&bin)
        .arg("--config-dir")
        .arg(&config_dir)
        .arg("daemon")
        .env("DBUS_SESSION_BUS_ADDRESS", &dbus_address)
        .env("GSK_RENDERER", "cairo")
        .env("GDK_BACKEND", "broadway")
        .env("NO_AT_BRIDGE", "1")
        .env("GTK_A11Y", "none")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn anyrun daemon");

    // Wait for the daemon to register its name on D-Bus Session
    let dbus_conn = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>).unwrap();
    let mut registered = false;
    let start_time = std::time::Instant::now();
    while start_time.elapsed() < Duration::from_secs(10) {
        let params = ("org.anyrun.anyrun",).to_variant();
        if dbus_conn.call_sync(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "GetNameOwner",
            Some(&params),
            None,
            gio::DBusCallFlags::NO_AUTO_START,
            100,
            None::<&gio::Cancellable>,
        ).is_ok() {
            registered = true;
            break;
        }
        
        // Ensure daemon hasn't crashed in the meantime
        if let Ok(Some(status)) = daemon_child.try_wait() {
            panic!("Daemon exited early with status: {} while waiting to register", status);
        }
        
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(registered, "Daemon failed to register its D-Bus name within 10 seconds");

    // 4. Start the client under the private DBus address to trigger show
    let mut client_child = Command::new(&bin)
        .arg("--config-dir")
        .arg(&config_dir)
        .env("DBUS_SESSION_BUS_ADDRESS", &dbus_address)
        .env("GSK_RENDERER", "cairo")
        .env("GDK_BACKEND", "broadway")
        .env("NO_AT_BRIDGE", "1")
        .env("GTK_A11Y", "none")
        .spawn()
        .expect("Failed to spawn anyrun client");

    // Wait for the client to connect and communicate
    std::thread::sleep(Duration::from_millis(1000));

    // 5. Send quit command via a new client invocation to shutdown daemon cleanly
    let quit_child = Command::new(&bin)
        .arg("--config-dir")
        .arg(&config_dir)
        .arg("quit")
        .env("DBUS_SESSION_BUS_ADDRESS", &dbus_address)
        .env("GSK_RENDERER", "cairo")
        .env("GDK_BACKEND", "broadway")
        .env("NO_AT_BRIDGE", "1")
        .env("GTK_A11Y", "none")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to run anyrun quit");

    let quit_status = quit_child.wait_with_output().expect("Failed to wait for quit command");

    assert!(
        quit_status.status.success(),
        "anyrun quit command failed - stdout: {} - stderr: {}",
        String::from_utf8_lossy(&quit_status.stdout),
        String::from_utf8_lossy(&quit_status.stderr)
    );

    // 6. Verify daemon exits cleanly
    let daemon_status = daemon_child
        .wait_with_timeout(Duration::from_secs(5))
        .expect("Daemon failed to exit in time");
    
    if daemon_status.is_none() {
        panic!("Daemon timed out after quit signal");
    }

    // 7. Verify client also exits
    let client_status = client_child
        .wait_with_timeout(Duration::from_secs(5))
        .expect("Client failed to exit in time");

    assert!(client_status.is_some(), "Client timed out after daemon quit");

    // Cleanup
    let _ = dbus_child.kill();
    let _ = fs::remove_dir_all(config_dir);
}

trait WaitTimeout {
    fn wait_with_timeout(&mut self, timeout: Duration) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl WaitTimeout for Child {
    fn wait_with_timeout(&mut self, timeout: Duration) -> std::io::Result<Option<std::process::ExitStatus>> {
        let start = std::time::Instant::now();
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(Some(status));
            }
            if start.elapsed() >= timeout {
                return Ok(None);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
