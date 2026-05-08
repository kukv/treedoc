use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

pub const SIDECAR_FILENAME: &str = ".treedoc.yaml";
const GITIGNORE_FILENAME: &str = ".gitignore";
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

#[derive(Clone, Debug)]
pub struct WalkOptions {
    pub show_hidden: bool,
    pub use_gitignore: bool,
    pub max_depth: Option<usize>,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            show_hidden: false,
            use_gitignore: true,
            max_depth: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RenderOptions {
    pub color: bool,
}

pub fn build(root: &Path, opts: &WalkOptions) -> Result<Node> {
    let gitignore = if opts.use_gitignore {
        let mut builder = GitignoreBuilder::new(root);
        let gi = root.join(GITIGNORE_FILENAME);
        if gi.exists() {
            if let Some(err) = builder.add(&gi) {
                return Err(err.into());
            }
        }
        builder
            .build()
            .with_context(|| format!("failed to build gitignore matcher at {}", root.display()))?
    } else {
        Gitignore::empty()
    };
    build_recurse(root, "", 0, opts, &gitignore)
}

fn build_recurse(
    path: &Path,
    rel: &str,
    depth: usize,
    opts: &WalkOptions,
    gitignore: &Gitignore,
) -> Result<Node> {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let is_dir = path.is_dir();
    let mut children = Vec::new();

    let descend = is_dir && opts.max_depth.is_none_or(|max| depth < max);
    if descend {
        for entry in fs::read_dir(path)? {
            let child_path = entry?.path();
            let child_name = child_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();

            if rel.is_empty() && child_name == SIDECAR_FILENAME {
                continue;
            }
            if !opts.show_hidden && child_name.starts_with('.') {
                continue;
            }
            let child_is_dir = child_path.is_dir();
            if opts.use_gitignore && gitignore.matched(&child_path, child_is_dir).is_ignore() {
                continue;
            }

            let child_rel = if rel.is_empty() {
                child_name
            } else {
                format!("{}/{}", rel, child_name)
            };
            children.push(build_recurse(
                &child_path,
                &child_rel,
                depth + 1,
                opts,
                gitignore,
            )?);
        }
        sort_children(&mut children);
    }

    Ok(Node {
        name,
        is_dir,
        rel_path: rel.to_string(),
        children,
    })
}

fn sort_children(children: &mut [Node]) {
    children.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
}

struct Line {
    prefix: String,
    name: String,
    is_dir: bool,
    comment: Option<String>,
}

impl Line {
    fn plain_width(&self) -> usize {
        let suffix = if self.is_dir { 1 } else { 0 };
        self.prefix.chars().count() + self.name.chars().count() + suffix
    }
}

pub fn render<W: Write>(
    out: &mut W,
    node: &Node,
    comments: &Comments,
    opts: &RenderOptions,
) -> io::Result<()> {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line {
        prefix: String::new(),
        name: node.name.clone(),
        is_dir: node.is_dir,
        comment: comments.get(&node.rel_path).map(String::from),
    });
    let mut parent_last = Vec::new();
    collect_lines(&node.children, &mut parent_last, comments, &mut lines);

    let max_width = lines.iter().map(Line::plain_width).max().unwrap_or(0);
    for line in &lines {
        emit_line(out, line, max_width, opts.color)?;
    }
    Ok(())
}

fn emit_line<W: Write>(out: &mut W, line: &Line, max_width: usize, color: bool) -> io::Result<()> {
    let suffix = if line.is_dir { "/" } else { "" };
    let display_name = format!("{}{}", line.name, suffix);

    if color {
        write!(out, "{}", line.prefix.bright_black())?;
        if line.is_dir {
            write!(out, "{}", display_name.blue().bold())?;
        } else {
            write!(out, "{}", display_name)?;
        }
    } else {
        write!(out, "{}{}", line.prefix, display_name)?;
    }

    if let Some(comment) = &line.comment {
        let pad = max_width.saturating_sub(line.plain_width()) + COMMENT_MARGIN;
        let tail = format!("{}# {}", " ".repeat(pad), comment);
        if color {
            write!(out, "{}", tail.bright_black())?;
        } else {
            write!(out, "{}", tail)?;
        }
    }
    writeln!(out)
}

fn collect_lines(
    children: &[Node],
    parent_last: &mut Vec<bool>,
    comments: &Comments,
    out: &mut Vec<Line>,
) {
    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        let mut prefix = String::new();
        for &p in parent_last.iter() {
            prefix.push_str(if p { "    " } else { "│   " });
        }
        prefix.push_str(if is_last { "└── " } else { "├── " });
        out.push(Line {
            prefix,
            name: child.name.clone(),
            is_dir: child.is_dir,
            comment: comments.get(&child.rel_path).map(String::from),
        });
        parent_last.push(is_last);
        collect_lines(&child.children, parent_last, comments, out);
        parent_last.pop();
    }
}
