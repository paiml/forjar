//! PMAT-1170: a canary-shaped receipt for an ALREADY MERGED squash PR travels
//! the path every backfill receipt travels today, end to end, on the real
//! scripts.
//!
//! The path (read from ci.yml, scripts/hooks/pre-push, quorum.yml and the
//! receipts of PR #1223):
//! 1. a pack branch commits `.quorum/<merged-pr-branch>.json` + evidence AFTER
//!    the merge; the pack is its own PR;
//! 2. the push gate (`quorum-gate.sh`, a WARN-only pre-push step) and the PR
//!    job (`quorum-store-gate.sh`) judge the PACK'S OWN receipt only; neither
//!    reads the backfill files the pack carries;
//! 3. the receipt for merged PR N is judged at the release: gate E
//!    (`quorum.sh`) and Arm 5 (`release-check.sh`) read it at HEAD, bound to
//!    N's merge commit and its first parent.
//!
//! Here: a merged squash commit S (no receipt), then a pack commit adding N's
//! canary receipt and evidence. Both release gates must accept; with one
//! planted claim missed both must refuse. Each mutant is a changed script that
//! must flip a case.
#![cfg(unix)]
#![allow(dead_code)]

#[path = "canary_receipts_fixture/mod.rs"]
mod cf;
#[path = "dogfood_gates_harness/mod.rs"]
mod ge;
#[path = "release_check_fixture/mod.rs"]
mod rc;

use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

/// Rebuild the window's one commit as a squash commit S with six added lines and
/// no receipt, then commit the pack: N's canary receipt and its evidence.
/// Returns S. `missed`: no judge refutes planted claim 2.
fn land(root: &Path, slug: &str, missed: bool, date: Option<&str>) -> String {
    cf::git(root, &["reset", "-q", "--hard", "HEAD~1"]);
    cf::put(root, "shipped.txt", "l1\nl2\nl3\nl4\nl5\nl6\n");
    cf::git(root, &["add", "-A"]);
    let commit = |msg: &str| {
        let mut c = Command::new("git");
        c.args(["commit", "-qm", msg]).current_dir(root);
        if let Some(d) = date {
            c.env("GIT_AUTHOR_DATE", d).env("GIT_COMMITTER_DATE", d);
        }
        assert!(c.status().expect("git").success());
    };
    commit("squash-merged work (#77)");
    let s = cf::git(root, &["rev-parse", "HEAD"]);
    let base = cf::git(root, &["rev-parse", "HEAD~1"]);
    let mut f = cf::base_fx();
    let planted = json!([
        {"id": "2", "text": cf::P_DELETE, "falsified_by": "shipped.txt:4", "position": 2},
        {"id": "4", "text": cf::P_FIRST, "falsified_by": "shipped.txt:5", "position": 4}
    ]);
    f.plan = json!({"planted": planted});
    f.receipt["quorum"]["canary"]["planted"] = planted;
    f.receipt["quorum"]["canary"]["reviewed_head"] = json!(s);
    f.receipt["quorum"]["canary"]["reviewed_base"] = json!(base);
    if missed {
        for v in f.verdicts.as_array_mut().expect("verdicts") {
            v["verdicts"]["2"] = json!("SURVIVED");
        }
    }
    let mut files = Vec::new();
    for (name, v, role) in [
        ("claims.json", &f.claims, "claims-blind"),
        ("plan.json", &f.plan, "canary-plan"),
        ("verdicts.json", &f.verdicts, "judge-verdicts"),
    ] {
        let body = serde_json::to_string_pretty(v).expect("json");
        cf::put(root, &format!(".quorum/evidence/{name}"), &body);
        files.push(json!({"path": format!(".quorum/evidence/{name}"), "roles": [role], "sha256": cf::sha256(&body)}));
    }
    f.receipt["evidence"]["files"] = Value::Array(files);
    cf::put(
        root,
        &format!(".quorum/{slug}.json"),
        &f.receipt.to_string(),
    );
    cf::git(root, &["add", "-A"]);
    commit("post-merge receipts pack");
    s
}

fn gh_json(s: &str, head_ref: &str) -> String {
    format!(
        r#"[{{"number":77,"mergedAt":"2026-09-05T00:00:00Z","mergeCommit":{{"oid":"{s}"}},"headRefName":"{head_ref}","title":"t","body":"b"}}]"#
    )
}

