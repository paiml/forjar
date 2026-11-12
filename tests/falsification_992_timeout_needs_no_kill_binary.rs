//! PMAT-992 (forjar#992): `--resource-timeout` bounds an apply on a host that
//! has NO external `kill` binary.
//!
//! MEASURED, infra clean-room on v1.34.0 (741b5b98), image `rust:1.95-slim`,
//! twice on the same inputs: `apply ran 30s; --resource-timeout 2 did not
//! bound it`. That image has no `/bin/kill` and no `/usr/bin/kill` (procps is
//! not installed). The timeout path killed the script's process group through
//! `Command::new("kill")` with the error discarded, so the spawn failed
//! silently, nothing was killed, and the join waited out the whole script.
//!
//! The fix signals the group by syscall (`killpg(2)`). Two falsifiers:
//! 1. behaviour: the real binary, under a `PATH` that holds every tool of the
//!    host except `kill`, ends a `sleep 30` task within the timeout;
//! 2. source: no file under `src/` spawns a `kill` binary.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

/// A directory that links every executable on the current `PATH` except
/// `kill`: the host as a procps-less image sees it.
fn path_without_kill(shim: &Path) -> PathBuf {
    std::fs::create_dir_all(shim).unwrap();
    let path = std::env::var_os("PATH").expect("PATH is set");
    for dir in std::env::split_paths(&path) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name == "kill" {
                continue;
            }
            // First hit wins, as in a PATH lookup; a name already linked stays.
            let _ = std::os::unix::fs::symlink(entry.path(), shim.join(&name));
        }
    }
    assert!(!shim.join("kill").exists(), "the shim must hold no kill");
    assert!(shim.join("sleep").exists(), "the shim must hold sleep");
    assert!(shim.join("bash").exists(), "the shim must hold bash");
    shim.to_path_buf()
}

#[test]
fn resource_timeout_bounds_the_apply_with_no_kill_binary_on_path() {
    let dir = tempfile::tempdir().unwrap();
    let shim = path_without_kill(&dir.path().join("bin"));
    std::fs::write(
        dir.path().join("forjar.yaml"),
        r#"version: '1.0'
name: fj992
machines:
  local:
    hostname: localhost
    addr: localhost
    transport: local
resources:
  slow:
    machine: local
    type: task
    command: "sleep 30"
policy:
  parallel_resources: true
"#,
    )
    .unwrap();

    let start = std::time::Instant::now();
    let out = Command::new(FORJAR)
        .current_dir(dir.path())
        .env("PATH", &shim)
        .args([
            "apply",
            "--yes",
            "-f",
            "forjar.yaml",
            "--state-dir",
            "state",
            "--resource-timeout",
            "2",
        ])
        .output()
        .expect("forjar binary must run");
    let elapsed = start.elapsed().as_secs();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        elapsed < 15,
        "apply ran {elapsed}s with no kill binary on PATH; --resource-timeout 2 did not bound it\n{text}"
    );
    assert!(
        !out.status.success(),
        "a timed-out resource must fail the apply\n{text}"
    );
}

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let p = entry.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn no_source_file_spawns_a_kill_binary() {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(
        files.len() > 100,
        "src/ was not walked: {} files",
        files.len()
    );
    let needle = concat!("Command::new(\"", "kill", "\")");
    let hits: Vec<String> = files
        .iter()
        .filter(|f| std::fs::read_to_string(f).is_ok_and(|s| s.contains(needle)))
        .map(|f| f.display().to_string())
        .collect();
    assert!(
        hits.is_empty(),
        "these files signal through an external kill binary, absent on a procps-less image: {hits:?}"
    );
}
