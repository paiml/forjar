//! PMAT-542: WHICH binary-measuring gate, not just whether any heavy job runs.
//!
//! `code=` is one boolean for every heavy job. Measured on the 25 PRs merged as
//! of cddf78cd, run through the classifier itself: 3 were `code=false` and
//! already skipped everything, 15 can move gate C or gate D, and 7 CANNOT and
//! paid 21.3 minutes of a 25.2-minute critical path for a release build and two
//! gates that read a surface their diff cannot reach.
//!
//! WHAT EACH GATE READS was derived by reading the two scripts, and a review
//! round refuted the first version of the list. Gate C runs
//! `scripts/dogfood/lib/binary.sh`, diffs the live surface against
//! `docs/audits/surface_audit.csv`, and evaluates
//! `tests/fixtures/dogfood/local-files.yaml` two ways. Gate D runs the same
//! resolver, every fenced invocation in `README.md` against the whole
//! `tests/fixtures/dogfood/` tree and a cookbook clone, and reconciles the
//! version against `Cargo.toml`.
//!
//! This is a LATENCY change and not a coverage change: `make dogfood-release`
//! runs A-H and T over the whole window before any tag, and a gate the lane did
//! not select prints NOT-SELECTED rather than PASS.

#![cfg(unix)]

mod changed_class_fixture;
use changed_class_fixture::{assert_code, assert_gates, gate_booleans, gates_of, workflow};

/// A change to the source tree can move both, and so can anything unclassified.
#[test]
fn source_and_the_manifests_select_both_gates() {
    assert_gates(
        &["src/core/mod.rs"],
        "C,D",
        "src/ is compiled into the binary",
    );
    assert_gates(&["Cargo.toml"], "C,D", "the manifest decides what is built");
    assert_gates(&["Cargo.lock"], "C,D", "the lock decides what is built");
    assert_gates(
        &["contracts/forjar-dogfood-coverage-v1.yaml"],
        "C,D",
        "the contract corpus is compiled in through forjar-contracts",
    );
}

/// The three files the two gates read by name, and which gate reads which.
#[test]
fn the_files_the_gates_read_select_exactly_their_own_gate() {
    assert_gates(
        &["docs/audits/surface_audit.csv"],
        "C,D",
        "gate C diffs the live surface against it and gate D checks every verb \
         the README names against it",
    );
    assert_gates(
        &["README.md"],
        "D",
        "gate D runs its fenced forjar blocks; gate C reads the binary and the \
         CSV and cannot see README at all",
    );
    assert_gates(
        &["scripts/dogfood/surface.sh"],
        "C",
        "gate C IS that script",
    );
    assert_gates(&["scripts/dogfood/docs.sh"], "D", "gate D IS that script");
    // And the neighbours in that directory are NOT selected. Selecting the whole
    // of scripts/dogfood/ was tried and measured: it takes the saving from 7
    // PRs in 25 to 2, because this repository develops its gate scripts
    // constantly and neither gate C nor gate D reads any of the others.
    assert_gates(
        &["scripts/dogfood/harness.sh"],
        "none",
        "gate A's script is read by neither gate C nor gate D",
    );
    assert_gates(
        &["scripts/dogfood/tagged.sh"],
        "none",
        "gate T's script is read by neither",
    );
}