/// What each release gate says, as (arm5 green, arm5 text, gate E green, gate E text).
/// `mutate` rewrites (file, from, to) in the fixtures' working trees first.
fn both(missed: bool, mutate: &[(&str, &str, &str)]) -> (bool, String, bool, String) {
    let r = rc::fixture(false, None);
    let s = land(&r.root, &rc::slug(), missed, Some("2026-09-10T00:00:00Z"));
    let g = ge::fixture(None, None);
    let s2 = land(&g.root, &ge::slug(), missed, None);
    for (file, from, to) in mutate {
        for root in [&r.root, &g.root] {
            let p = root.join(file);
            if let Ok(t) = std::fs::read_to_string(&p) {
                assert_eq!(t.matches(from).count(), 1, "needle in {file}: {from}");
                std::fs::write(&p, t.replace(from, to)).expect("write mutant");
            }
        }
    }
    let rr = rc::run(
        &r,
        &rc::stub_gh(
            r.root.parent().expect("dir"),
            "gh-s",
            &gh_json(&s, rc::HEAD_REF),
        ),
    );
    let gr = ge::run(
        &g,
        "quorum.sh",
        &ge::stub_gh(
            g.root.parent().expect("dir"),
            "gh-s",
            &gh_json(&s2, ge::HEAD_REF),
        ),
    );
    (
        rr.code == 0 && rr.text.contains("#77 feat-pr-77 receipt=ok"),
        rr.text,
        gr.code == 0
            && gr
                .text
                .contains(&format!("GATE E #77 {} ok", ge::quorum_receipt_path())),
        gr.text,
    )
}

#[test]
fn a_canary_receipt_in_a_pack_after_the_squash_merge_is_ok_at_arm5_and_gate_e() {
    let (a5, t5, e, te) = both(false, &[]);
    assert!(a5, "Arm 5 must count N as receipt=ok:\n{t5}");
    assert!(e, "gate E must accept N's receipt:\n{te}");
}

#[test]
fn a_missed_planted_claim_in_that_pack_is_refused_at_arm5_and_gate_e() {
    let (a5, t5, e, te) = both(true, &[]);
    assert!(!a5 && t5.contains("receipt=thin"), "Arm 5:\n{t5}");
    assert!(
        !e && te.contains("not refuted by a majority"),
        "gate E:\n{te}"
    );
}

/// (name, file, needle, replacement, the case it must flip: missed?)
const MUTANTS: [(&str, &str, &str, &str, bool); 5] = [
    (
        "Arm 5 reads only the merge-commit copy of the receipt",
        "scripts/dogfood/release-check.sh",
        "blob=\"$(dogfood_receipt_pick \"$oid\" \"$path\")\"",
        "blob=\"$(git show \"${oid}:${path}\" 2>/dev/null || true)\"",
        false,
    ),
    (
        "Arm 5 reads evidence only at the merge commit",
        "scripts/dogfood/release-check.sh",
        "\"$blob\" \"$oid\" \"${oid}^1\" \"$oid\" HEAD)",
        "\"$blob\" \"$oid\" \"${oid}^1\" \"$oid\")",
        false,
    ),
    (
        "gate E reads evidence only at the merge commit",
        "scripts/dogfood/quorum.sh",
        "\"$mc\" \"${mc}^1\" \"$mc\" HEAD)",
        "\"$mc\" \"${mc}^1\" \"$mc\")",
        false,
    ),
    (
        "the predicate does not require a judge majority on a planted claim",
        "scripts/dogfood/lib/receipt.sh",
        "chk(any($pid[]; majority($v; .) | not);",
        "chk(false;",
        true,
    ),
    (
        "gate E binds the falsifier diff to the merge commit itself, not its parent",
        "scripts/dogfood/quorum.sh",
        "\"$mc\" \"${mc}^1\" \"$mc\" HEAD)",
        "\"$mc\" \"$mc\" \"$mc\" HEAD)",
        false,
    ),
];

#[test]
fn every_mutant_of_the_post_merge_path_flips_its_case() {
    eprintln!("post-merge path mutants {}", MUTANTS.len());
    let mut bad = Vec::new();
    for (name, file, from, to, missed) in MUTANTS {
        let (a5, _, e, _) = both(missed, &[(file, from, to)]);
        // The mutated script must change its gate's answer: a green case goes
        // red, a refused case goes green. Only the gate that reads the file counts.
        let (gate_ok, flipped) = if file.ends_with("release-check.sh") {
            (a5, a5 == missed)
        } else if file.ends_with("quorum.sh") {
            (e, e == missed)
        } else {
            (a5 && e, a5 == missed && e == missed)
        };
        let _ = gate_ok;
        if !flipped {
            bad.push(format!("{name}: arm5={a5} gateE={e} missed={missed}"));
        }
    }
    assert!(
        bad.is_empty(),
        "mutants that did not flip:\n{}",
        bad.join("\n")
    );
}
