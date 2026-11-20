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

use fx::cases::failures;
use fx::{manifest, OLD_ZERO, SCRIPT};
use std::process::Command;

#[test]
fn the_canary_shape_accepts_and_refuses_as_designed() {
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

/// (needle, replacement): one-token breaks of the predicate, each applied by
/// hand to a copy of the script. Each needle occurs exactly once.
const MUTANTS: &[(&str, &str)] = &[
    (
        "(hascanary | not)\n    then \"quorum.claims_refuted",
        "false\n    then \"quorum.claims_refuted",
    ),
    (
        "chk(($c.planted | length) < 2;",
        "chk(($c.planted | length) < 1;",
    ),
    (
        "or (($c.caught | sort) != ([$c.planted[].id] | sort))",
        "or false",
    ),
    ("(refd($v; $id) * 2) > ($v | length)", "refd($v; $id) >= 1"),
    (
        "if [ \"$rc\" -eq 0 ]; then\n    printf 'quorum.canary.claims_file carries",
        "if [ \"$rc\" -eq 99 ]; then\n    printf 'quorum.canary.claims_file carries",
    ),
    ("($q | has(\"claims_reviewed\")) and", "false and"),
    ("if [ \"$got\" != \"$want\" ]; then", "if false; then"),
    ("== ($c.planted | proj)", "!= ($c.planted | proj)"),
    (
        "test(\"refuter|judge\"; \"i\")",
        "test(\"refuterx|judgex\"; \"i\")",
    ),
    (
        "any($c.real_claim_ids[]; . == $i)",
        "any($c.real_claim_ids[]; . == \"zz\")",
    ),
    (
        "    if [ \"$rc\" -ne 0 ]; then\n      printf 'quorum.canary.planted[%s]",
        "    if [ \"$rc\" -ne 99 ]; then\n      printf 'quorum.canary.planted[%s]",
    ),
    ("chk(($v | length) != $q.judges;", "chk(false;"),
    ("test(\"waiv|override\")", "test(\"waivx|override\")"),
    (
        "(keys == [\"id\", \"text\"]) and (.id | type",
        "(keys | length > 0) and (.id | type",
    ),
    ("$cl[$p.position - 1] !=", "$cl[0] !="),
    (
        "if [ \"$why\" = canary ]; then",
        "if [ \"$why\" = canaryx ]; then",
    ),
    ("chk($q.claims_refuted != $realref;", "chk(false;"),
    (
        "chk(($c.real_claim_ids | length) < 1;",
        "chk(($c.real_claim_ids | length) < 0;",
    ),
    (
        "chk(([$c.planted[].position] | unique | length) != ($c.planted | length);",
        "chk(false;",
    ),
    (
        "if (try ((.planted | proj) == ($c.planted | proj)) catch false)",
        "if true",
    ),
];

#[test]
fn every_mutant_of_the_predicate_turns_the_table_red() {
    let src = std::fs::read_to_string(manifest().join(SCRIPT)).expect("script");
    assert!(MUTANTS.len() >= 12);
    let dir = tempfile::tempdir().expect("tempdir");
    let mut survived = Vec::new();
    for (i, (needle, repl)) in MUTANTS.iter().enumerate() {
        assert_eq!(
            src.matches(needle).count(),
            1,
            "mutant {i}: needle must occur once: {needle:?}"
        );
        let p = dir.path().join(format!("m{i}.sh"));
        std::fs::write(&p, src.replacen(needle, repl, 1)).expect("write mutant");
        if failures(&p, true).is_empty() {
            survived.push(format!("mutant {i}: {needle:?}"));
        }
    }
    assert!(
        survived.is_empty(),
        "mutants the table did not catch:\n{}",
        survived.join("\n")
    );
}
