//! forjar#452: a test that queried aprender's corpus decided whether to assert
//! by looking at what else was on the developer's disk.
//!
//! `crates/forjar-contracts` is vendored into forjar. forjar ships an IaC
//! contract corpus; aprender ships the kernel corpus (`softmax`, `rmsnorm`).
//! Tests that need the second are gated behind the `aprender-corpus` feature,
//! which the crate's own `Cargo.toml` documents and `scripts/dogfood/coverage.sh`
//! ratchets EXACTLY in both directions, so every change to the excluded set is a
//! visible edit.
//!
//! `query::coverage_tests::coverage_map_enrichment` used a different gate:
//!
//! ```ignore
//! let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
//! if !root.parent().is_some_and(|p| p.join("aprender").exists()) { return; }
//! ```
//!
//! A neighbouring checkout is the wrong question. It does not put softmax
//! contracts into forjar's index, so the guard passes on a workstation that has
//! aprender beside forjar, and the assertion then cannot hold:
//!
//! ```text
//! assertion failed: !output.results.is_empty()
//! ```
//!
//! Vacuously green in CI and in any worktree outside `~/src`, red on a fleet
//! workstation. A test whose verdict depends on what else is on the disk is
//! worse than one that simply fails: it cost a wrong conclusion before it
//! yielded a right one, when a branch was compared against a `main` that had
//! only ever run the vacuous half.
//!
//! WHAT THIS RULE IS, EXACTLY. It is a TEXT ratchet over the crate's sources,
//! not semantic analysis. It reads for the shape of the defect and nothing
//! more. Three reviewers said so independently and they are right, so it is
//! written here rather than left for the next reader to discover: the rule can
//! be evaded by moving the check into a helper, by `is_dir()` instead of
//! `exists()`, or by binding the path to a local first. It catches the defect
//! coming back the way it went in, which is the common case, and it is honest
//! about not catching a determined rewrite.
//!
//! `cross_project_tests.rs` guards assertions with `has_sibling_repos()` on
//! purpose and documents it, and those assertions are ABOUT the sibling —
//! `find_binding_path(&aprender_dir, "aprender")` needs the directory and asks
//! nothing of forjar's index. That idiom is sound. It is EXEMPTED BY NAME
//! below, because it used to pass this rule only by accident: its check lives
//! in a helper, so the banned shape never appears inside a `#[test]` body. An
//! exemption that holds by accident is not an exemption, it is a gap that
//! happens to be empty, and the next file to use a helper would inherit it
//! silently. The scan is now file-wide and the one allowed file is named.

use std::fs;
use std::path::{Path, PathBuf};

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Every `#[test]` body in a file, as text.
///
/// The attributes above each test are NOT returned. An earlier draft parsed
/// them with a 12-line window and then discarded the result, which three
/// reviewers each spotted as dead code; the rule reads bodies, so bodies are
/// all it collects.
fn test_bodies(src: &str) -> Vec<String> {
    let lines: Vec<&str> = src.lines().collect();
    let mut bodies = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.trim() != "#[test]" {
            continue;
        }
        let mut end = i + 1;
        while end < lines.len() && lines[end].trim() != "#[test]" {
            end += 1;
        }
        bodies.push(lines[i..end].join("\n"));
    }
    bodies
}

