use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result};

pub const SIDECAR_FILENAME: &str = ".treedoc.yaml";
const COMMENT_MARGIN: usize = 2;

pub struct Node {
    pub name: String,
    pub is_dir: bool,
    pub rel_path: String,
    pub children: Vec<Node>,
}

#[derive(Default)]
pub struct Comments(pub HashMap<String, String>);

impl Comments {
    pub fn load(root: &Path) -> Result<Self> {
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

    pub fn get(&self, rel_path: &str) -> Option<&str> {
        self.0.get(rel_path).map(String::as_str)
    }
}

fn normalize_key(k: &str) -> String {
    k.trim_end_matches('/').to_string()
}

pub fn build(path: &Path) -> io::Result<Node> {
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

pub fn render<W: Write>(out: &mut W, node: &Node, comments: &Comments) -> io::Result<()> {
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
