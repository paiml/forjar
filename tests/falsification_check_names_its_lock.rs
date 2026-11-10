//! forjar#585: a check verdict names the lock it read (human and `--json`).

use std::fs;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

fn run(args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(FORJAR)
        .args(args)
        .arg("--no-color")
        .output()
        .expect("forjar must run");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn check_names_the_lock_it_read() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("st");
    let target = dir.path().join("t.txt");
    let cfg = dir.path().join("c.yaml");
    fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: s\nmachines:\n  box:\n    hostname: box\n    addr: 127.0.0.1\nresources:\n  r:\n    type: file\n    machine: box\n    path: {}\n    content: \"x\\n\"\n",
            target.display()
        ),
    )
    .unwrap();
    let (c, _) = run(&[
        "apply",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
        "--yes",
    ]);
    assert_eq!(c, Some(0));
    let want = fs::canonicalize(state.join("box").join("state.lock.yaml"))
        .unwrap()
        .display()
        .to_string();
    let (_, human) = run(&[
        "check",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
    ]);
    assert!(
        human.contains(&want),
        "human output lacks lock path {want}:\n{human}"
    );
    let (_, json) = run(&[
        "check",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
        "--json",
    ]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["locks"]["box"], want);
}
