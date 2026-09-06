//! Harness for `falsification_state_stamp_per_name`: the paiml/infra shape —
//! N stacks, each with its own `name:`, its own machine and its own file
//! resource, all applied through ONE `--state-dir`.
//!
//! Kept out of the test file so the tests stay assertions and narrative; the
//! fixture is the part that would otherwise be copied into the next suite that
//! needs a multi-stack state dir.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use forjar::core::state;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// One stack, in its own directory, with its OWN machine name and its own file
/// resource — the paiml/infra shape. `machines/<m>/forjar.yaml` differs from
/// its neighbours in exactly these three ways.
pub fn write_stack(root: &Path, dir: &str, name: &str, machine: &str, content: &str) -> PathBuf {
    let d = root.join(dir);
    std::fs::create_dir_all(&d).unwrap();
    let cfg = d.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: {name}
policy:
  snapshot_generations: 10
machines:
  {machine}:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
resources:
  {name}_file:
    type: file
    machine: {machine}
    path: {}
    content: "{content}\n"
"#,
            d.join("marker.txt").display()
        ),
    )
    .unwrap();
    cfg
}

pub fn run(args: &[&str]) -> (i32, String) {
    let out = forjar().args(args).output().unwrap();
    let mut merged = String::from_utf8_lossy(&out.stdout).into_owned();
    merged.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), merged)
}

pub fn apply(cfg: &Path, state: &Path) -> (i32, String) {
    run(&[
        "apply",
        "-f",
        &cfg.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--yes",
    ])
}

pub fn undo(cfg: &Path, state: &Path) -> (i32, String) {
    run(&[
        "undo",
        "-f",
        &cfg.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--yes",
    ])
}

/// `undo` naming its target explicitly. The flag is `--generations N` ("how
/// many back"); `undo` has no `--generation`, that spelling belongs to
/// `rollback`.
pub fn undo_generations(cfg: &Path, state: &Path, back: &str) -> (i32, String) {
    run(&[
        "undo",
        "-f",
        &cfg.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--generations",
        back,
        "--yes",
    ])
}

pub fn undo_resume(cfg: &Path, state: &Path) -> (i32, String) {
    run(&[
        "undo",
        "-f",
        &cfg.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--resume",
        "--yes",
    ])
}

/// The other door onto the same primitive: `rollback --generation N` reaches
/// `generation::rollback_to_generation` with no config in the picture at all,
/// which is why the guard cannot live in `undo`.
pub fn rollback(state: &Path, generation: &str) -> (i32, String) {
    run(&[
        "rollback",
        "--state-dir",
        &state.display().to_string(),
        "--generation",
        generation,
        "--yes",
    ])
}

/// Every path under `dir` with a digest of what it holds — file bytes, symlink
/// target, or the fact that it is a directory.
///
/// "The other stacks were untouched" has to be asserted over the WHOLE state
/// dir: the restore is whole-dir, so what it reverts is every machine lock and
/// `forjar.lock.yaml` itself, none of which the host-side marker files show.
pub fn fingerprint(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    fingerprint_into(dir, dir, &mut out);
    out
}

fn fingerprint_into(root: &Path, at: &Path, out: &mut BTreeMap<String, String>) {
    let Ok(entries) = std::fs::read_dir(at) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let key = path.strip_prefix(root).unwrap().display().to_string();
        let meta = std::fs::symlink_metadata(&path).unwrap();
        if meta.is_symlink() {
            let target = std::fs::read_link(&path).unwrap();
            out.insert(key, format!("-> {}", target.display()));
        } else if meta.is_dir() {
            out.insert(key, "dir".to_string());
            fingerprint_into(root, &path, out);
        } else {
            out.insert(key, digest(&std::fs::read(&path).unwrap()));
        }
    }
}

fn digest(bytes: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Every `warning:` line, whatever printed it. The point of forjar#469 is that
/// a supported layout produces NONE — a warning an operator sees on every
/// correct apply is a warning they stop reading.
///
/// ONE exclusion, and it is not a stack signal: named snapshots carry a
/// second-resolution timestamp, so two applies inside the same second collide
/// on the name and the second prints "pre-apply snapshot failed … already
/// exists". A test that drives five applies in three seconds hits that; an
/// operator does not. Excluded by its text so anything else still fails here.
pub fn warnings(out: &str) -> Vec<String> {
    out.lines()
        .filter(|l| l.trim_start().starts_with("warning:"))
        .filter(|l| !l.contains("pre-apply snapshot failed"))
        .map(str::to_string)
        .collect()
}

pub fn lock_of(state: &Path) -> forjar::core::types::GlobalLock {
    state::load_global_lock(state).unwrap().unwrap()
}

pub fn marker(root: &Path, dir: &str) -> String {
    std::fs::read_to_string(root.join(dir).join("marker.txt")).unwrap()
}

/// alpha/bravo/charlie, distinct machines, distinct resources, ONE state dir.
pub struct Fleet {
    pub _dir: tempfile::TempDir,
    pub root: PathBuf,
    pub state: PathBuf,
    pub alpha: PathBuf,
    pub bravo: PathBuf,
}

pub fn fleet() -> (Fleet, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let alpha = write_stack(&root, "alpha", "alpha", "mini", "one");
    let bravo = write_stack(&root, "bravo", "bravo", "lambda-labs", "one");
    let charlie = write_stack(&root, "charlie", "charlie", "clean-room", "one");

    let mut warned = Vec::new();
    for cfg in [&alpha, &bravo, &charlie] {
        let (rc, out) = apply(cfg, &state);
        assert_eq!(rc, 0, "apply of {} failed:\n{out}", cfg.display());
        warned.extend(warnings(&out));
    }
    (
        Fleet {
            _dir: dir,
            root,
            state,
            alpha,
            bravo,
        },
        warned,
    )
}
