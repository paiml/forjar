//! Shared fixture for the two suites that drive `scripts/ci/changed-class.sh`.
//!
//! The classifier is a script precisely so a test can feed it fabricated file
//! lists — PMAT-237 put the decision there instead of in a YAML `if:` for that
//! reason — and both PMAT-237's suite and PMAT-542's drive it the same way.
//! One copy of the plumbing, so the two cannot drift apart about what the
//! classifier's output even means.

#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/ci/changed-class.sh")
}

/// The classifier's whole output for a file list: a `code=` line and, since
/// PMAT-542, a `gates=` line.
pub fn output_of(files: &[&str]) -> String {
    let mut child = Command::new("bash")
        .arg(script())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bash must run");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(files.join("\n").as_bytes())
        .expect("write the file list");
    let out = child.wait_with_output().expect("classifier must finish");
    assert!(
        out.status.success(),
        "the classifier exited {:?}: {}{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// One `key=value` line of the classifier's output, by key.
///
/// Read by NAME rather than by position: the two lines are consumed by two
/// different workflow outputs, and a test that depended on their order would
/// pass over a classifier that swapped them and broke one of those consumers.
pub fn field(files: &[&str], key: &str) -> String {
    let out = output_of(files);
    let want = format!("{key}=");
    let hit: Vec<&str> = out.lines().filter_map(|l| l.strip_prefix(&want)).collect();
    assert_eq!(
        hit.len(),
        1,
        "the classifier printed {} `{key}=` lines for {files:?}; exactly one is \
         the contract:\n{out}",
        hit.len()
    );
    hit[0].to_string()
}

pub fn class_of(files: &[&str]) -> String {
    format!("code={}", field(files, "code"))
}

/// Which binary-measuring gates the change can move: `C,D`, `C`, `D` or `none`.
pub fn gates_of(files: &[&str]) -> String {
    field(files, "gates")
}

/// The two booleans a workflow `if:` compares exactly.
pub fn gate_booleans(files: &[&str]) -> (String, String) {
    (field(files, "gate_c"), field(files, "gate_d"))
}

pub fn assert_gates(files: &[&str], want: &str, why: &str) {
    assert_eq!(
        gates_of(files),
        want,
        "PMAT-542: {why}. A change touching {files:?} must select `{want}`. \
         Selecting too little skips a release build and a surface measurement \
         over a change that could move them; selecting too much is the 21.3 \
         minutes this ticket exists to stop paying."
    );
}

pub fn assert_code(files: &[&str], why: &str) {
    assert_eq!(
        class_of(files),
        "code=true",
        "PMAT-237: {why}. A change touching {files:?} must run every heavy job; \
         reading it as harmless skips the suite over it."
    );
}

pub fn assert_harmless(files: &[&str], why: &str) {
    assert_eq!(
        class_of(files),
        "code=false",
        "PMAT-237: {why}. A change touching only {files:?} cannot alter what any \
         heavy job measures, and paying forty minutes for it is the waste this \
         ticket exists to remove."
    );
}

pub fn workflow(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}"))
}
