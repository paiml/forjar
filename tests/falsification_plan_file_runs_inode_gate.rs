//! forjar#847: `apply --plan-file` returns before `apply_pre_validate`, so it
//! skipped the inode pre-check an ordinary `apply` runs. A reviewed plan must
//! not write onto a host that is out of inodes.

use std::process::Command;

fn forjar(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap()
}

fn out(o: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn run(threshold: u8) -> (bool, String, bool) {
    let d = tempfile::tempdir().unwrap();
    let target = d.path().join("made");
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: t\nmachines:\n  box:\n    hostname: localhost\n    addr: 127.0.0.1\n\
             resources:\n  r:\n    type: file\n    machine: box\n    path: {}\n    content: hi\n\
             inode_pressure:\n  threshold_pct: 100\n  mounts:\n    - path: /\n      threshold_pct: {threshold}\n",
            target.display()
        ),
    )
    .unwrap();
    let (c, s, p) = (
        cfg.to_str().unwrap(),
        d.path().join("state"),
        d.path().join("plan.json"),
    );
    let (s, p) = (s.to_str().unwrap(), p.to_str().unwrap());
    let made = forjar(&["plan", "-f", c, "--state-dir", s, "--out", p]);
    assert!(made.status.success(), "{}", out(&made));
    let o = forjar(&[
        "apply",
        "-f",
        c,
        "--state-dir",
        s,
        "--plan-file",
        p,
        "--yes",
    ]);
    (o.status.success(), out(&o), target.exists())
}

#[test]
fn a_plan_file_apply_is_refused_on_an_inode_starved_host() {
    let (ok, text, wrote) = run(1);
    assert!(!ok, "plan-file apply ignored the inode gate: {text}");
    assert!(text.contains("inode"), "{text}");
    assert!(!wrote, "plan-file apply wrote onto a failing host");
}

#[test]
fn the_same_plan_file_applies_when_inodes_are_fine() {
    let (ok, text, wrote) = run(100);
    assert!(ok && wrote, "{text}");
}
