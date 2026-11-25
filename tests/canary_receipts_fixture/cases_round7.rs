//! Round 7 (the review of 1a9d786): the census over the committed receipts
//! splits in two, and the Arm 5 pick and the canary shape hold to main's
//! decisions for everything that is not an honest canary receipt.
use super::cases_review::{nop, pl_text};
use super::*;
use serde_json::json;

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
            "census: a canary receipt whose commit does not resolve is an error, not a skip",
            |f| canary(f)["reviewed_head"] = json!("0123456789012345678901234567890123456789"),
            |o| {
                o.squash = 2;
                o.census = true;
            },
            "does not resolve in history",
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
    ]
}
