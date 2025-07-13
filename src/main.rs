mod cli;
mod core;
mod fs;
mod error;

use clap::Parser;
use cli::args::Args;
use core::explorer::Explorer;
use error::ExplorerError;

fn main() -> Result<(), ExplorerError> {
    let args = Args::try_parse();
    let explorer = Explorer::new(args.unwrap().directory)?;
    explorer.run()
}
