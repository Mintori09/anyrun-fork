use arboard::Clipboard;

pub mod icon;
pub mod log;
pub mod mazzy_matcher;

pub fn focus_to_class(class: &str) {
    let output = std::process::Command::new("kdotool")
        .args(["search", "--class", class])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);

            if let Some(window_id) = stdout.lines().next().map(|s| s.trim()) {
                if !window_id.is_empty() {
                    focus_to_window_by_id(window_id);
                }
            } else {
                eprintln!("No window found with class: '{}'", class);
            }
        }
        Ok(out) => {
            eprintln!(
                "kdotool search failed with exit code: {:?}",
                out.status.code()
            );
        }
        Err(e) => {
            eprintln!("Failed to execute kdotool. Is it installed? Error: {}", e);
        }
    }
}

pub fn focus_to_window_by_id(window_id: &str) {
    let activate_res = std::process::Command::new("kdotool")
        .args(["windowactivate", window_id])
        .spawn();

    if let Err(e) = activate_res {
        eprintln!("Failed to spawn kdotool activate: {}", e);
    }
}

pub fn get_clipboard() -> String {
    let mut clipboard = match Clipboard::new() {
        Ok(cb) => cb,
        Err(_) => return String::new(),
    };

    clipboard.get_text().unwrap_or_default()
}

pub fn set_clipboard(content: String) -> Result<(), arboard::Error> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(content)
}