/// THE RATCHET MUST MATCH THE TREE.
///
/// `scripts/dogfood/coverage.sh` records two EXACT numbers, not ceilings, and
/// says why in its own comment: a ceiling catches somebody parking a new test
/// and silently accepts the other direction, so an ignored test deleted or
/// un-ignored leaves slack that the next parked test fits under with nothing
/// said. Exact figures only stay honest while they match, and nothing was
/// checking that they do outside a full gate F run — which cannot currently
/// complete on this workstation at all (PMAT-216).
///
/// This rule is the cheap half of that gate, and it is the one that fails when
/// somebody adds an exclusion without recording it.
#[test]
fn the_aprender_corpus_ratchet_matches_what_the_tree_actually_carries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let gate = fs::read_to_string(root.join("scripts/dogfood/coverage.sh"))
        .expect("the gate script must be where this rule looks");

    let recorded = |key: &str| -> usize {
        gate.lines()
            .find_map(|l| l.trim().strip_prefix(key))
            .unwrap_or_else(|| panic!("{key} is not recorded in scripts/dogfood/coverage.sh"))
            .trim()
            .parse()
            .expect("the recorded figure must be a number")
    };
    let recorded_annotations = recorded("APRENDER_ANNOTATIONS=");

    let mut files = Vec::new();
    rust_sources(&root.join("crates/forjar-contracts/src"), &mut files);
    assert!(
        files.len() > 20,
        "the sweep found only {} source file(s), so it is measuring nothing",
        files.len()
    );
    let measured: usize = files
        .iter()
        .filter_map(|f| fs::read_to_string(f).ok())
        .map(|src| src.matches("not(feature = \"aprender-corpus\")").count())
        .sum();

    // THE FIGURE LIVES IN TWO PLACES, so the invariant has to reach both. The
    // gate's own falsification suite runs coverage.sh against a `cargo` shim
    // that prints a canned "N ignored" line, and that N has to agree with
    // IGNORED_EXPECTED or gate F fails inside its own tests rather than on the
    // tree. Moving one number and not the other is precisely the mistake this
    // rule exists to catch, and it caught it on the commit that added the rule.
    let recorded_ignored = recorded("IGNORED_EXPECTED=");
    let shim = fs::read_to_string(root.join("tests/falsification_coverage_gate_mutation_scope.rs"))
        .expect("the gate's falsification suite must be where this rule looks");
    let shim_ignored: usize = shim
        .lines()
        .find_map(|l| {
            l.split_once(" ignored;")
                .and_then(|(head, _)| head.rsplit_once("failed; "))
                .map(|(_, n)| n.to_string())
        })
        .expect("the cargo shim must print an ignored count")
        .parse()
        .expect("the shim's ignored count must be a number");
    assert_eq!(
        shim_ignored, recorded_ignored,
        "forjar#452: the cargo shim in the gate's own falsification suite prints \
         {shim_ignored} ignored and scripts/dogfood/coverage.sh records \
         {recorded_ignored}. Both must move in the same commit, or gate F goes red \
         inside its own tests."
    );

    // AND THE LIVING PROSE THAT STATES IT. The figure turned out to live in
    // five places, and the first version of this rule covered two. A reviewer
    // found `Cargo.toml` and `VENDORED.md` still saying 38 after the set had
    // grown to 39 — documentation that contradicts the gate it documents, which
    // is how the next reader learns to distrust both. Historical text is NOT
    // included: a completed roadmap row states what was true when it was filed
    // and rewriting it would falsify the record.
    for (path, phrase) in [
        (
            "crates/forjar-contracts/Cargo.toml",
            "tests that read aprender's contract corpus",
        ),
        ("crates/forjar-contracts/VENDORED.md", "of the crate's"),
    ] {
        let text = fs::read_to_string(root.join(path))
            .unwrap_or_else(|e| panic!("{path} must be readable: {e}"));
        let stale: Vec<&str> = text
            .lines()
            .filter(|l| l.contains(phrase))
            .filter(|l| !l.contains(&recorded_annotations.to_string()))
            .collect();
        assert!(
            stale.is_empty(),
            "forjar#452: {path} describes the gated set with a figure other than \
             the recorded {recorded_annotations}. Living documentation that \
             contradicts the ratchet teaches readers to trust neither:\n  {}",
            stale.join("\n  ")
        );
    }

    assert_eq!(
        measured, recorded_annotations,
        "forjar#452: the tree carries {measured} aprender-corpus annotation(s) and \
         scripts/dogfood/coverage.sh records {recorded_annotations}. Both numbers are \
         EXACT in both directions on purpose, so every change to the excluded set \
         is a deliberate edit a reviewer sees in the diff. Move the recorded \
         figure in the same commit that changes the set."
    );
}

/// AND THE GATE MUST NOT BE A DIRECTORY CHECK.
///
/// The specific wrong guard, named, so re-introducing it fails here rather than
/// on someone's workstation. The scan is FILE-WIDE, not per-test-body, because
/// the shape can be hidden one call deep: `cross_project_tests.rs` puts it in
/// `has_sibling_repos()` and so never carried it inside a `#[test]` at all. A
/// body-only scan would have let any file do the same and call it a pass.
///
/// One file is exempt, by name and with its reason. That is the point of the
/// change: it used to be exempt by accident.
#[test]
fn no_test_decides_to_assert_by_looking_for_a_sibling_checkout_of_aprender() {
    // `cross_project_tests.rs` asserts ABOUT the sibling — `find_binding_path`
    // on the aprender directory, `CrossProjectIndex::build_with_extra` given
    // that path. Those assertions need the directory and ask nothing of
    // forjar's own index, so "is the neighbour here" is the right question for
    // them and the wrong one for a corpus query.
    const EXEMPT: [&str; 1] = ["cross_project_tests.rs"];

    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/forjar-contracts/src");
    let mut files = Vec::new();
    rust_sources(&crate_src, &mut files);
    assert!(
        files.len() > 20,
        "the sweep found only {} source file(s), so it is measuring nothing",
        files.len()
    );

    let mut offenders = Vec::new();
    let mut exempt_seen = 0usize;
    for f in &files {
        let name = f
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let Ok(src) = fs::read_to_string(f) else {
            continue;
        };
        let joined: String = src.split_whitespace().collect::<Vec<_>>().join(" ");
        let reaches_for_the_sibling = joined.contains("join(\"aprender\").exists()")
            || joined.contains("join(\"aprender\").is_dir()");
        if !reaches_for_the_sibling {
            continue;
        }
        if EXEMPT.contains(&name.as_str()) {
            exempt_seen += 1;
            continue;
        }
        // A file that reaches for the neighbour AND asserts is making a claim
        // whose truth depends on what else is on the disk.
        if test_bodies(&src).iter().any(|b| b.contains("assert")) {
            offenders.push(f.display().to_string());
        }
    }

    assert_eq!(
        exempt_seen,
        EXEMPT.len(),
        "the exemption list names {} file(s) and {} of them still reach for a \
         sibling aprender checkout. An exemption for something that no longer \
         does it is dead text: drop the name from EXEMPT.",
        EXEMPT.len(),
        exempt_seen
    );
    assert!(
        offenders.is_empty(),
        "forjar#452: {} file(s) look for a sibling aprender checkout and then \
         assert. That guard answers 'is a neighbour on disk', not 'does THIS \
         repo ship the corpus', so it is vacuous in CI and lets the assertion \
         run — and fail — on a workstation. Use the aprender-corpus feature \
         gate, which says the true thing:\n  {}",
        offenders.len(),
        offenders.join("\n  ")
    );
}
