use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use treedoc::{
    build, init_comments, render, Comments, RenderOptions, WalkOptions, SIDECAR_FILENAME,
};

#[derive(Parser, Debug)]
#[command(name = "treedoc", version, about = "Tree with comments")]
#[command(args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<CommandKind>,

    #[command(flatten)]
    show: ShowArgs,
}

#[derive(Subcommand, Debug)]
enum CommandKind {
    /// Render the tree (default).
    Show(ShowArgs),
    /// Create a `.treedoc.yaml` populated with empty comments for every entry.
    Init(InitArgs),
    /// Set or update a comment for a single path.
    Set(SetArgs),
    /// Open `.treedoc.yaml` in $EDITOR.
    Edit(EditArgs),
}

#[derive(Args, Debug, Clone)]
struct ShowArgs {
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

    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Console)]
    format: Format,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Format {
    /// Colored tree intended for the terminal (default).
    Console,
    /// Plain ASCII tree without colors.
    Plain,
    /// Tree wrapped in a Markdown code fence.
    Markdown,
}

#[derive(Args, Debug)]
struct InitArgs {
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Overwrite an existing .treedoc.yaml.
    #[arg(short = 'f', long)]
    force: bool,

    /// Include dotfiles when scanning.
    #[arg(short = 'a', long)]
    all: bool,

    /// Ignore .gitignore when scanning.
    #[arg(long)]
    no_ignore: bool,
}

#[derive(Args, Debug)]
struct SetArgs {
    /// Relative path inside the tree (e.g. `src/main.rs`).
    target: String,
    /// Comment text.
    comment: String,
    /// Tree root.
    #[arg(short = 'C', long, default_value = ".")]
    path: PathBuf,
}

#[derive(Args, Debug)]
struct EditArgs {
    /// Tree root.
    #[arg(default_value = ".")]
    path: PathBuf,
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

fn run_show(args: ShowArgs) -> Result<()> {
    let walk = WalkOptions {
        show_hidden: args.all,
        use_gitignore: !args.no_ignore,
        max_depth: args.depth,
    };
    let color = match args.format {
        Format::Console => should_use_color(args.no_color),
        Format::Plain | Format::Markdown => false,
    };
    let root = build(&args.path, &walk)?;
    let comments = Comments::load(&args.path)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if matches!(args.format, Format::Markdown) {
        writeln!(out, "```")?;
    }
    render(&mut out, &root, &comments, &RenderOptions { color })?;
    if matches!(args.format, Format::Markdown) {
        writeln!(out, "```")?;
    }
    Ok(())
}

fn run_init(args: InitArgs) -> Result<()> {
    let target = args.path.join(SIDECAR_FILENAME);
    if target.exists() && !args.force {
        bail!(
            "{} already exists; use --force to overwrite",
            target.display()
        );
    }
    let walk = WalkOptions {
        show_hidden: args.all,
        use_gitignore: !args.no_ignore,
        max_depth: None,
    };
    let comments = init_comments(&args.path, &walk)?;
    comments.save(&args.path)?;
    eprintln!("wrote {}", target.display());
    Ok(())
}

fn run_set(args: SetArgs) -> Result<()> {
    let mut comments = Comments::load(&args.path)?;
    comments.set(&args.target, args.comment);
    comments.save(&args.path)?;
    Ok(())
}

fn run_edit(args: EditArgs) -> Result<()> {
    let yaml_path = args.path.join(SIDECAR_FILENAME);
    if !yaml_path.exists() {
        std::fs::write(&yaml_path, "{}\n")
            .with_context(|| format!("failed to create {}", yaml_path.display()))?;
    }
    spawn_editor(&yaml_path)?;
    // Validate by reparsing.
    Comments::load(&args.path).with_context(|| {
        format!(
            "{} is no longer valid YAML after editing",
            yaml_path.display()
        )
    })?;
    Ok(())
}

fn spawn_editor(file: &Path) -> Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
        if cfg!(windows) {
            "notepad".to_string()
        } else {
            "vi".to_string()
        }
    });
    let mut parts = editor.split_whitespace();
    let prog = parts.next().context("EDITOR is empty")?;
    let mut cmd = Command::new(prog);
    for arg in parts {
        cmd.arg(arg);
    }
    cmd.arg(file);
    let status = cmd
        .status()
        .with_context(|| format!("failed to spawn editor `{}`", editor))?;
    if !status.success() {
        bail!("editor `{}` exited with {}", editor, status);
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => run_show(cli.show),
        Some(CommandKind::Show(args)) => run_show(args),
        Some(CommandKind::Init(args)) => run_init(args),
        Some(CommandKind::Set(args)) => run_set(args),
        Some(CommandKind::Edit(args)) => run_edit(args),
    }
}
