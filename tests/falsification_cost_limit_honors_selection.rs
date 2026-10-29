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
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(["apply", "-f"])
        .arg(&cfg)
        .arg("--state-dir")
        .arg(dir.join("state"))
        .args(["--yes", "--cost-limit", "1"])
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
