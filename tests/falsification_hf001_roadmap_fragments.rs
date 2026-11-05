//! HF-001: `docs/roadmaps/roadmap.yaml` is the generated aggregate of
//! itself plus `docs/roadmaps/entries/<id>.yaml` (`pmat roadmap aggregate`).
//! New rows are fragments; nobody edits the aggregate by hand.
//!
//! Falsifier: edit a row of roadmap.yaml directly, or delete a fragment whose
//! row is only in entries/, and `pmat roadmap aggregate --check` exits 1.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn pmat(args: &[&str], dir: &Path) -> (i32, String) {
    let out = Command::new("pmat")
        .args(["roadmap", "aggregate"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("pmat must be on PATH: an unmeasured check is a failing check");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn entries_dir_is_tracked_and_holds_the_hf001_fragment() {
    let entries = root().join("docs/roadmaps/entries");
    assert!(
        entries.join(".gitkeep").is_file(),
        "entries/.gitkeep missing"
    );
    assert!(
        entries.join("PMAT-5881.yaml").is_file(),
        "PMAT-5881 fragment missing"
    );
}

#[test]
fn aggregate_check_is_clean_and_byte_identical() {
    let (code, _) = pmat(&["--check"], &root());
    assert_eq!(code, 0, "pmat roadmap aggregate --check must be clean");
    let (code, printed) = pmat(&[], &root());
    assert_eq!(code, 0);
    let on_disk = std::fs::read_to_string(root().join("docs/roadmaps/roadmap.yaml")).unwrap();
    assert_eq!(
        printed, on_disk,
        "aggregate is not byte-identical to roadmap.yaml"
    );
}

/// Copy the roadmap layout to a scratch dir so the falsifiers never touch the tree.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hf001-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let rm = dir.join("docs/roadmaps");
    std::fs::create_dir_all(rm.join("entries")).unwrap();
    let src = root().join("docs/roadmaps");
    std::fs::copy(src.join("roadmap.yaml"), rm.join("roadmap.yaml")).unwrap();
    std::fs::copy(
        src.join("entries/PMAT-5881.yaml"),
        rm.join("entries/PMAT-5881.yaml"),
    )
    .unwrap();
    dir
}

/// A hand edit of a row a fragment owns (the aggregate drifts from its fragment).
#[test]
fn hand_edit_of_a_fragment_owned_row_is_red() {
    let dir = scratch("edit");
    let p = dir.join("docs/roadmaps/roadmap.yaml");
    let text = std::fs::read_to_string(&p).unwrap();
    let edited = text.replacen(
        "kind:code\n  notes: 'KZ-001",
        "kind:code\n  notes: 'HAND EDIT",
        1,
    );
    assert_ne!(edited, text, "fixture edit did not apply");
    std::fs::write(&p, edited).unwrap();
    let (code, _) = pmat(&["--check"], &dir);
    assert_eq!(
        code, 1,
        "a hand-edited fragment-owned row must fail --check"
    );
}

/// The PR-lane case: a fragment added without the aggregate carrying it.
#[test]
fn fragment_not_yet_aggregated_is_red() {
    let dir = scratch("new");
    let frag = std::fs::read_to_string(dir.join("docs/roadmaps/entries/PMAT-5881.yaml")).unwrap();
    let extra = frag.replace("PMAT-5881", "PMAT-5882");
    std::fs::write(dir.join("docs/roadmaps/entries/PMAT-5882.yaml"), extra).unwrap();
    let (code, _) = pmat(&["--check"], &dir);
    assert_eq!(code, 1, "an unaggregated fragment must fail --check");
}