/// THE HOLE A REVIEW ROUND MEASURED: the fixtures and the shared resolver.
///
/// The first version of this arm listed only the two gate scripts and the CSV,
/// so `tests/` and `scripts/` swallowed everything else. Both gates run
/// `scripts/dogfood/lib/binary.sh`; gate C evaluates
/// `tests/fixtures/dogfood/local-files.yaml` two ways and compares them; gate D
/// runs every documented invocation against the whole `tests/fixtures/dogfood/`
/// tree. A PR editing the very fixture gate C compares would have SKIPPED gate
/// C — the fail-open direction, which ships an unmeasured surface.
#[test]
fn the_fixtures_and_the_shared_resolver_are_not_harmless() {
    assert_gates(
        &["tests/fixtures/dogfood/local-files.yaml"],
        "C,D",
        "gate C compares two evaluations of exactly this file",
    );
    assert_gates(
        &["tests/fixtures/dogfood/Makefile"],
        "C,D",
        "gate D runs a documented invocation against it",
    );
    assert_gates(
        &["scripts/dogfood/lib/binary.sh"],
        "C,D",
        "both gates run it to resolve the binary under test",
    );
    // The siblings those live among are still harmless, or the arm would have
    // bought its safety by selecting everything.
    assert_gates(
        &["tests/falsification_dist.rs"],
        "none",
        "an integration test is not compiled into the release binary",
    );
    assert_gates(
        &["scripts/quorum-gate.sh"],
        "none",
        "a script outside scripts/dogfood/ is read by neither gate",
    );
}

/// One token of a gate script's source, as a repository path, if it names one.
fn gate_path(tok: &str) -> Option<String> {
    if tok.len() <= 5 {
        return None;
    }
    if let Some(rest) = tok.strip_prefix("lib/") {
        return Some(format!("scripts/dogfood/lib/{rest}"));
    }
    if tok.starts_with("tests/fixtures/") {
        return Some(tok.to_string());
    }
    None
}

/// Every `tests/fixtures/…` and `lib/…` path the two gate scripts name.
fn paths_the_gates_name() -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for gate in ["scripts/dogfood/surface.sh", "scripts/dogfood/docs.sh"] {
        let text = workflow(gate);
        let toks = text.split(|c: char| !(c.is_alphanumeric() || "._/-".contains(c)));
        for path in toks.filter_map(gate_path) {
            if !found.contains(&path) {
                found.push(path);
            }
        }
    }
    found
}

/// THE RULE THAT KEEPS THE CLOSURE TRUE.
///
/// The selection was not guessed; it is what the two gate scripts actually
/// open. This case re-derives that from the scripts themselves and asserts the
/// classifier selects each path. A gate that starts reading a new fixture, or
/// sourcing a new library, turns this RED rather than blinding itself — the
/// same shape as PMAT-237's
/// `every_record_path_a_test_reads_is_classified_as_code`.
#[test]
fn every_path_the_two_gates_name_is_selected_by_the_classifier() {
    let found = paths_the_gates_name();
    assert!(
        found.len() >= 3,
        "PMAT-542: only {found:?} re-derived from the two gate scripts. This \
         case is supposed to find at least the two fixtures and the shared \
         binary resolver; finding fewer means it has stopped reading them."
    );
    for path in &found {
        assert_ne!(
            gates_of(&[path.as_str()]),
            "none",
            "PMAT-542: {path} is named by a gate script and the classifier calls \
             it harmless. That is the fail-open direction: a change to it would \
             skip the very gate that reads it."
        );
    }
}

/// The seven-in-twenty-five this ticket is about.
///
/// A receipt, its evidence, its log, a script outside `scripts/dogfood/` and a
/// test — the shape of #515, #517, #519, #523, #532, #536 and #541, the last of
/// which is the pull request that landed the ticket before this one.
/// `code=true`, because the guard tests and the workspace suite can all move;
/// `gates=none`, because the release build and both surface gates cannot.
#[test]
fn the_shape_that_pays_for_nothing_selects_no_gate() {
    let files = &[
        "docs/audits/impl-PMAT-535-receipt.md",
        "docs/audits/logs/PMAT-535-branch-names-its-ticket.log",
        ".quorum/PMAT-535-a-branch-names-its-own-ticket.json",
        ".quorum/evidence/branch-names-its-ticket-judges.md",
        "scripts/quorum-gate.sh",
        "tests/falsification_a_branch_names_its_own_ticket.rs",
        "docs/roadmaps/roadmap.yaml",
    ];
    assert_code(files, "the guard tests and the workspace suite read these");
    assert_gates(
        files,
        "none",
        "none of these is the binary, README.md, the surface CSV or either \
         gate's own script",
    );
}

