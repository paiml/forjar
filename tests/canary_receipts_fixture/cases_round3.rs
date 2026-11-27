//! The cases that reproduce the second review of f7a2012: ids and hashes with a
//! Second half of the round-3 review cases (split to stay under the file-health limit).
//! trailing newline, path:line forms, a falsifier inside the receipt's own
//! evidence, honest prose that held the letters of a marker word, and the
//! squash-merge shape. Each is a receipt the f7a2012 predicate got wrong.
use super::cases_review::{nop, pl, raw_claims, rename, s};
use super::*;

fn newline_id(f: &mut Fx) {
    rename(f, "1", "1\n");
    canary(f)["real_claim_ids"] = json!(["1\n", "3"]);
}

fn own_evidence(f: &mut Fx) {
    canary(f)["reviewed_head"] = json!("@EVID");
    pl(
        f,
        0,
        "falsified_by",
        json!(".quorum/evidence/claims.json:1"),
    );
    pl(
        f,
        1,
        "falsified_by",
        json!(".quorum/evidence/claims.json:2"),
    );
}

pub(super) fn claim0(f: &mut Fx, t: &str) {
    set_claims(f, 0, "text", json!(t));
}

pub fn table() -> Vec<Case> {
    let mut v: Vec<Case> = vec![
        // 4/5. canonical path:line, anchored ids and hashes.
        (
            "a claim id with a trailing newline is refused",
            newline_id,
            nop,
            "plain number",
        ),
        (
            "a planted id with a trailing newline is refused",
            |f| {
                pl(f, 0, "id", json!("2\n"));
                canary(f)["caught"] = json!(["2\n", "4"]);
            },
            nop,
            "plain numeric id",
        ),
        (
            "a reviewed_head with a trailing newline is refused",
            |f| canary(f)["reviewed_head"] = json!("@HEAD\n"),
            nop,
            "not a 40-hex",
        ),
        (
            "a falsified_by line with a leading zero is refused",
            |f| pl(f, 1, "falsified_by", json!("src/a.rs:05")),
            nop,
            "falsified_by (path:line",
        ),
        (
            "a falsified_by with a trailing newline is refused",
            |f| pl(f, 1, "falsified_by", json!("src/a.rs:5\n")),
            nop,
            "falsified_by (path:line",
        ),
        (
            "a falsified_by with a ./ prefix is refused",
            |f| pl(f, 0, "falsified_by", json!("./src/a.rs:4")),
            nop,
            "falsified_by (path:line",
        ),
        (
            "a falsified_by with line zero is refused",
            |f| pl(f, 0, "falsified_by", json!("src/a.rs:0")),
            nop,
            "falsified_by (path:line",
        ),
        // 3. the receipt's own evidence is no falsifying fact.
        (
            "a falsifier under .quorum/ on a deletion-only diff is refused",
            own_evidence,
            |o| {
                o.delete_only = true;
                o.evid_head = true;
                o.subj_head = s("@EVID");
            },
            "lies under .quorum/",
        ),
        (
            "a falsifier under .pmat/ is refused",
            |f| pl(f, 0, "falsified_by", json!(".pmat/baseline.json:1")),
            nop,
            "lies under .quorum/ or .pmat/",
        ),
        (
            "a falsifier in a file the receipt lists as evidence is refused",
            |_| {},
            |o| o.extra_ev = Some("src/a.rs"),
            "names a file the receipt lists as evidence",
        ),
        // 6. honest prose is not a marker; disguised markers still are.
        (
            "an honest claim about the forjar plan is accepted",
            |f| {
                claim0(
                    f,
                    "The forjar plan takes the target state and adds added_one to src/a.rs.",
                )
            },
            nop,
            "ok",
        ),
        (
            "a claim ending in plan before a claim starting with The is accepted",
            |f| claim0(f, "The change updates the execution plan."),
            nop,
            "ok",
        ),
        (
            "a claim about a planned change is accepted",
            |f| {
                claim0(
                    f,
                    "The planned change adds added_one to src/a.rs at line four.",
                )
            },
            nop,
            "ok",
        ),
        (
            "a separated marker word is refused",
            |_| {},
            |o| o.claims_raw = raw_claims("The c a n a r y claim adds added_two to src/a.rs."),
            "were not blind",
        ),
        (
            "a plural marker word is refused",
            |f| claim0(f, "Two decoys were added to src/a.rs at line four."),
            nop,
            "were not blind",
        ),
        (
            "a newline inside a claim string is refused",
            |_| {},
            |o| o.claims_raw = raw_claims("The change adds added_two\\nto src/a.rs."),
            "non-ASCII or control",
        ),
        // non-blocking.
        (
            "an evidence file that is not committed anywhere is refused",
            |_| {},
            |o| o.no_commit = Some("verdicts.json"),
            "is not committed at any of",
        ),
        (
            "an empty lane list is refused",
            |f| f.receipt["quorum"]["lanes"] = json!([]),
            nop,
            "quorum.lanes = 0",
        ),
        (
            "a blank lane name is refused",
            |f| f.receipt["quorum"]["lanes"] = json!(["a", "b", " "]),
            nop,
            "blank or non-string lane name",
        ),
        (
            "duplicate lane names are refused",
            |f| f.receipt["quorum"]["lanes"] = json!(["a", "A ", "c"]),
            nop,
            "lanes names are not distinct",
        ),
        (
            "a duplicate planted id is refused",
            |f| pl(f, 1, "id", json!("2")),
            nop,
            "planted[].id are not distinct",
        ),
        (
            "a duplicate planted text is refused",
            |f| pl(f, 1, "text", json!(P_DELETE)),
            nop,
            "planted[].text are not distinct",
        ),
        (
            "a planted position beyond the claims file is refused",
            |f| pl(f, 1, "position", json!(9)),
            nop,
            "lies beyond the claims file",
        ),
        (
            "real claims out of order in the claims file are refused",
            |f| f.claims.as_array_mut().expect("claims").swap(0, 2),
            nop,
            "not quorum.canary.real_claim_ids in order",
        ),
        (
            "a verdict that is neither REFUTED nor SURVIVED is refused",
            |f| f.verdicts[0]["verdicts"]["1"] = json!("MAYBE"),
            nop,
            "every judge must answer every claim",
        ),
        // 2. squash merges: the PR-branch receipt is bound to a commit that is
        // not on main; the receipt committed after the merge is.
        (
            "a squash-merged canary PR with only its pre-merge receipt is refused",
            |_| {},
            |o| o.squash = 1,
            "reviewed_head",
        ),
        (
            "a squash-merged canary PR is judged on its post-merge receipt",
            |_| {},
            |o| o.squash = 2,
            "ok",
        ),
    ];
    v.extend(super::cases_round3b::table());
    v
}
