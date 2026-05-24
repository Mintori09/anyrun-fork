use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_helper::icon::SystemIcon;
use anyrun_helper::terminal;
use anyrun_plugin::*;
use serde::Deserialize;

use std::process::Command;
use std::{fs, path::PathBuf};

#[derive(Deserialize, Debug)]
struct Scope {
    prefix: String,
    description: String,

    /// Command template.
    ///
    /// Use `{}` as the placeholder for the user query.
    /// Example:
    /// command: "firefox https://google.com/search?q={}"
    ///
    /// The query will be shell-escaped automatically.
    command: String,
}

#[derive(Deserialize, Debug, Default)]
struct Config {
    #[serde(default)]
    scopes: Vec<Scope>,
}

pub struct State {
    config: Config,
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = PathBuf::from(config_dir.to_string()).join("shell_wrapper_once.ron");

    let config = match fs::read_to_string(&config_path) {
        Ok(content) => match ron::from_str::<Config>(&content) {
            Ok(config) => config,
            Err(error) => {
                notify_error(
                    "Shell Wrapper Once",
                    &format!("Failed to parse config:\n{}", error),
                );
                Config::default()
            }
        },
        Err(error) => {
            notify_error(
                "Shell Wrapper Once",
                &format!(
                    "Failed to read config:\n{}\n\nPath: {}",
                    error,
                    config_path.display()
                ),
            );
            Config::default()
        }
    };

    State { config }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Shell Wrapper Once".into(),
        icon: "utilities-terminal-symbolic".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let input_str = input.to_string();

    for (index, scope) in state.config.scopes.iter().enumerate() {
        if let Some(query) = strip_prefix_case_insensitive(&input_str, &scope.prefix) {
            let query = query.trim();

            if query.is_empty() {
                return RVec::new();
            }

            let item = Match {
                title: query.into(),
                description: ROption::RSome(scope.description.clone().into()),
                id: ROption::RSome(index as u64),
                icon: ROption::RSome(SystemIcon::Settings.as_str().into()),
                use_pango: false,
            };

            let mut items = RVec::new();
            items.push(item);
            return items;
        }
    }

    RVec::new()
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    let ROption::RSome(id) = selection.id else {
        notify_error("Shell Wrapper Once", "Missing selection id.");
        return HandleResult::Close;
    };

    let Ok(index) = id.to_string().parse::<usize>() else {
        notify_error(
            "Shell Wrapper Once",
            &format!("Invalid selection id: {}", id),
        );
        return HandleResult::Close;
    };

    let Some(scope) = state.config.scopes.get(index) else {
        notify_error(
            "Shell Wrapper Once",
            &format!("Scope index out of range: {}", index),
        );
        return HandleResult::Close;
    };

    let query = selection.title.to_string();
    let escaped_query = shell_escape_single_arg(&query);

    let cmd = if scope.command.contains("{}") {
        scope.command.replace("{}", &escaped_query)
    } else {
        format!("{} {}", scope.command, escaped_query)
    };

    if let Err(error) = execute_command(&cmd) {
        notify_error(
            "Shell Wrapper Once",
            &format!("Failed to spawn command:\n{}\n\nCommand:\n{}", error, cmd),
        );
    }

    HandleResult::Close
}

fn strip_prefix_case_insensitive<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    let input_prefix = input.get(..prefix.len())?;

    if input_prefix.eq_ignore_ascii_case(prefix) {
        input.get(prefix.len()..)
    } else {
        None
    }
}

fn shell_escape_single_arg(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    let mut escaped = String::from("'");

    for ch in value.chars() {
        if ch == '\'' {
            escaped.push_str("'\\''");
        } else {
            escaped.push(ch);
        }
    }

    escaped.push('\'');

    escaped
}

fn execute_command(cmd: &str) -> std::io::Result<()> {
    let mut command = Command::new("sh");
    terminal::configure_terminal_environment(&mut command);

    anyrun_plugin::spawn_detached(command.arg("-c").arg(cmd))?;
    Ok(())
}

fn notify_error(title: &str, message: &str) {
    let _ = Command::new("notify-send")
        .arg("-u")
        .arg("critical")
        .arg(title)
        .arg(message)
        .spawn();
}
