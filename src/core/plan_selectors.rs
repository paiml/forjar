//! Refs #358 — the selectors a saved plan was produced under.
//!
//! # Why a plan file has to say this
//!
//! `apply --plan-file` re-plans and compares, because a seal proves a document
//! was not edited, never that what it says is true. That comparison only works
//! if forjar can compute the plan the file CLAIMS to be. Without this record it
//! cannot, because two documents that must be treated differently are
//! byte-identical:
//!
//! ```text
//!   $ forjar plan -r bravo --out narrow.json     # bravo already converged
//!   changes: [ bravo: no_op ]   counters 0/0/0/1
//!
//!   # an adversary takes an honest whole-stack plan (alpha: create,
//!   # bravo: no_op), DELETES the alpha line, and re-seals:
//!   changes: [ bravo: no_op ]   counters 0/0/0/1
//! ```
//!
//! Measured on the un-fixed build: the first is a legitimate no-op apply that
//! must exit 0, and the second printed `Plan has no changes to apply.` and
//! exited 0 with a create still pending. Nothing in the document distinguished
//! them, so no predicate over the document could either — which is why the
//! first attempt at this check keyed off `plan.changes.is_empty()` and the
//! second would have keyed off "the scope is empty" and refused the legitimate
//! one.
//!
//! Recording the selectors gives the comparison a fixed point: re-plan under
//! the plan's OWN selectors and require agreement in both directions. The
//! narrow plan then reproduces exactly, and the forged one is contradicted by
//! an `alpha: create` the planner produces and the body does not name.
//!
//! # This is sealed, not decorative
//!
//! The record is folded into the seal's diff leg (see
//! [`crate::core::plan_seal::digest::diff_leg`]), so editing it is a
//! `PLAN_HASH_MISMATCH` exactly like editing the change list. An adversary can
//! of course re-seal — the seal is unkeyed and says so — but they can no longer
//! do it INVISIBLY: a forgery must now declare itself as a filtered plan, and
//! `apply --plan-file` prints that declaration and what it leaves undone.

use serde::{Deserialize, Serialize};

/// The four filters `forjar plan` accepts, as saved into a plan file.
///
/// Every field is always serialised, `null` included: an absent key and a null
/// key must hash the same, or the record's contribution to the seal would
/// depend on which writer produced it.
///
/// `deny_unknown_fields` — a fifth selector invented by an editor is refused
/// rather than silently ignored, which would make the re-plan run under weaker
/// filters than the document claims.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanSelectors {
    /// `plan -m` — one machine.
    pub machine: Option<String>,
    /// `plan -r` — one resource id.
    pub resource: Option<String>,
    /// `plan -t` — one tag.
    pub tag: Option<String>,
    /// `plan -g` — one resource group.
    pub group: Option<String>,
}

impl PlanSelectors {
    /// Build from the four `Option<&str>` a command already holds.
    pub fn new(
        machine: Option<&str>,
        resource: Option<&str>,
        tag: Option<&str>,
        group: Option<&str>,
    ) -> Self {
        Self {
            machine: machine.map(String::from),
            resource: resource.map(String::from),
            tag: tag.map(String::from),
            group: group.map(String::from),
        }
    }

    /// True when the plan covers the whole config.
    pub fn is_unfiltered(&self) -> bool {
        self == &Self::default()
    }

    /// The flags as the operator would have typed them, for messages.
    ///
    /// `None` when nothing was filtered, so a caller can say "this plan is
    /// filtered (…)" or say nothing at all, without composing an empty
    /// parenthetical.
    pub fn describe(&self) -> Option<String> {
        let parts: Vec<String> = [
            ("-m", &self.machine),
            ("-r", &self.resource),
            ("-t", &self.tag),
            ("-g", &self.group),
        ]
        .into_iter()
        .filter_map(|(flag, value)| value.as_ref().map(|v| format!("{flag} {v}")))
        .collect();
        (!parts.is_empty()).then(|| parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_record_is_unfiltered_and_describes_nothing() {
        let sel = PlanSelectors::default();
        assert!(sel.is_unfiltered());
        assert_eq!(sel.describe(), None);
    }

    #[test]
    fn one_selector_makes_it_filtered() {
        let sel = PlanSelectors::new(None, Some("bravo"), None, None);
        assert!(!sel.is_unfiltered());
        assert_eq!(sel.describe().as_deref(), Some("-r bravo"));
    }

    #[test]
    fn every_selector_is_described_in_flag_order() {
        let sel = PlanSelectors::new(Some("web"), Some("bravo"), Some("db"), Some("core"));
        assert_eq!(
            sel.describe().as_deref(),
            Some("-m web -r bravo -t db -g core")
        );
    }

    /// A null field and an absent field must produce the SAME record, or two
    /// honest writers would seal the same plan to two different diff legs.
    #[test]
    fn an_absent_selector_reads_back_as_a_null_one() {
        let from_null: PlanSelectors =
            serde_json::from_str(r#"{"machine":null,"resource":"bravo","tag":null,"group":null}"#)
                .expect("null form");
        let from_absent: PlanSelectors =
            serde_json::from_str(r#"{"resource":"bravo"}"#).expect("absent form");
        assert_eq!(from_null, from_absent);
    }

    /// The serialised form is what gets hashed, so it must be total: a writer
    /// that omitted its nulls would seal a different byte string.
    #[test]
    fn every_field_is_serialised_even_when_none() {
        let json = serde_json::to_string(&PlanSelectors::default()).expect("render");
        assert_eq!(
            json,
            r#"{"machine":null,"resource":null,"tag":null,"group":null}"#
        );
    }

    #[test]
    fn a_selector_this_build_does_not_know_is_refused() {
        let err = serde_json::from_str::<PlanSelectors>(r#"{"arch":"x86_64"}"#)
            .expect_err("an unknown selector must not be silently dropped");
        assert!(err.to_string().contains("arch"), "{err}");
    }
}
