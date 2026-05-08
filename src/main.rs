use std::io;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use treedoc::{build, render, Comments};

#[derive(Parser, Debug)]
#[command(name = "treedoc", about = "Tree with comments", version)]
struct Cli {
    #[arg(default_value = ".")]
    path: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = build(&cli.path)?;
    let comments = Comments::load(&cli.path)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    render(&mut out, &root, &comments)?;
    Ok(())
}
