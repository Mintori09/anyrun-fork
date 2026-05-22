use clap::Parser;
use std::env;

mod app;
mod args;
mod client;
mod config;
mod daemon;
mod dbus;
mod doctor;
mod plugin_box;
mod provider;

use crate::args::{Args, Command};
use crate::client::run_client;
use crate::daemon::run_daemon;
use crate::dbus::fast_ipc_call;

fn main() {
    if env::var_os("GSK_RENDERER").is_none() {
        unsafe {
            env::set_var("GSK_RENDERER", "gl");
        }
    }
    let args = Args::parse();

    if let Some(cmd) = args.command {
        match cmd {
            Command::Close | Command::Quit => {
                fast_ipc_call(if matches!(cmd, Command::Close) {
                    "Close"
                } else {
                    "Quit"
                });
                return;
            }
            Command::Daemon => {
                run_daemon(args);
                return;
            }
            Command::Reload => {
                fast_ipc_call("Reload");
                return;
            }
            Command::Doctor => {
                std::process::exit(doctor::run(args.config_dir.as_deref()));
            }
        }
    }
    run_client(args);
}