/// The direction the selection must fail in, mirroring `an_unclassified_path_is_code`.
#[test]
fn an_unclassified_path_selects_both_gates() {
    assert_gates(
        &["some/path/nobody/classified"],
        "C,D",
        "an allow-list of the harmless must treat the unknown as reaching \
         everything; a deny-list of the heavy fails open the moment someone \
         adds a crate nobody listed",
    );
    assert_gates(
        &["docs/audits/x.md", "src/core/mod.rs"],
        "C,D",
        "one reaching path in a list of harmless ones still selects",
    );
}

/// An empty file list measured nothing, and nothing is not a selection.
#[test]
fn an_empty_file_list_selects_both_gates() {
    assert_gates(
        &[],
        "C,D",
        "no readable file list is not a licence to skip a measurement",
    );
}

/// `none`, never the empty string.
///
/// The workflow gate refuses an empty `gates` because it cannot tell "nothing
/// was selected" from "nothing computed a selection". That only works if the
/// classifier never emits the empty string in the first place.
#[test]
fn the_selection_is_never_the_empty_string() {
    for files in [
        vec!["docs/a.md"],
        vec!["tests/b.rs"],
        vec!["scripts/c.sh"],
        vec!["CLAUDE.md"],
        vec![".quorum/d.json"],
    ] {
        let g = gates_of(&files);
        assert!(
            !g.is_empty(),
            "PMAT-542: the classifier emitted an empty selection for {files:?}. \
             The workflow gate reads an empty value as UNMEASURED and refuses \
             the run, so a harmless change would fail CI."
        );
        assert_eq!(
            g, "none",
            "PMAT-542: {files:?} should select no gate and say so by name"
        );
    }
}

/// The workflow must not print PASS for a gate it did not run.
#[test]
fn a_gate_the_lane_did_not_select_says_so_by_name() {
    let ci = workflow(".github/workflows/ci.yml");
    for g in ["Gate C — NOT-SELECTED", "Gate D — NOT-SELECTED"] {
        assert!(
            ci.contains(g),
            "PMAT-542: ci.yml has no `{g}` step. A gate the lane skipped and a \
             gate that passed must never print the same thing — that is the \
             rule in CLAUDE.md, and it is the whole difference between this \
             change and a `--skip`."
        );
    }
    assert!(
        ci.contains("NOT-SELECTED — this change can move neither gate C nor gate D"),
        "PMAT-542: the gate job does not say, in its own summary, that the \
         surface job was NOT-SELECTED rather than passing:\n"
    );
}

/// A skipped `dogfood-surface` is refused whenever the selection says it should
/// have run — the same treatment PMAT-237 gives a skipped heavy job.
#[test]
fn a_surface_skip_is_refused_when_the_selection_says_it_should_have_run() {
    let ci = workflow(".github/workflows/ci.yml");
    assert!(
        ci.contains("dogfood-surface was skipped while the selection says"),
        "PMAT-542: ci.yml's gate accepts a skipped dogfood-surface without \
         re-reading the selection. Trusting the skip is how a saving becomes a \
         hole, and this job skips on its OWN condition rather than on the class."
    );
    assert!(
        ci.contains("the gate selection is empty"),
        "PMAT-542: ci.yml's gate does not refuse an empty selection. An empty \
         value means nothing measured which gates the change can move, and an \
         unmeasured selection is not a selection."
    );
}

/// The selection is computed by the same script the cases above drive.
#[test]
fn the_workflow_reads_the_selection_from_the_one_script() {
    let action = workflow(".github/actions/changed-class/action.yml");
    assert!(
        action.contains("steps.classify.outputs.gates")
            && action.contains("steps.classify.outputs.gate_c")
            && action.contains("steps.classify.outputs.gate_d"),
        "PMAT-542: the composite action does not expose `gates`, so the \
         workflow computes the selection some other way and the cases above \
         test something CI does not run:\n{action}"
    );
    let ci = workflow(".github/workflows/ci.yml");
    assert!(
        ci.contains("gates: ${{ steps.class.outputs.gates }}")
            && ci.contains("gate_c: ${{ steps.class.outputs.gate_c }}")
            && ci.contains("gate_d: ${{ steps.class.outputs.gate_d }}"),
        "PMAT-542: ci.yml's classify job does not pass `gates` on, so every \
         consumer below reads an empty value and the gate refuses the run."
    );
}

