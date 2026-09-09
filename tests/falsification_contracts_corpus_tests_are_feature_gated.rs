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
//! WHAT THIS RULE DOES NOT SAY. `cross_project_tests.rs` guards assertions with
//! `has_sibling_repos()` on purpose and documents it, and those assertions are
//! ABOUT the sibling — `find_binding_path(&aprender_dir, "aprender")` needs the
//! directory and asks for nothing from forjar's index. That idiom is sound and
//! is left alone. The rule below is narrow on purpose: it is about querying
//! aprender's KERNELS, which only the corpus can answer.

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

/// Split a file into `#[test]` blocks: the attributes above each one, and the
/// body up to the next `#[test]`. Crude on purpose — a parser here would be a
/// second implementation of rustc, and the shape this rule reads is stable.
fn test_blocks(src: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = src.lines().collect();
    let mut blocks = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.trim() != "#[test]" {
            continue;
        }
        // ATTRIBUTES: a fixed window back to the end of the previous item,
        // because the gate is written multi-line and a line-shape walk stopped
        // one line short of `)]` every time.
        let mut start = i;
        while start > 0 && i - start < 12 {
            let prev = lines[start - 1].trim();
            if prev == "}" || prev.ends_with(';') || prev.is_empty() {
                break;
            }
            start -= 1;
        }
        let mut end = i + 1;
        while end < lines.len() && lines[end].trim() != "#[test]" {
            end += 1;
        }
        blocks.push((lines[start..i].join("\n"), lines[i..end].join("\n")));
    }
    blocks
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

    assert_eq!(
        measured, recorded_annotations,
        "forjar#452: the tree carries {measured} aprender-corpus annotation(s) and \
         scripts/dogfood/coverage.sh records {recorded_annotations}. Both numbers are \
         EXACT in both directions on purpose, so every change to the excluded set \
         is a deliberate edit a reviewer sees in the diff. Move the recorded \
         figure in the same commit that changes the set."
    );
}

/// AND THE GATE MUST NOT BE A DIRECTORY CHECK. The specific wrong guard, named,
/// so re-introducing it fails here rather than on someone's workstation.
#[test]
fn no_test_decides_to_assert_by_looking_for_a_sibling_checkout_of_aprender() {
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/forjar-contracts/src");
    let mut files = Vec::new();
    rust_sources(&crate_src, &mut files);

    let mut offenders = Vec::new();
    for f in &files {
        let Ok(src) = fs::read_to_string(f) else {
            continue;
        };
        for (_, body) in test_blocks(&src) {
            let joined: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
            // `if !<something>.join("aprender").exists() { return; }` — bail out
            // when the neighbour is missing, then assert regardless of whether
            // THIS repo can answer the question.
            let bails_on_missing_sibling = joined.contains("join(\"aprender\").exists()")
                && (joined.contains("if !") || joined.contains("{ return; }"))
                && joined.contains("return");
            if bails_on_missing_sibling && body.contains("assert") {
                let name = body
                    .lines()
                    .find(|l| l.contains("fn "))
                    .unwrap_or("<unknown>")
                    .trim()
                    .to_string();
                offenders.push(format!("{}: {}", f.display(), name));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "forjar#452: {} test(s) return early when a sibling aprender checkout is \
         missing and then assert. That guard answers 'is a neighbour on disk', \
         not 'does THIS repo ship the corpus', so it is vacuous in CI and lets \
         the assertion run — and fail — on a workstation. Use the \
         aprender-corpus feature gate, which says the true thing:\n  {}",
        offenders.len(),
        offenders.join("\n  ")
    );
}
