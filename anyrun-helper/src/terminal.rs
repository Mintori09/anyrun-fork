use std::process::Command;

pub fn get_available_terminal() -> Option<String> {
    let terminals = [
        "kitty",
        "alacritty",
        "konsole",
        "gnome-terminal",
        "st",
        "2term",
        "xterm",
    ];

    for term in terminals {
        if command_exists(term) {
            return Some(term.to_string());
        }
    }

    None
}

fn command_exists(cmd: &str) -> bool {
    #[cfg(unix)]
    let check_cmd = "which";
    #[cfg(windows)]
    let check_cmd = "where";

    Command::new(check_cmd)
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
