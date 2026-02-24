use std::env;
use std::process::Command;

const PREFERRED_TERMINALS: &[&str] = &[
    "kitty",
    "alacritty",
    "wezterm",
    "konsole",
    "gnome-terminal",
    "gnome-terminal",
    "st",
    "2term",
    "xterm",
    "konsole",
];

pub fn get_available_terminal() -> Option<String> {
    PREFERRED_TERMINALS
        .iter()
        .find(|&&terminal| is_program_in_path(terminal))
        .map(|&terminal| terminal.to_string())
}

// fn command_exists(cmd: &str) -> bool {
//     #[cfg(unix)]
//     let check_cmd = "which";
//     #[cfg(windows)]
//     let check_cmd = "where";

//     Command::new(check_cmd)
//         .arg(cmd)
//         .output()
//         .map(|output| output.status.success())
//         .unwrap_or(false)
// }

fn is_program_in_path(program: &str) -> bool {
    let path_var = match env::var_os("PATH") {
        Some(paths) => paths,
        None => return false,
    };

    env::split_paths(&path_var).any(|dir| {
        let full_path = dir.join(program);
        full_path.exists()
    })
}

pub fn configure_terminal_environment(command: &mut Command) {
    command.env(
        "FREETYPE_PROPERTIES",
        "autofitter:no-stem-darkening=1 cff:no-stem-darkening=1",
    );
}

fn launch_in_terminal(terminal: &str, shell_command: &str) {
    let mut process = Command::new(terminal);

    configure_terminal_environment(&mut process);

    process
        .arg("sh")
        .arg("-c")
        .arg(shell_command)
        .spawn()
        .expect("Failed to launch terminal process");
}

pub fn launch(command: &str) {
    match get_available_terminal() {
        Some(terminal) => {
            println!("Launching with: {}", terminal);
            launch_in_terminal(&terminal, command);
        }
        None => {
            eprintln!("No supported terminal found in PATH");
        }
    }
}
