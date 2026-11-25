//! Round 7 (the review of 1a9d786): the census over the committed receipts
//! splits in two, and the Arm 5 pick and the canary shape hold to main's
//! decisions for everything that is not an honest canary receipt.
use super::cases_review::{nop, pl_text};
use super::*;
use serde_json::json;

fn refuted_one(f: &mut Fx) {
    f.receipt["quorum"]["claims_refuted"] = json!(1);
    f.receipt["quorum"]["claims_confirmed"] = json!(1);
    f.verdicts[0]["verdicts"]["1"] = json!("REFUTED");
    f.verdicts[1]["verdicts"]["1"] = json!("REFUTED");
}

fn drop_reviewed_head(s: String) -> String {
    let mut v: serde_json::Value = serde_json::from_str(&s).expect("receipt json");
    v["quorum"]["canary"]
        .as_object_mut()
        .expect("canary object")
        .remove("reviewed_head");
    v.to_string()
}

pub fn table() -> Vec<Case> {
    vec![
        (
            "census: one honest committed canary receipt is ok by the bound path",
            |_| {},
            |o| {
                o.squash = 2;
                o.census = true;
            },
            "ok",
        ),
        (
            "census: a forged canary receipt (no judge refutes a planted claim) is refused",
            |f| {
                f.verdicts[0]["verdicts"]["2"] = json!("SURVIVED");
                f.verdicts[1]["verdicts"]["2"] = json!("SURVIVED");
            },
            |o| {
                o.squash = 2;
                o.census = true;
            },
            "!ok",
        ),
        (
            "census: a canary receipt whose commit does not resolve is pending, not ok",
            |f| canary(f)["reviewed_head"] = json!("0123456789012345678901234567890123456789"),
            |o| {
                o.squash = 2;
                o.census = true;
            },
            "pending",
        ),
        (
            "census: a receipt with no canary block is judged without commits and keeps main's verdict",
            |f| {
                f.receipt["quorum"].as_object_mut().expect("quorum").remove("canary");
                f.receipt["quorum"]["claims_refuted"] = json!(1);
            },
            |o| {
                o.squash = 2;
                o.census = true;
            },
            "ok",
        ),
        (
            "a receipt with no canary block and a Waived_note key keeps main's verdict",
            |f| {
                f.receipt["quorum"].as_object_mut().expect("quorum").remove("canary");
                f.receipt["quorum"]["claims_refuted"] = json!(1);
                f.receipt["Waived_note"] = json!("x");
            },
            nop,
            "ok",
        ),
        (
            "a grep without -P support is refused as UNMEASURED, never ok",
            |_| {},
            |o| o.shim = Some(("grep", "rej:*P*")),
            "UNMEASURED",
        ),
        (
            "a planted text padded with internal blanks to 20 characters is refused",
            |f| pl_text(f, 0, "a    b    c        d"),
            nop,
            "20+ non-blank chars",
        ),
        (
            "census: an honest multi-commit branch canary receipt is ok",
            |_| {},
            |o| {
                o.multi_commit = true;
                o.census = true;
            },
            "ok",
        ),
        (
            "census: a forged multi-commit branch canary receipt (missed canary) is refused",
            |f| {
                f.verdicts[0]["verdicts"]["2"] = json!("SURVIVED");
                f.verdicts[1]["verdicts"]["2"] = json!("SURVIVED");
            },
            |o| {
                o.multi_commit = true;
                o.census = true;
            },
            "not refuted by a majority",
        ),
        (
            "census: a multi-commit branch canary receipt whose reviewed_base is not an ancestor of reviewed_head is refused",
            |f| canary(f)["reviewed_base"] = json!("@OTHER"),
            |o| {
                o.multi_commit = true;
                o.census = true;
            },
            "is not an ancestor",
        ),
        (
            "census: a pre-backfill squash-merged canary receipt whose reviewed_head is not on HEAD is pending",
            |_| {},
            |o| {
                o.squash = 1;
                o.census = true;
            },
            "pending",
        ),
        (
            "census: a pre-backfill squash-merged canary receipt with a forged claims_refuted string is refused",
            |f| f.receipt["quorum"]["claims_refuted"] = json!("1"),
            |o| {
                o.squash = 1;
                o.census = true;
            },
            "invalid claims_refuted",
        ),
        (
            "census: a pre-backfill squash-merged canary receipt with claims_refuted = 1 next to a waiver is refused",
            |f| {
                f.receipt["quorum"]["claims_refuted"] = json!(1);
                f.receipt["waived"] = json!({"reason": "x"});
            },
            |o| {
                o.squash = 1;
                o.census = true;
            },
            "not 'ok'",
        ),
        (
            "census: a pre-backfill squash-merged canary receipt with claims_refuted = 0 next to a waiver is refused",
            |f| f.receipt["waived"] = json!({"reason": "x"}),
            |o| {
                o.squash = 1;
                o.census = true;
            },
            "not the zero-refuted refusal",
        ),
        (
            "census: a post-merge squash-merged canary receipt with squash = 2 is judged, not pending",
            |_| {},
            |o| {
                o.squash = 2;
                o.census = true;
            },
            "ok",
        ),
        (
            "census: a forged canary receipt whose reviewed_head is an ancestor of HEAD is refused, not pending",
            |f| {
                f.verdicts[0]["verdicts"]["2"] = json!("SURVIVED");
                f.verdicts[1]["verdicts"]["2"] = json!("SURVIVED");
            },
            |o| o.census = true,
            "not refuted by a majority",
        ),
        (
            "census: a canary receipt with a missing reviewed_head is refused",
            |_| {},
            |o| {
                o.census = true;
                o.rec_edit = Some(drop_reviewed_head);
            },
            "reviewed_head",
        ),
        (
            "census: a canary receipt with a non-string reviewed_head is refused",
            |f| canary(f)["reviewed_head"] = json!(123),
            |o| {
                o.squash = 2;
                o.census = true;
            },
            "reviewed_head",
        ),
        (
            "census: a canary receipt whose reviewed_base does not resolve is refused",
            |f| canary(f)["reviewed_base"] = json!("0123456789012345678901234567890123456789"),
            |o| {
                o.squash = 2;
                o.census = true;
            },
            "does not resolve in history",
        ),
        (
            "census: an honest canary receipt with claims_refuted >= 1 is ok",
            refuted_one,
            |o| {
                o.squash = 2;
                o.census = true;
            },
            "ok",
        ),
        (
            "census: a forged canary receipt with claims_refuted >= 1 (missed canary) is refused",
            |f| {
                refuted_one(f);
                f.verdicts[0]["verdicts"]["2"] = json!("SURVIVED");
                f.verdicts[1]["verdicts"]["2"] = json!("SURVIVED");
            },
            |o| {
                o.squash = 2;
                o.census = true;
            },
            "not refuted by a majority",
        ),
    ]
}
