use std::io::{self, IsTerminal};
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use treedoc::{build, render, Comments, RenderOptions, WalkOptions};

#[derive(Parser, Debug)]
#[command(name = "treedoc", about = "Tree with comments", version)]
struct Cli {
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Descend at most this many levels of directories.
    #[arg(short = 'L', long)]
    depth: Option<usize>,

    /// Show entries whose names begin with a dot.
    #[arg(short = 'a', long)]
    all: bool,

    /// Do not honour .gitignore rules.
    #[arg(long)]
    no_ignore: bool,

    /// Disable colored output.
    #[arg(long)]
    no_color: bool,
}

fn should_use_color(no_color_flag: bool) -> bool {
    if no_color_flag {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    io::stdout().is_terminal()
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let walk = WalkOptions {
        show_hidden: cli.all,
        use_gitignore: !cli.no_ignore,
        max_depth: cli.depth,
    };
    let render_opts = RenderOptions {
        color: should_use_color(cli.no_color),
    };
    let root = build(&cli.path, &walk)?;
    let comments = Comments::load(&cli.path)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    render(&mut out, &root, &comments, &render_opts)?;
    Ok(())
}
