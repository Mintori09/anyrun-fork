use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

pub fn log_to_desktop(msg: &str) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let log_path = std::path::PathBuf::from(home).join("Desktop/log.txt");

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}
