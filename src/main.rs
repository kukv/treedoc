use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "treedoc", about = "Tree with comments", version)]
struct Cli {
    #[arg(default_value = ".")]
    path: PathBuf,
}

struct Node {
    name: String,
    is_dir: bool,
    children: Vec<Node>,
}

fn build(path: &Path) -> io::Result<Node> {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let is_dir = path.is_dir();
    let mut children = Vec::new();
    if is_dir {
        for entry in fs::read_dir(path)? {
            children.push(build(&entry?.path())?);
        }
        children.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    }
    Ok(Node {
        name,
        is_dir,
        children,
    })
}

fn display_name(node: &Node) -> String {
    if node.is_dir {
        format!("{}/", node.name)
    } else {
        node.name.clone()
    }
}

fn render(node: &Node) {
    println!("{}", display_name(node));
    let mut parent_last = Vec::new();
    render_children(&node.children, &mut parent_last);
}

fn render_children(children: &[Node], parent_last: &mut Vec<bool>) {
    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        for &p in parent_last.iter() {
            print!("{}", if p { "    " } else { "│   " });
        }
        print!("{}", if is_last { "└── " } else { "├── " });
        println!("{}", display_name(child));
        parent_last.push(is_last);
        render_children(&child.children, parent_last);
        parent_last.pop();
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let root = build(&cli.path)?;
    render(&root);
    Ok(())
}
