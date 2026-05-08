use std::path::PathBuf;

use clap::Parser;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "treedoc", about = "Tree with comments", version)]
struct Cli {
    #[arg(default_value = ".")]
    path: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    for entry in WalkDir::new(&cli.path).into_iter().filter_map(Result::ok) {
        println!("{}", entry.path().display());
    }
}