/// The two representations cannot drift apart.
///
/// `gates=` is a readable token for the run summary; `gate_c=`/`gate_d=` are
/// what a workflow `if:` compares. Two spellings of one decision is two places
/// to be wrong, so this case re-derives each from the other on every shape the
/// suite uses.
#[test]
fn the_booleans_and_the_summary_token_are_one_decision() {
    for files in [
        vec!["src/core/mod.rs"],
        vec!["README.md"],
        vec!["scripts/dogfood/surface.sh"],
        vec!["docs/audits/surface_audit.csv"],
        vec!["tests/a.rs", "docs/b.md", ".quorum/c.json"],
        vec![],
    ] {
        let (c, d) = gate_booleans(&files);
        let want = match (c.as_str(), d.as_str()) {
            ("true", "true") => "C,D",
            ("true", "false") => "C",
            ("false", "true") => "D",
            ("false", "false") => "none",
            other => panic!("PMAT-542: gate_c/gate_d are not booleans for {files:?}: {other:?}"),
        };
        assert_eq!(
            gates_of(&files),
            want,
            "PMAT-542: for {files:?} the booleans say ({c}, {d}) and the summary \
             token says something else. The workflow compares the booleans and \
             the gate job prints the token, so a disagreement is a run that \
             reports one thing and did another."
        );
    }
}

/// The workflow compares the selection EXACTLY, never with `contains()`.
///
/// GitHub's `contains(search, item)` is a case-insensitive SUBSTRING test.
/// `contains('none', 'C')` is false today only because the word `none` happens
/// not to contain the letter `c`; rename a gate to `N` and the summary word
/// itself would select it. A gate whose selection depends on that is a gate
/// waiting to be wrong, and it would be wrong in the direction that SKIPS a
/// measurement.
#[test]
fn the_workflow_compares_the_selection_exactly() {
    let ci = workflow(".github/workflows/ci.yml");
    // Anchored on `if: `, not on the bare expression: a comment carrying the
    // same words would satisfy a bare substring test while the workflow did
    // something else. A review lane made that exact point.
    for want in [
        "if: needs.classify.outputs.gate_c == 'true'",
        "if: needs.classify.outputs.gate_c != 'true'",
        "if: needs.classify.outputs.gate_d == 'true'",
        "if: needs.classify.outputs.gate_d != 'true'",
    ] {
        assert!(
            ci.contains(want),
            "PMAT-542: ci.yml does not select gates with `{want}`. Exact \
             comparison of a boolean is the only form that cannot be fooled by \
             a substring."
        );
    }
    assert!(
        !ci.contains("contains(needs.classify.outputs.gates"),
        "PMAT-542: ci.yml selects a gate with `contains()` on the summary \
         token. That is a case-insensitive substring test:\n"
    );
}

/// An absent base is UNMEASURED, and unmeasured is everything.
///
/// The action used to fall back to `HEAD~1` when it had no PR base and no
/// `event.before` — a `workflow_dispatch` re-run of a twenty-commit branch was
/// then classified from ONE commit. That narrows the diff, which selects too
/// little, which skips a measurement. Found by a review lane on this ticket.
#[test]
fn an_absent_base_is_not_a_narrower_base() {
    let action = workflow(".github/actions/changed-class/action.yml");
    assert!(
        !action.contains("BASE=$(git rev-parse 'HEAD~1'"),
        "PMAT-542: the action still falls back to HEAD~1 when it has no base. \
         A narrower diff looks like a measurement and is not:\n{action}"
    );
    assert!(
        action.contains("the change is UNMEASURED, which is code"),
        "PMAT-542: the action does not say what it does when it has no usable \
         base. It must hand the empty file list to the classifier, whose own \
         rule is that no readable file list is not a licence to skip anything."
    );
}
