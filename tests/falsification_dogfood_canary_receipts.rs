//! PMAT-1170 (Refs #990): the SECOND receipt shape. `quorum.claims_refuted >= 1`
//! stays the floor ("a round that killed no claim did not hunt"); a clean round
//! may instead carry `quorum.canary`: >= 2 planted FALSE claims, blind in form,
//! each refuted by a majority of judges, each falsified by an ADDED line of the
//! reviewed diff. Every case runs the REAL `scripts/dogfood/lib/receipt.sh`
//! against a throwaway git repo; every mutant of that script must turn the
//! table red. Nothing is grandfathered: a 0-refuted receipt without a block is
//! refused with the sentence it always had.
#![cfg(unix)]

#[path = "canary_receipts_fixture/mod.rs"]
mod fx;

use fx::cases::{answer, failures};
use fx::mutants::MUTANTS;
use fx::{manifest, SCRIPT};

#[test]
fn the_canary_shape_accepts_and_refuses_as_designed() {
    eprintln!(
        "cases {}, mutants {}",
        fx::cases::all().len(),
        MUTANTS.len()
    );
    let bad = failures(&manifest().join(SCRIPT), false);
    assert!(
        bad.is_empty(),
        "{} case(s) failed:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[test]
fn every_receipt_on_main_keeps_its_verdict() {
    let root = manifest();
    let s = fx::census_all(&root.join(SCRIPT), &root, false).unwrap_or_else(|e| panic!("{e}"));
    // The census must be real: an empty list would pass every comparison.
    assert!(s.total >= 100, "only {} receipts on main", s.total);
    assert!(s.ok >= 100, "only {} receipts read ok", s.ok);
    assert_eq!(
        s.plain + s.canary_judged + s.canary_pending,
        s.total,
        "census partition must sum to total"
    );
    eprintln!(
        "receipts {}, ok {}, refused for zero refuted {}, plain {}, canary-judged {}, canary-pending {} (pending files: [{}])",
        s.total,
        s.ok,
        s.zero,
        s.plain,
        s.canary_judged,
        s.canary_pending,
        s.pending_files.join(", ")
    );
}

/// The mutant of the census: canary receipts left out of the comparison with no
/// assertion of their own. It must pass the forged receipt that the real census
/// refuses, so the separate (commit-bound) assertion is what the cases pin.
#[test]
fn a_census_that_skips_canary_receipts_is_caught() {
    let script = manifest().join(SCRIPT);
    let forged = "census: a forged canary receipt (no judge refutes a planted claim) is refused";
    let (real, _) = answer(&script, forged);
    assert!(
        real != "ok" && !real.is_empty(),
        "real census gave {real:?}"
    );
    let (mut fx_, mut o) = (fx::base_fx(), fx::Opt::default());
    for c in fx::cases::all().into_iter().filter(|c| c.0 == forged) {
        (c.1)(&mut fx_);
        (c.2)(&mut o);
    }
    o.census_skip_canary = true;
    assert_eq!(
        fx::run(&script, &fx_, &o),
        "ok",
        "the skipping census must be blind to a forged canary receipt"
    );
}

/// Flow 2 census counts: a pre-backfill squash-merged canary receipt (`squash = 1`)
/// is counted as `canary_pending = 1` (never `ok`, never failing), while after
/// the post-merge receipt lands (`squash = 2`) it is `canary_judged = 1` and
/// `canary_pending = 0`. A plain receipt is counted as `plain = 1`.
#[test]
fn squash_merged_canary_receipt_counts_pending_before_backfill_and_judged_after() {
    let script = manifest().join(SCRIPT);
    let fx_ = fx::base_fx();
    let pre = fx::Opt {
        squash: 1,
        census: true,
        ..Default::default()
    };
    let s1 = fx::run_census(&script, &fx_, &pre).expect("pre-backfill squash census must succeed");
    assert_eq!(s1.total, 1);
    assert_eq!(s1.plain, 0);
    assert_eq!(s1.canary_judged, 0);
    assert_eq!(s1.canary_pending, 1);
    assert_eq!(s1.pending_files, vec![".quorum/x.json".to_string()]);
    assert_ne!(fx::run(&script, &fx_, &pre), "ok");

    let post = fx::Opt {
        squash: 2,
        census: true,
        ..Default::default()
    };
    let s2 = fx::run_census(&script, &fx_, &post).expect("post-merge squash census must succeed");
    assert_eq!(s2.total, 1);
    assert_eq!(s2.plain, 0);
    assert_eq!(s2.canary_judged, 1);
    assert_eq!(s2.canary_pending, 0);
    assert!(s2.pending_files.is_empty());
    assert_eq!(fx::run(&script, &fx_, &post), "ok");

    let mut plain_fx = fx::base_fx();
    plain_fx.receipt["quorum"]
        .as_object_mut()
        .expect("quorum")
        .remove("canary");
    plain_fx.receipt["quorum"]["claims_refuted"] = serde_json::json!(1);
    let plain_o = fx::Opt {
        census: true,
        ..Default::default()
    };
    let sp = fx::run_census(&script, &plain_fx, &plain_o).expect("plain census must succeed");
    assert_eq!(sp.total, 1);
    assert_eq!(sp.plain, 1);
    assert_eq!(sp.canary_judged, 0);
    assert_eq!(sp.canary_pending, 0);
}

/// The second census mutant: passing off a canary receipt whose `reviewed_head`
/// IS an ancestor of `HEAD` as pending. Both on a post-merge squash receipt
/// (`squash = 2`) and on an earlier branch/main ancestor (`squash = 0`), a
/// forged canary with a reachable head must be refused by the real census, and
/// an honest one must read `"ok"` — never `"pending"`.
#[test]
fn a_census_that_passes_off_a_reachable_head_as_pending_is_caught() {
    let script = manifest().join(SCRIPT);
    let honest = fx::base_fx();
    let mut forged = fx::base_fx();
    forged.verdicts[0]["verdicts"]["2"] = serde_json::json!("SURVIVED");
    forged.verdicts[1]["verdicts"]["2"] = serde_json::json!("SURVIVED");

    for squash in [0u8, 2u8] {
        let mut o = fx::Opt {
            squash,
            census: true,
            ..Default::default()
        };
        assert_eq!(
            fx::run(&script, &honest, &o),
            "ok",
            "honest reachable receipt (squash={squash}) must be judged ok, not pending"
        );
        let real_forged = fx::run(&script, &forged, &o);
        assert!(
            real_forged.contains("not refuted by a majority"),
            "real census must refuse forged reachable receipt (squash={squash}), got {real_forged:?}"
        );
        o.census_pending_reachable = true;
        assert_eq!(
            fx::run(&script, &forged, &o),
            "pending",
            "mutant passing off reachable head as pending hides a forged receipt (squash={squash})"
        );
    }
}

/// The other half of the skip: a receipt with NO canary block must never be
/// skipped, whatever text it holds. A plain receipt that merely carries the word
/// "canary" in a string, judged by a script whose plain-receipt verdict has
/// drifted from main (the case-insensitive waiver match leaking to plain
/// receipts), must fail the real census; a census that skips by the text
/// `"canary"` instead of by the block would let the drift through.
#[test]
fn a_plain_receipt_is_never_skipped_by_the_census() {
    let real = manifest().join(SCRIPT);
    let name = "the case-insensitive waiver match reaches receipts with no canary block";
    let mu = MUTANTS.iter().find(|m| m.name == name).expect("mutant");
    let mut text = std::fs::read_to_string(&real).expect("script");
    for (needle, repl) in mu.pairs {
        assert_eq!(text.matches(needle).count(), 1, "needle {needle:?}");
        text = text.replacen(needle, repl, 1);
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let drifted = dir.path().join("drifted.sh");
    std::fs::write(&drifted, text).expect("write");
    let mut fx_ = fx::base_fx();
    fx_.receipt["quorum"]
        .as_object_mut()
        .expect("quorum")
        .remove("canary");
    fx_.receipt["quorum"]["claims_refuted"] = serde_json::json!(1);
    fx_.receipt["Waived_note"] = serde_json::json!("canary");
    let mut o = fx::Opt {
        squash: 2,
        census: true,
        ..Default::default()
    };
    assert_eq!(
        fx::run(&real, &fx_, &o),
        "ok",
        "the shipped script keeps main's verdict"
    );
    let got = fx::run(&drifted, &fx_, &o);
    assert!(
        got.contains("the verdict changed"),
        "the real census must catch the drift on a plain receipt, got {got:?}"
    );
    o.census_skip_canary = true;
    assert_eq!(
        fx::run(&drifted, &fx_, &o),
        "ok",
        "a census that skips by text is blind to the drift (the mutant of the skip)"
    );
}

#[test]
fn every_mutant_of_the_predicate_flips_its_kill_case() {
    let src = std::fs::read_to_string(manifest().join(SCRIPT)).expect("script");
    assert!(MUTANTS.len() >= 35, "only {} mutants", MUTANTS.len());
    let dir = tempfile::tempdir().expect("tempdir");
    let mut bad = Vec::new();
    for (i, mu) in MUTANTS.iter().enumerate() {
        let mut text = src.clone();
        for (needle, repl) in mu.pairs {
            assert_eq!(
                text.matches(needle).count(),
                1,
                "mutant {i} ({}): needle must occur once: {needle:?}",
                mu.name
            );
            text = text.replacen(needle, repl, 1);
        }
        let p = dir.path().join(format!("m{i}.sh"));
        std::fs::write(&p, text).expect("write mutant");
        let (got, want) = answer(&p, mu.kill);
        // A case that wants "ok" must stop being ok; a refusal must flip.
        let flipped = if mu.flip.is_empty() {
            // A guard behind another guard: the answer must at least change.
            got != answer(&manifest().join(SCRIPT), mu.kill).0
        } else if want == "ok" {
            got != "ok"
        } else {
            got == mu.flip
        };
        if !flipped {
            bad.push(format!(
                "mutant {i} ({}): kill case {:?} wanted {want:?}, mutant gave {got:?}",
                mu.name, mu.kill
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "mutants that did not flip:\n{}",
        bad.join("\n")
    );
}

/// Callers must name the commit they vouch for, and the push gate must check a
/// present block on every path. Structural: these files are shell and Python
/// inside shell, run by CI only against a real GitHub.
#[test]
fn the_callers_bind_the_receipt_to_a_commit() {
    let read = |p: &str| std::fs::read_to_string(manifest().join(p)).expect(p);
    let store = read("scripts/dogfood/lib/store.sh");
    assert!(store.contains("dogfood_receipt_status \"$blob\" \"$head\" \"$base\" \"$head\""));
    let quorum = read("scripts/dogfood/quorum.sh");
    assert!(quorum.contains("dogfood_receipt_resolve \"$why\" \"$text\" \"$mc\" \"${mc}^1\""));
    let rel = read("scripts/dogfood/release-check.sh");
    assert!(rel.contains("receipt_status \"$blob\" \"$oid\" \"${oid}^1\""));
    let gate = read("scripts/quorum-gate.sh");
    // Detection is bash + jq (650ae482 removed the python `if "canary" in q:` branch) and
    // runs unconditionally before the python pass, so it is the every-path check.
    assert!(
        gate.contains(r#"(.quorum | has("canary"))"#),
        "block not checked on every path"
    );
    assert!(
        gate.contains("has_canary_rc=0"),
        "block not checked on every path"
    );
    assert!(!gate.contains("refuted == 0 and \"canary\""));
    assert!(gate.contains("reviewed_head is not an ancestor"));
}
