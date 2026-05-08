use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

const SIDECAR_FILENAME: &str = ".treedoc.yaml";
const COMMENT_MARGIN: usize = 2;

#[derive(Parser, Debug)]
#[command(name = "treedoc", about = "Tree with comments", version)]
struct Cli {
    #[arg(default_value = ".")]
    path: PathBuf,
}

struct Node {
    name: String,
    is_dir: bool,
    rel_path: String,
    children: Vec<Node>,
}

#[derive(Default)]
struct Comments(HashMap<String, String>);

impl Comments {
    fn load(root: &Path) -> Result<Self> {
        let path = root.join(SIDECAR_FILENAME);
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let raw: HashMap<String, String> = serde_yaml::from_str(&text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        let normalized = raw
            .into_iter()
            .map(|(k, v)| (normalize_key(&k), v))
            .collect();
        Ok(Self(normalized))
    }

    fn get(&self, rel_path: &str) -> Option<&str> {
        self.0.get(rel_path).map(String::as_str)
    }
}

fn normalize_key(k: &str) -> String {
    k.trim_end_matches('/').to_string()
}

fn build(path: &Path) -> io::Result<Node> {
    build_with_rel(path, String::new())
}

fn build_with_rel(path: &Path, rel: String) -> io::Result<Node> {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let is_dir = path.is_dir();
    let mut children = Vec::new();
    if is_dir {
        for entry in fs::read_dir(path)? {
            let child_path = entry?.path();
            let child_name = child_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if child_name == SIDECAR_FILENAME && rel.is_empty() {
                continue;
            }
            let child_rel = if rel.is_empty() {
                child_name
            } else {
                format!("{}/{}", rel, child_name)
            };
            children.push(build_with_rel(&child_path, child_rel)?);
        }
        sort_children(&mut children);
    }
    Ok(Node {
        name,
        is_dir,
        rel_path: rel,
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

fn render<W: Write>(out: &mut W, node: &Node, comments: &Comments) -> io::Result<()> {
    let mut lines: Vec<(String, Option<String>)> = Vec::new();
    lines.push((
        display_name(node),
        comments.get(&node.rel_path).map(String::from),
    ));
    let mut parent_last = Vec::new();
    collect_lines(&node.children, &mut parent_last, comments, &mut lines);

    let max_width = lines
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);

    for (label, comment) in lines {
        match comment {
            Some(c) => {
                let pad = max_width.saturating_sub(label.chars().count()) + COMMENT_MARGIN;
                writeln!(out, "{}{}# {}", label, " ".repeat(pad), c)?;
            }
            None => writeln!(out, "{}", label)?,
        }
    }
    Ok(())
}

fn collect_lines(
    children: &[Node],
    parent_last: &mut Vec<bool>,
    comments: &Comments,
    out: &mut Vec<(String, Option<String>)>,
) {
    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        let mut line = String::new();
        for &p in parent_last.iter() {
            line.push_str(if p { "    " } else { "│   " });
        }
        line.push_str(if is_last { "└── " } else { "├── " });
        line.push_str(&display_name(child));
        let comment = comments.get(&child.rel_path).map(String::from);
        out.push((line, comment));
        parent_last.push(is_last);
        collect_lines(&child.children, parent_last, comments, out);
        parent_last.pop();
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dir_at(name: &str, rel: &str, children: Vec<Node>) -> Node {
        Node {
            name: name.into(),
            is_dir: true,
            rel_path: rel.into(),
            children,
        }
    }

    fn file_at(name: &str, rel: &str) -> Node {
        Node {
            name: name.into(),
            is_dir: false,
            rel_path: rel.into(),
            children: vec![],
        }
    }

    fn dir(name: &str, children: Vec<Node>) -> Node {
        dir_at(name, name, children)
    }

    fn file(name: &str) -> Node {
        file_at(name, name)
    }

    fn render_to_string(node: &Node, comments: &Comments) -> String {
        let mut buf = Vec::new();
        render(&mut buf, node, comments).unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn empty_comments() -> Comments {
        Comments::default()
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
    fn normalize_key_strips_trailing_slash() {
        assert_eq!(normalize_key("src/"), "src");
        assert_eq!(normalize_key("src"), "src");
        assert_eq!(normalize_key("src/main.rs"), "src/main.rs");
    }

    #[test]
    fn render_without_comments_matches_phase2_output() {
        let tree = dir(
            "root",
            vec![
                dir_at(
                    "a",
                    "a",
                    vec![
                        dir_at("aa", "a/aa", vec![file_at("x.txt", "a/aa/x.txt")]),
                        file_at("y.txt", "a/y.txt"),
                    ],
                ),
                dir_at("b", "b", vec![file_at("z.txt", "b/z.txt")]),
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
        assert_eq!(render_to_string(&tree, &empty_comments()), expected);
    }

    #[test]
    fn render_aligns_comments() {
        let tree = dir(
            "root",
            vec![
                dir_at("src", "src", vec![file_at("main.rs", "src/main.rs")]),
                file("README.md"),
            ],
        );
        let mut map = HashMap::new();
        map.insert("src".into(), "ソース".into());
        map.insert("src/main.rs".into(), "エントリ".into());
        map.insert("README.md".into(), "ドキュメント".into());
        let comments = Comments(map);

        let output = render_to_string(&tree, &comments);
        // Each line has either a comment (padded to max+2) or no comment.
        // Max label width: "├── src/" = 8 chars, "│   └── main.rs" = 15, "└── README.md" = 13.
        let expected = "\
root/
├── src/         # ソース
│   └── main.rs  # エントリ
└── README.md    # ドキュメント
";
        assert_eq!(output, expected);
    }

    #[test]
    fn comments_load_missing_file_returns_empty() {
        let tmp = unique_tmp();
        fs::create_dir(&tmp).unwrap();
        let comments = Comments::load(&tmp).unwrap();
        assert!(comments.get("anything").is_none());
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn comments_load_parses_yaml_and_normalizes_keys() {
        let tmp = unique_tmp();
        fs::create_dir(&tmp).unwrap();
        fs::write(
            tmp.join(SIDECAR_FILENAME),
            "src/: \"ソース\"\nsrc/main.rs: \"エントリ\"\n",
        )
        .unwrap();
        let comments = Comments::load(&tmp).unwrap();
        assert_eq!(comments.get("src"), Some("ソース"));
        assert_eq!(comments.get("src/main.rs"), Some("エントリ"));
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn comments_load_invalid_yaml_returns_error() {
        let tmp = unique_tmp();
        fs::create_dir(&tmp).unwrap();
        fs::write(tmp.join(SIDECAR_FILENAME), "this: is: not: valid").unwrap();
        let result = Comments::load(&tmp);
        assert!(result.is_err());
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn build_excludes_sidecar_file_at_root() {
        let tmp = unique_tmp();
        fs::create_dir(&tmp).unwrap();
        fs::write(tmp.join(SIDECAR_FILENAME), "{}").unwrap();
        fs::write(tmp.join("README.md"), "").unwrap();
        let node = build(&tmp).unwrap();
        let names: Vec<&str> = node.children.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, vec!["README.md"]);
        fs::remove_dir_all(&tmp).unwrap();
    }

    fn unique_tmp() -> PathBuf {
        std::env::temp_dir().join(format!(
            "treedoc-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
