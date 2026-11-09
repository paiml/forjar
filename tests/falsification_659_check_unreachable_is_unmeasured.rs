//! forjar#659: `forjar check` tells a host that NEEDS CHANGES from a host it
//! NEVER REACHED.
//!
//! WHAT WAS WRONG. `ssh` exits 255 when it cannot connect; that arrived as
//! `Ok(out)` and was tallied as `fail`, so a down box printed
//! `Check: 0 pass, 1 fail` and exited 1 exactly like drift. A caller using
//! `apply --check` as a drift probe read an unreachable host as drift.
//!
//! WHY THESE ASSERTIONS. The BINARY is driven: the exit code is the contract.
//!   * unreachable host (RFC 6761 `.invalid`): exit 4 (Connection), counted
//!     `unmeasured`, never `fail`; `--json` says `status: "unmeasured"`.
//!   * local drift (absent file): still a FAIL, still exit 1.
//!   * converged: exit 0 (negative control, so the fix is not "always unmeasured").

use std::path::Path;
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

fn config(dir: &Path, addr: &str, path: &Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: check-exit-probe\nmachines:\n  ghost:\n    hostname: ghost\n    \
             addr: {addr}\n    user: nobody\nresources:\n  marker:\n    type: file\n    machine: ghost\n    \
             path: {}\n    content: \"x\\n\"\n",
            path.display()
        ),
    )
    .expect("write config");
    cfg
}

fn run(cfg: &Path, dir: &Path, extra: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(forjar())
        .args(["check", "-f"])
        .arg(cfg)
        .arg("--state-dir")
        .arg(dir.join("st"))
        .args(extra)
        .output()
        .expect("run forjar");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), text)
}

#[test]
fn unreachable_host_is_unmeasured_exit_4() {
    let t = tempfile::tempdir().expect("tempdir");
    let cfg = config(
        t.path(),
        "forjar-check-probe.invalid",
        &t.path().join("absent.txt"),
    );
    let (rc, text) = run(&cfg, t.path(), &[]);
    assert_eq!(rc, Some(4), "an unreachable host exits 4:\n{text}");
    assert!(text.contains("1 unmeasured"), "counted unmeasured:\n{text}");
    assert!(text.contains("0 fail"), "never counted as a fail:\n{text}");
}

#[test]
fn unreachable_host_json_status_is_unmeasured() {
    let t = tempfile::tempdir().expect("tempdir");
    let cfg = config(
        t.path(),
        "forjar-check-probe.invalid",
        &t.path().join("absent.txt"),
    );
    let (rc, text) = run(&cfg, t.path(), &["--json"]);
    assert_eq!(rc, Some(4), "{text}");
    assert!(text.contains("\"status\": \"unmeasured\""), "{text}");
    assert!(text.contains("\"all_passed\": false"), "{text}");
}

#[test]
fn local_drift_is_still_a_fail_exit_1() {
    let t = tempfile::tempdir().expect("tempdir");
    let cfg = config(t.path(), "127.0.0.1", &t.path().join("absent.txt"));
    let (rc, text) = run(&cfg, t.path(), &[]);
    assert_eq!(rc, Some(1), "drift keeps exit 1:\n{text}");
    assert!(text.contains("1 fail"), "{text}");
    assert!(!text.contains("unmeasured"), "{text}");
}

#[test]
fn converged_local_host_exits_0() {
    let t = tempfile::tempdir().expect("tempdir");
    let file = t.path().join("present.txt");
    std::fs::write(&file, "x\n").expect("file");
    let cfg = config(t.path(), "127.0.0.1", &file);
    let (rc, text) = run(&cfg, t.path(), &[]);
    assert_eq!(rc, Some(0), "{text}");
}
