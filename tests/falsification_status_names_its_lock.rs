//! forjar#585: status names the lock each machine row was read from.

use std::fs;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

fn run(args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(FORJAR)
        .args(args)
        .arg("--no-color")
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn status_names_the_lock_it_read() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("st");
    let cfg = dir.path().join("c.yaml");
    fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: s\nmachines:\n  box:\n    hostname: box\n    addr: 127.0.0.1\nresources:\n  r:\n    type: file\n    machine: box\n    path: {}\n    content: \"x\\n\"\n",
            dir.path().join("t.txt").display()
        ),
    )
    .unwrap();
    let s = state.to_str().unwrap();
    let (c, _) = run(&[
        "apply",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        s,
        "--yes",
    ]);
    assert_eq!(c, Some(0));
    let want = fs::canonicalize(state.join("box").join("state.lock.yaml"))
        .unwrap()
        .display()
        .to_string();
    let (_, human) = run(&["status", "--state-dir", s]);
    assert!(human.contains(&want), "human lacks {want}:\n{human}");
    let (_, json) = run(&["status", "--state-dir", s, "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["machines"][0]["lock"], want, "{json}");
}

#[test]
fn apply_names_the_lock_it_wrote() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("st");
    let cfg = dir.path().join("c.yaml");
    fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: s\nmachines:\n  box:\n    hostname: box\n    addr: 127.0.0.1\nresources:\n  r:\n    type: file\n    machine: box\n    path: {}\n    content: \"x\\n\"\n",
            dir.path().join("t.txt").display()
        ),
    )
    .unwrap();
    let s = state.to_str().unwrap();
    let (c, human) = run(&[
        "apply",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        s,
        "--yes",
    ]);
    assert_eq!(c, Some(0));
    let want = fs::canonicalize(state.join("box").join("state.lock.yaml"))
        .unwrap()
        .display()
        .to_string();
    assert!(human.contains(&want), "human lacks {want}:\n{human}");
    let (_, json) = run(&[
        "apply",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        s,
        "--yes",
        "--json",
    ]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["locks"]["box"], want, "{json}");
}
