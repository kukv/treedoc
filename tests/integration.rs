use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use treedoc::{build, render, Comments, Node, SIDECAR_FILENAME};

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

#[test]
fn render_single_file() {
    assert_eq!(
        render_to_string(&file("README.md"), &Comments::default()),
        "README.md\n"
    );
}

#[test]
fn render_empty_directory() {
    assert_eq!(
        render_to_string(&dir("empty", vec![]), &Comments::default()),
        "empty/\n"
    );
}

#[test]
fn render_nested_tree_uses_correct_box_drawing() {
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
    assert_eq!(render_to_string(&tree, &Comments::default()), expected);
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
    assert_eq!(render_to_string(&tree, &Comments::default()), expected);
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

    let expected = "\
root/
├── src/         # ソース
│   └── main.rs  # エントリ
└── README.md    # ドキュメント
";
    assert_eq!(render_to_string(&tree, &comments), expected);
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
fn comments_load_parses_yaml_and_normalizes_trailing_slash() {
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

#[test]
fn build_sorts_directories_first_then_alphabetical() {
    let tmp = unique_tmp();
    fs::create_dir(&tmp).unwrap();
    fs::write(tmp.join("z.txt"), "").unwrap();
    fs::write(tmp.join("a.txt"), "").unwrap();
    fs::create_dir(tmp.join("b")).unwrap();
    fs::create_dir(tmp.join("a")).unwrap();
    let node = build(&tmp).unwrap();
    let names: Vec<&str> = node.children.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "a.txt", "z.txt"]);
    fs::remove_dir_all(&tmp).unwrap();
}
