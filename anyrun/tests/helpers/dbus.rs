use std::io::BufRead;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Start a private dbus-daemon --session --print-address --nofork.
/// Returns (child_process, dbus_address).
pub fn start_private_dbus() -> (Child, String) {
    let mut child = Command::new("dbus-daemon")
        .args(["--session", "--print-address", "--nofork"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start dbus-daemon");

    let reader = std::io::BufReader::new(child.stdout.take().unwrap());
    let addr = reader.lines().next().unwrap().unwrap();
    (child, addr)
}

/// Wait for a D-Bus name to be registered.
pub async fn wait_for_bus_name(addr: &str, name: &str) {
    let start = Instant::now();
    loop {
        let output = Command::new("dbus-send")
            .args([
                &format!("--bus={}", addr),
                "--dest=org.freedesktop.DBus",
                "--print-reply",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus.NameHasOwner",
                &format!("string:{}", name),
            ])
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("true") {
            return;
        }

        assert!(
            start.elapsed() < Duration::from_secs(10),
            "Timeout waiting for bus name {}",
            name
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
