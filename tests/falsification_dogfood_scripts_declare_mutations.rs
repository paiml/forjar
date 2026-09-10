//! PMAT-163: a gate with no registered mutation is inadmissible.
//!
//! # Why this test exists
//!
//! A shell gate is trusted because it is green, and a green shell gate proves
//! nothing on its own: `set -e` absent, a `|| true` on the measuring command,
//! an `exit 0` at the bottom — each of those turns a gate into a printer, and
//! all three look identical in CI output to a gate that actually ran.
//!
//! So every `scripts/dogfood/*.sh` carries a `# mutation:` line naming the
//! ONE-LINE change that turns it RED. That comment is the falsifier's address:
//! a reviewer can apply it and watch the gate fail. This test asserts the
//! address exists, that `set -euo pipefail` is present so a failing stage
//! actually stops the script, and that no `|| true` swallows a measurement.
//!
//! The floor at the bottom is the anti-vacuity clause: an empty
//! `scripts/dogfood/` would satisfy every `for` loop above it.

use std::path::{Path, PathBuf};

/// The mechanical gates, by the letter they carry in the dogfood receipt.
const REQUIRED: [(&str, &str); 10] = [
    ("harness.sh", "A — harness receipt per merged PR"),
    ("comply.sh", "B — pmat comply"),
    ("surface.sh", "C — transport surface"),
    ("docs.sh", "D — documented invocations"),
    ("quorum.sh", "E — quorum receipt per merged PR"),
    ("coverage.sh", "F — coverage and mutants"),
    ("contracts.sh", "G — contracts"),
    ("crux-reconcile.sh", "H — crux reconciliation"),
    (
        "tagged.sh",
        "T — release goals: every tag declared, every ticket labelled, the cut on time",
    ),
    ("release-check.sh", "release artifacts"),
];

fn dogfood_scripts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/dogfood")
}

fn scripts() -> Vec<PathBuf> {
    let dir = dogfood_scripts_dir();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("no {} — the mechanical gates are gone: {e}", dir.display()));
    let mut out: Vec<PathBuf> = entries
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "sh"))
        .collect();
    out.sort();
    out
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("reading {}: {e}", p.display()))
}

#[test]
fn every_gate_script_names_its_mutation() {
    for p in scripts() {
        let body = read(&p);
        let line = body
            .lines()
            .find(|l| l.trim_start().starts_with("# mutation:"));
        let line = line.unwrap_or_else(|| {
            panic!(
                "{} has no `# mutation:` line — a gate that cannot be shown to \
                 go red is indistinguishable from a gate that prints",
                p.display()
            )
        });
        let detail = line.trim_start().trim_start_matches("# mutation:").trim();
        assert!(
            detail.len() >= 20,
            "{}: the `# mutation:` line must name the one-line change that turns \
             the gate red, not merely exist; got {detail:?}",
            p.display()
        );
    }
}

#[test]
fn every_gate_script_is_strict() {
    for p in scripts() {
        let body = read(&p);
        assert!(
            body.lines().any(|l| l.trim() == "set -euo pipefail"),
            "{} does not `set -euo pipefail`; without it a failing stage is a \
             passing script",
            p.display()
        );
    }
}

#[test]
fn no_gate_script_swallows_a_measurement() {
    for p in scripts() {
        let body = read(&p);
        for (n, l) in body.lines().enumerate() {
            // A `|| true` in a comment is still an instruction to copy.
            assert!(
                !l.contains("|| true"),
                "{}:{}: `|| true` — the measurement it guards can no longer fail: {}",
                p.display(),
                n + 1,
                l.trim()
            );
        }
    }
}

#[test]
fn the_expected_gates_are_all_present() {
    let dir = dogfood_scripts_dir();
    for (name, what) in REQUIRED {
        let p = dir.join(name);
        assert!(
            p.is_file(),
            "{} is missing — gate {what} has no implementation, so every loop \
             above this test runs over a smaller set than the receipt claims",
            p.display()
        );
    }
    assert!(
        scripts().len() >= REQUIRED.len(),
        "expected at least {} gate scripts, found {}",
        REQUIRED.len(),
        scripts().len()
    );
}
