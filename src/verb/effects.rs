//! Effect classification — the one property clap cannot tell us.
//!
//! See [`super::spec::Effects`] for why the default is `Mutating`.

use super::spec::Effects;

/// Verbs that run a server or an interactive session and therefore cannot be
/// invoked *through* a server. Dispatching one would recurse without bound.
pub const TRANSPORT: &[&str] = &["lsp", "mcp", "serve", "watch"];

/// Verbs that read configuration or state and write nothing outside their own
/// stdout and stderr.
///
/// Membership is the claim "this verb does not change the world". It is checked
/// two ways: [`allowlist_is_live`] fails if a name here is not a real verb, and
/// the read-only e2e sweep runs each of these against a fixture and fails if
/// the working tree changes.
pub const READ_ONLY: &[&str] = &[
    "audit",
    "catalog-list",
    "cbom",
    "compare",
    "complexity",
    "compliance",
    "contracts",
    "cost-estimate",
    "cross-deps",
    "diff",
    "doctor",
    "env",
    "env-diff",
    "explain",
    "graph",
    "history",
    "impact",
    "inventory",
    "lineage",
    "lint",
    "lock-info",
    "lock-stats",
    "lock-verify",
    "lock-verify-chain",
    "lock-verify-hmac",
    "lock-verify-schema",
    "lock-verify-sig",
    "model-card",
    "output",
    "plan",
    "plan-compact",
    "policy",
    "policy-coverage",
    "preservation",
    "privilege-analysis",
    "query",
    "registry-list",
    "sbom",
    "schema",
    "score",
    "security-scan",
    "show",
    "stack-diff",
    "stack-graph",
    "state-list",
    "status",
    "suggest",
    "template",
    "validate",
];

/// Classify a verb by name.
///
/// Unknown names are [`Effects::Mutating`]: a verb added without being
/// classified is treated as dangerous, never as safe.
#[must_use]
pub fn classify(name: &str) -> Effects {
    if TRANSPORT.contains(&name) {
        Effects::Transport
    } else if READ_ONLY.contains(&name) {
        Effects::ReadOnly
    } else {
        Effects::Mutating
    }
}

/// Check that every allowlisted name is a verb that still exists.
///
/// Returns the stale names. A rename or removal leaves an entry here pointing
/// at nothing; without this check the list would rot silently and its next
/// reader would trust a claim about a verb that is gone.
#[must_use]
pub fn allowlist_is_live(known: &[String]) -> Vec<String> {
    TRANSPORT
        .iter()
        .chain(READ_ONLY.iter())
        .filter(|n| !known.iter().any(|k| k == *n))
        .map(|n| (*n).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_verbs_are_mutating() {
        assert_eq!(classify("something-brand-new"), Effects::Mutating);
        assert_eq!(classify(""), Effects::Mutating);
    }

    #[test]
    fn transport_beats_read_only() {
        assert_eq!(classify("mcp"), Effects::Transport);
        assert_eq!(classify("serve"), Effects::Transport);
        assert_eq!(classify("lsp"), Effects::Transport);
    }

    #[test]
    fn known_read_only_verbs_classify() {
        assert_eq!(classify("plan"), Effects::ReadOnly);
        assert_eq!(classify("validate"), Effects::ReadOnly);
        assert_eq!(classify("apply"), Effects::Mutating);
        assert_eq!(classify("destroy"), Effects::Mutating);
    }

    #[test]
    fn stale_allowlist_entries_are_reported() {
        let known: Vec<String> = vec!["plan".into()];
        let stale = allowlist_is_live(&known);
        assert!(stale.contains(&"validate".to_string()));
        assert!(!stale.contains(&"plan".to_string()));
    }

    #[test]
    fn a_fully_live_allowlist_reports_nothing() {
        let known: Vec<String> = TRANSPORT
            .iter()
            .chain(READ_ONLY.iter())
            .map(|s| (*s).to_string())
            .collect();
        assert!(allowlist_is_live(&known).is_empty());
    }

    #[test]
    fn allowlists_are_sorted_and_free_of_duplicates() {
        // Sorted order makes a diff to this list readable; duplicates would hide
        // a second, contradictory classification of the same verb.
        for list in [TRANSPORT, READ_ONLY] {
            let mut sorted = list.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.as_slice(), list, "list must be sorted and unique");
        }
    }

    #[test]
    fn no_verb_is_both_transport_and_read_only() {
        for t in TRANSPORT {
            assert!(!READ_ONLY.contains(t), "{t} is in both lists");
        }
    }
}
