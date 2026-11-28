//! The head-bound store receipts (forjar#876, forjar#1327) the fixtures write for
//! gates R and E. Split out of `mod.rs` to keep it under the 500-line gate.

use super::{Fixture, PR};
use std::path::PathBuf;

/// A head-bound store receipt (forjar#876) for [`PR`] that clears every floor for `head`.
pub(crate) fn store_receipt(head: &str, lanes: &str, judges: u32, author: &str) -> String {
    store_receipt_for(PR, head, lanes, judges, author)
}

/// The same receipt for PR `pr`: the store gate binds a receipt to its own PR
/// number, so a window of many PRs needs one receipt per number.
pub(crate) fn store_receipt_for(
    pr: u32,
    head: &str,
    lanes: &str,
    judges: u32,
    author: &str,
) -> String {
    let v = |l: &str, m: &str| {
        format!(r#"{{"lane":"{l}","model":"{m}","verdict":"PASS","blockers":[]}}"#)
    };
    format!(
        r#"{{"repo":"paiml/forjar","pr":{pr},"sha":"{head}","author_model":"{author}","verdicts":[{},{},{}],"quorum":{{"lanes":{lanes},"judges":{judges},"refuters_per_claim":3,"claims_refuted":2}},"evidence":{{"files":["a.md"]}}}}"#,
        v("a", "claude-opus-5-5"),
        v("b", "claude-haiku-5-5"),
        v("c", "gemini-3")
    )
}

/// Where the store receipt for the fixture PR's head lives.
pub(crate) fn store_receipt_path(fx: &Fixture) -> PathBuf {
    fx.store
        .join(format!("quorum/paiml/forjar/{PR}/{}.json", fx.head))
}
