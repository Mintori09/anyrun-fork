use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use crate::config::ConfigArgs;

#[derive(Parser, Clone, Debug, Serialize, Deserialize)]
#[command(version, about)]
pub struct Args {
    #[arg(short, long)]
    pub config_dir: Option<String>,
    #[command(flatten)]
    pub config: ConfigArgs,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Command {
    Daemon,
    Close,
    Quit,
    Reload,
}
