//! forjar#722 falsification: lint.yml and msrv.yml run on arch-neutral clean-room
//! runners, so every actions/cache key must carry `runner.arch`. The shell guard
//! `tests/cache-key-arch-guard.sh` decides; this test runs it on the real tree
//! (must pass) and on a copy with the arch segment reverted (must fail), so the
//! guard is shown capable of failing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const WORKFLOWS: [&str; 2] = [".github/workflows/lint.yml", ".github/workflows/msrv.yml"];

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Run the guard from `root/tests/`, which is where it resolves its workflows.
fn run_guard(root: &Path) -> (bool, String) {
    let out = Command::new("bash")
        .arg(root.join("tests/cache-key-arch-guard.sh"))
        .output()
        .expect("bash runs the guard");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    (out.status.success(), text)
}

/// Copy the guard and both workflows into a scratch tree, optionally reverting the
/// arch segment of every cache key.
fn scratch_tree(name: &str, revert: bool) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("forjar-722-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    fs::copy(
        repo().join("tests/cache-key-arch-guard.sh"),
        dir.join("tests/cache-key-arch-guard.sh"),
    )
    .unwrap();
    for wf in WORKFLOWS {
        let mut text = fs::read_to_string(repo().join(wf)).unwrap();
        if revert {
            text = text.replace("${{ runner.arch }}-", "");
        }
        fs::write(dir.join(wf), text).unwrap();
    }
    dir
}

#[test]
fn every_cache_key_carries_runner_arch() {
    let (ok, text) = run_guard(&repo());
    assert!(ok, "guard failed on the real tree:\n{text}");
    for wf in WORKFLOWS {
        assert!(
            text.contains(&format!("ok {wf}")),
            "guard did not report {wf}:\n{text}"
        );
    }
}

#[test]
fn the_pre_722_cache_keys_are_caught() {
    let dir = scratch_tree("reverted", true);
    let (ok, text) = run_guard(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert!(!ok, "guard passed with runner.arch reverted:\n{text}");
    for wf in WORKFLOWS {
        assert!(
            text.contains(&format!("FAIL {wf}")),
            "guard did not name {wf}:\n{text}"
        );
    }
}

#[test]
fn an_unmodified_copy_passes() {
    let dir = scratch_tree("clean", false);
    let (ok, text) = run_guard(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert!(ok, "guard failed on an unmodified copy:\n{text}");
}
