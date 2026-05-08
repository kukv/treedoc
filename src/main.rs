use std::fs;
use std::io::{self, Write};
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
        sort_children(&mut children);
    }
    Ok(Node {
        name,
        is_dir,
        children,
    })
}

fn sort_children(children: &mut [Node]) {
    children.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
}

fn display_name(node: &Node) -> String {
    if node.is_dir {
        format!("{}/", node.name)
    } else {
        node.name.clone()
    }
}

fn render<W: Write>(out: &mut W, node: &Node) -> io::Result<()> {
    writeln!(out, "{}", display_name(node))?;
    let mut parent_last = Vec::new();
    render_children(out, &node.children, &mut parent_last)
}

fn render_children<W: Write>(
    out: &mut W,
    children: &[Node],
    parent_last: &mut Vec<bool>,
) -> io::Result<()> {
    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        for &p in parent_last.iter() {
            write!(out, "{}", if p { "    " } else { "│   " })?;
        }
        write!(out, "{}", if is_last { "└── " } else { "├── " })?;
        writeln!(out, "{}", display_name(child))?;
        parent_last.push(is_last);
        render_children(out, &child.children, parent_last)?;
        parent_last.pop();
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let root = build(&cli.path)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    render(&mut out, &root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str, children: Vec<Node>) -> Node {
        Node {
            name: name.into(),
            is_dir: true,
            children,
        }
    }

    fn file(name: &str) -> Node {
        Node {
            name: name.into(),
            is_dir: false,
            children: vec![],
        }
    }

    fn render_to_string(node: &Node) -> String {
        let mut buf = Vec::new();
        render(&mut buf, node).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn display_name_appends_slash_for_directories() {
        assert_eq!(display_name(&dir("src", vec![])), "src/");
        assert_eq!(display_name(&file("README.md")), "README.md");
    }

    #[test]
    fn sort_children_puts_directories_first_then_alphabetical() {
        let mut children = vec![
            file("z.txt"),
            dir("b", vec![]),
            file("a.txt"),
            dir("a", vec![]),
        ];
        sort_children(&mut children);
        let order: Vec<&str> = children.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(order, vec!["a", "b", "a.txt", "z.txt"]);
    }

    #[test]
    fn render_single_file() {
        assert_eq!(render_to_string(&file("README.md")), "README.md\n");
    }

    #[test]
    fn render_empty_directory() {
        assert_eq!(render_to_string(&dir("empty", vec![])), "empty/\n");
    }

    #[test]
    fn render_nested_tree_uses_correct_box_drawing() {
        let tree = dir(
            "root",
            vec![
                dir("a", vec![dir("aa", vec![file("x.txt")]), file("y.txt")]),
                dir("b", vec![file("z.txt")]),
                file("root.md"),
            ],
        );
        let expected = "\
root/
├── a/
│   ├── aa/
│   │   └── x.txt
│   └── y.txt
├── b/
│   └── z.txt
└── root.md
";
        assert_eq!(render_to_string(&tree), expected);
    }

    #[test]
    fn render_last_child_uses_spaces_not_pipe() {
        let tree = dir(
            "root",
            vec![dir("only", vec![file("a.txt"), file("b.txt")])],
        );
        let expected = "\
root/
└── only/
    ├── a.txt
    └── b.txt
";
        assert_eq!(render_to_string(&tree), expected);
    }

    #[test]
    fn build_reads_real_directory() {
        let tmp = std::env::temp_dir().join(format!(
            "treedoc-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&tmp).unwrap();
        fs::create_dir(tmp.join("sub")).unwrap();
        fs::write(tmp.join("a.txt"), "").unwrap();
        fs::write(tmp.join("sub").join("b.txt"), "").unwrap();

        let node = build(&tmp).unwrap();
        let output = render_to_string(&node);

        let expected = format!(
            "\
{name}/
├── sub/
│   └── b.txt
└── a.txt
",
            name = tmp.file_name().unwrap().to_string_lossy()
        );
        assert_eq!(output, expected);

        fs::remove_dir_all(&tmp).unwrap();
    }
}
