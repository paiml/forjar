//! forjar#829: `apply -r <one> --cost-limit 1` planned the whole file and
//! refused with "N changes planned, limit is 1" for N unselected resources.
//! The cost limit must count the selected resources only.

use std::fs;
use std::path::Path;
use std::process::Command;

fn config(dir: &Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    let res: String = ["a", "b", "c"]
        .iter()
        .map(|n| {
            format!(
                "\x20 {n}: {{ type: file, machine: local, path: {}, content: \"{n}\\n\" }}\n",
                dir.join(format!("{n}.txt")).display()
            )
        })
        .collect();
    fs::write(
        &cfg,
        format!("version: \"1.0\"\nname: cl\nmachines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\nresources:\n{res}"),
    )
    .unwrap();
    cfg
}

fn apply(dir: &Path, extra: &[&str]) -> (bool, String) {
    let cfg = config(dir);
    run(dir, &cfg, 1, extra)
}

/// Four resources: `base` <- `app` (depends_on), `other` (group web, tag t1),
/// `lone` (no group, no tag).
fn config_scoped(dir: &Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    let f = |n: &str, extra: &str| {
        format!(
            "  {n}: {{ type: file, machine: local, path: {}, content: \"{n}\\n\"{extra} }}\n",
            dir.join(format!("{n}.txt")).display()
        )
    };
    let res = [
        f("base", ", resource_group: web"),
        f("app", ", resource_group: web, depends_on: [base]"),
        f("other", ", tags: [t1]"),
        f("lone", ""),
    ]
    .concat();
    fs::write(
        &cfg,
        format!("version: \"1.0\"\nname: cl\nmachines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\nresources:\n{res}"),
    )
    .unwrap();
    cfg
}

fn run(dir: &Path, cfg: &Path, limit: usize, extra: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(["apply", "-f"])
        .arg(cfg)
        .arg("--state-dir")
        .arg(dir.join("state"))
        .args(["--yes", "--cost-limit", &limit.to_string()])
        .args(extra)
        .output()
        .expect("forjar starts");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), s)
}

#[test]
fn cost_limit_counts_only_the_selected_resource() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, out) = apply(dir.path(), &["-r", "a"]);
    assert!(!out.contains("Cost limit exceeded"), "{out}");
    assert!(ok, "{out}");
    assert_eq!(fs::read_to_string(dir.path().join("a.txt")).unwrap(), "a\n");
    assert!(
        !dir.path().join("b.txt").exists(),
        "unselected b was applied"
    );
}

#[test]
fn cost_limit_still_counts_every_resource_without_a_selector() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, out) = apply(dir.path(), &[]);
    assert!(!ok, "{out}");
    assert!(out.contains("3 changes planned, limit is 1"), "{out}");
}

#[test]
fn cost_limit_honors_exclude_too() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, out) = apply(dir.path(), &["--exclude", "b", "--skip", "c"]);
    assert!(ok, "{out}");
    assert!(!out.contains("Cost limit exceeded"), "{out}");
}

fn scoped(extra: &[&str], limit: usize) -> (bool, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config_scoped(dir.path());
    let (ok, out) = run(dir.path(), &cfg, limit, extra);
    (ok, out, dir)
}

#[test]
fn cost_limit_counts_a_group_with_its_depends_on_closure() {
    // -g web = app + its dependency base = 2 changes; lone/other are unselected.
    let (ok, out, dir) = scoped(&["-g", "web"], 2);
    assert!(ok && !out.contains("Cost limit exceeded"), "{out}");
    assert!(dir.path().join("app.txt").exists() && dir.path().join("base.txt").exists());
    assert!(!dir.path().join("lone.txt").exists());
    let (ok, out, _d) = scoped(&["-g", "web"], 1);
    assert!(
        !ok && out.contains("2 changes planned, limit is 1"),
        "{out}"
    );
}

#[test]
fn cost_limit_counts_a_tag_selection_only() {
    let (ok, out, dir) = scoped(&["-t", "t1"], 1);
    assert!(ok && !out.contains("Cost limit exceeded"), "{out}");
    assert!(dir.path().join("other.txt").exists());
    assert!(!dir.path().join("lone.txt").exists());
}

#[test]
fn cost_limit_counts_a_subset_selection_only() {
    let (ok, out, dir) = scoped(&["--subset", "lone"], 1);
    assert!(ok && !out.contains("Cost limit exceeded"), "{out}");
    assert!(dir.path().join("lone.txt").exists());
    assert!(!dir.path().join("other.txt").exists());
}
