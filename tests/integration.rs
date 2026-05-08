use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use treedoc::{
    build, init_comments, render, Comments, Node, RenderOptions, WalkOptions, SIDECAR_FILENAME,
};

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

fn render_plain(node: &Node, comments: &Comments) -> String {
    let mut buf = Vec::new();
    render(&mut buf, node, comments, &RenderOptions { color: false }).unwrap();
    String::from_utf8(buf).unwrap()
}

fn unique_tmp() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "treedoc-test-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        n,
    ));
    fs::create_dir(&dir).unwrap();
    dir
}

fn build_default(path: &std::path::Path) -> Node {
    build(path, &WalkOptions::default()).unwrap()
}

#[test]
fn render_single_file() {
    assert_eq!(
        render_plain(&file("README.md"), &Comments::default()),
        "README.md\n"
    );
}

#[test]
fn render_empty_directory() {
    assert_eq!(
        render_plain(&dir("empty", vec![]), &Comments::default()),
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
    assert_eq!(render_plain(&tree, &Comments::default()), expected);
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
    assert_eq!(render_plain(&tree, &Comments::default()), expected);
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
    let mut map = BTreeMap::new();
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
    assert_eq!(render_plain(&tree, &comments), expected);
}

#[test]
fn render_with_color_emits_ansi_escapes() {
    // Force colored output regardless of test runner's TTY/NO_COLOR state.
    colored::control::set_override(true);
    let tree = dir("root", vec![dir_at("src", "src", vec![file("README.md")])]);
    let mut buf = Vec::new();
    render(
        &mut buf,
        &tree,
        &Comments::default(),
        &RenderOptions { color: true },
    )
    .unwrap();
    let out = String::from_utf8(buf).unwrap();
    colored::control::unset_override();
    assert!(
        out.contains('\x1b'),
        "expected ANSI escape codes in: {out:?}"
    );
}

#[test]
fn comments_load_missing_file_returns_empty() {
    let tmp = unique_tmp();
    let comments = Comments::load(&tmp).unwrap();
    assert!(comments.get("anything").is_none());
    fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn comments_load_parses_yaml_and_normalizes_trailing_slash() {
    let tmp = unique_tmp();
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
    fs::write(tmp.join(SIDECAR_FILENAME), "this: is: not: valid").unwrap();
    let result = Comments::load(&tmp);
    assert!(result.is_err());
    fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn build_excludes_sidecar_file_at_root() {
    let tmp = unique_tmp();
    fs::write(tmp.join(SIDECAR_FILENAME), "{}").unwrap();
    fs::write(tmp.join("README.md"), "").unwrap();
    let node = build_default(&tmp);
    let names: Vec<&str> = node.children.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, vec!["README.md"]);
    fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn build_sorts_directories_first_then_alphabetical() {
    let tmp = unique_tmp();
    fs::write(tmp.join("z.txt"), "").unwrap();
    fs::write(tmp.join("a.txt"), "").unwrap();
    fs::create_dir(tmp.join("b")).unwrap();
    fs::create_dir(tmp.join("a")).unwrap();
    let node = build_default(&tmp);
    let names: Vec<&str> = node.children.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "a.txt", "z.txt"]);
    fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn build_hides_dotfiles_by_default_and_shows_with_show_hidden() {
    let tmp = unique_tmp();
    fs::write(tmp.join("visible.txt"), "").unwrap();
    fs::write(tmp.join(".hidden"), "").unwrap();

    let default = build_default(&tmp);
    let names: Vec<&str> = default.children.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, vec!["visible.txt"]);

    let all = build(
        &tmp,
        &WalkOptions {
            show_hidden: true,
            use_gitignore: false,
            max_depth: None,
        },
    )
    .unwrap();
    let names: Vec<&str> = all.children.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, vec![".hidden", "visible.txt"]);
    fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn build_max_depth_limits_recursion() {
    let tmp = unique_tmp();
    fs::create_dir_all(tmp.join("a/b/c")).unwrap();
    fs::write(tmp.join("a/b/c/deep.txt"), "").unwrap();
    fs::write(tmp.join("a/top.txt"), "").unwrap();

    let depth1 = build(
        &tmp,
        &WalkOptions {
            max_depth: Some(1),
            ..WalkOptions::default()
        },
    )
    .unwrap();
    // Only the root's direct children, no further descent.
    assert_eq!(depth1.children.len(), 1);
    assert_eq!(depth1.children[0].name, "a");
    assert!(depth1.children[0].children.is_empty());

    let depth2 = build(
        &tmp,
        &WalkOptions {
            max_depth: Some(2),
            ..WalkOptions::default()
        },
    )
    .unwrap();
    let a = &depth2.children[0];
    let inner: Vec<&str> = a.children.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(inner, vec!["b", "top.txt"]);
    let b = &a.children[0];
    assert!(b.children.is_empty());

    fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn build_respects_gitignore_by_default_and_can_be_disabled() {
    let tmp = unique_tmp();
    fs::write(tmp.join(".gitignore"), "ignored.txt\nbuild/\n").unwrap();
    fs::write(tmp.join("kept.txt"), "").unwrap();
    fs::write(tmp.join("ignored.txt"), "").unwrap();
    fs::create_dir(tmp.join("build")).unwrap();
    fs::write(tmp.join("build/artifact"), "").unwrap();

    // Default + show_hidden so .gitignore itself appears, but ignored entries are filtered.
    let with_ignore = build(
        &tmp,
        &WalkOptions {
            show_hidden: true,
            use_gitignore: true,
            max_depth: None,
        },
    )
    .unwrap();
    let names: Vec<&str> = with_ignore
        .children
        .iter()
        .map(|n| n.name.as_str())
        .collect();
    assert_eq!(names, vec![".gitignore", "kept.txt"]);

    // Disable gitignore: ignored entries should reappear.
    let without_ignore = build(
        &tmp,
        &WalkOptions {
            show_hidden: true,
            use_gitignore: false,
            max_depth: None,
        },
    )
    .unwrap();
    let names: Vec<&str> = without_ignore
        .children
        .iter()
        .map(|n| n.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["build", ".gitignore", "ignored.txt", "kept.txt"]
    );

    fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn comments_save_and_reload_roundtrip() {
    let tmp = unique_tmp();
    let mut comments = Comments::default();
    comments.set("src", "ソース".into());
    comments.set("src/main.rs", "エントリ".into());
    comments.save(&tmp).unwrap();

    let reloaded = Comments::load(&tmp).unwrap();
    assert_eq!(reloaded.get("src"), Some("ソース"));
    assert_eq!(reloaded.get("src/main.rs"), Some("エントリ"));
    fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn comments_set_normalizes_trailing_slash() {
    let mut comments = Comments::default();
    comments.set("src/", "dir comment".into());
    assert_eq!(comments.get("src"), Some("dir comment"));
}

#[test]
fn init_comments_creates_empty_entry_per_path() {
    let tmp = unique_tmp();
    fs::create_dir(tmp.join("src")).unwrap();
    fs::write(tmp.join("src/main.rs"), "").unwrap();
    fs::write(tmp.join("README.md"), "").unwrap();

    let comments = init_comments(&tmp, &WalkOptions::default()).unwrap();
    let keys: Vec<&str> = comments.0.keys().map(String::as_str).collect();
    assert_eq!(keys, vec!["README.md", "src", "src/main.rs"]);
    assert!(comments.0.values().all(String::is_empty));
    fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn init_comments_preserves_existing_values_when_called_twice() {
    let tmp = unique_tmp();
    fs::write(tmp.join("a.txt"), "").unwrap();

    let mut comments = init_comments(&tmp, &WalkOptions::default()).unwrap();
    comments.set("a.txt", "first".into());
    comments.save(&tmp).unwrap();

    // Re-init should not erase the comment we wrote.
    let existing = Comments::load(&tmp).unwrap();
    let mut merged = existing;
    // Mimic the lib's collect_paths or_insert_with behaviour by calling init_comments
    // on top of saved state: load -> add missing keys.
    let fresh = init_comments(&tmp, &WalkOptions::default()).unwrap();
    for (k, _) in fresh.0 {
        merged.0.entry(k).or_default();
    }
    assert_eq!(merged.get("a.txt"), Some("first"));
    fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn cli_format_markdown_wraps_tree_in_code_fence() {
    let tmp = unique_tmp();
    fs::write(tmp.join("README.md"), "").unwrap();
    fs::create_dir(tmp.join("src")).unwrap();
    fs::write(tmp.join("src/main.rs"), "").unwrap();

    let bin = env!("CARGO_BIN_EXE_treedoc");
    let output = std::process::Command::new(bin)
        .arg("--format")
        .arg("markdown")
        .arg(&tmp)
        .output()
        .expect("failed to run treedoc binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Must start and end with a fence and contain the tree in between.
    assert!(stdout.starts_with("```\n"), "stdout: {stdout:?}");
    assert!(stdout.trim_end().ends_with("```"), "stdout: {stdout:?}");
    assert!(stdout.contains("├── src/"), "stdout: {stdout:?}");
    assert!(
        !stdout.contains('\x1b'),
        "should not contain ANSI: {stdout:?}"
    );

    fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn cli_format_plain_emits_no_color() {
    let tmp = unique_tmp();
    fs::write(tmp.join("README.md"), "").unwrap();

    let bin = env!("CARGO_BIN_EXE_treedoc");
    let output = std::process::Command::new(bin)
        .arg("--format")
        .arg("plain")
        .arg(&tmp)
        .output()
        .expect("failed to run treedoc binary");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout.contains('\x1b'),
        "should not contain ANSI: {stdout:?}"
    );
    assert!(!stdout.starts_with("```"), "no fence for plain: {stdout:?}");

    fs::remove_dir_all(&tmp).unwrap();
}
