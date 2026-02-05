use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "turfops", version, about = "Lawn care management TUI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to config.yaml
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Override SQLite data directory
    #[arg(short, long)]
    pub data_dir: Option<PathBuf>,

    /// Increase log verbosity (-v for debug, -vv for trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Re-run interactive setup
    Init,
    /// Validate config and test connections
    Check,
}
