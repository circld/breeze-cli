use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "breeze")]
#[command(about = "A terminal-based file explorer")]
pub struct Args {
    /// Directory to explore
    #[arg(default_value = ".")]
    pub directory: PathBuf,

    /// Show hidden files
    #[arg(short, long)]
    pub all: bool,
}
