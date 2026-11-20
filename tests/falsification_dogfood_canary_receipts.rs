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
use fx::{manifest, OLD_ZERO, SCRIPT};
use std::process::Command;

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

/// The jq program of main at 9f158fad, verbatim: what every receipt that
/// carries no canary key must still be told.
const OLD_JQ: &str = r#"
  def num($v): if ($v | type) == "number" then $v else -1 end;
  def len($v): if ($v | type) == "array" then ($v | length) else -1 end;
  if ([paths | .[-1]? | tostring] | any(test("waiv|override")))
    then "carries a waiver/override key: a waived quorum is an unrefuted claim, not a passed one"
  elif len(.quorum.lanes) < 3
    then "quorum.lanes = \(len(.quorum.lanes)), below the floor of 3 that scripts/quorum-gate.sh enforces at push time"
  elif num(.quorum.judges) < 3
    then "quorum.judges = \(num(.quorum.judges)), below the floor of 3: a majority needs three"
  elif num(.quorum.refuters_per_claim) < 3
    then "quorum.refuters_per_claim = \(num(.quorum.refuters_per_claim)), below the floor of 3: a claim one refuter failed to kill only met one refuter"
  elif num(.quorum.claims_refuted) < 1
    then "quorum.claims_refuted = \(num(.quorum.claims_refuted)): a round that killed no claim did not hunt, which is the vacuous receipt this floor exists to refuse"
  elif len(.evidence.files) < 1
    then "evidence.files is empty: a verdict that cites no file is an opinion"
  else "ok"
  end
"#;

#[test]
fn every_receipt_on_main_keeps_its_verdict() {
    let root = manifest();
    let mut list = String::new();
    for e in std::fs::read_dir(root.join(".quorum")).expect("quorum dir") {
        let p = e.expect("entry").path();
        if p.extension().is_some_and(|x| x == "json") {
            list.push_str(&format!(
                ".quorum/{}\n",
                p.file_name().unwrap().to_string_lossy()
            ));
        }
    }
    let (mut total, mut ok, mut zero) = (0, 0, 0);
    for rel in list.lines().filter(|l| !l.contains("/evidence/")) {
        let body = std::fs::read_to_string(root.join(rel)).expect("receipt");
        let old = jq(&body, OLD_JQ);
        let new = Command::new("bash")
            .args(["-c", r#". "$1"; dogfood_receipt_status "$(cat "$2")""#, "x"])
            .arg(root.join(SCRIPT))
            .arg(root.join(rel))
            .current_dir(&root)
            .output()
            .expect("bash");
        let new = String::from_utf8_lossy(&new.stdout).trim().to_string();
        assert_eq!(new, old, "{rel}: the verdict changed");
        total += 1;
        ok += usize::from(old == "ok");
        zero += usize::from(old == OLD_ZERO);
    }
    // The census must be real: an empty list would pass every comparison.
    assert!(total >= 100, "only {total} receipts on main");
    assert!(ok >= 100, "only {ok} receipts read ok");
    eprintln!("receipts {total}, ok {ok}, refused for zero refuted {zero}");
}

fn jq(body: &str, prog: &str) -> String {
    let mut ch = Command::new("jq")
        .args(["-r", prog])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("jq");
    std::io::Write::write_all(&mut ch.stdin.take().expect("stdin"), body.as_bytes()).expect("feed");
    String::from_utf8_lossy(&ch.wait_with_output().expect("wait").stdout)
        .trim()
        .to_string()
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
    assert!(
        gate.contains("if \"canary\" in q:"),
        "block not checked on every path"
    );
    assert!(!gate.contains("refuted == 0 and \"canary\""));
    assert!(gate.contains("reviewed_head is not an ancestor"));
}
