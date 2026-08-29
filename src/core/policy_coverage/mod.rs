//! THE policy-coverage calculation (FJ-3208, paiml/forjar#356).
//!
//! # Why this module was rewritten
//!
//! There used to be TWO policy-coverage calculations, and they answered
//! different questions under one name:
//!
//! | | this module (before #356) | `cli::policy_coverage::build_report` |
//! |---|---|---|
//! | question | which RESOURCES fall in the scope of some rule | which RULES fired, and which resources are violation-free |
//! | callers | none in production — tests only | `forjar policy-coverage` |
//! | scope match | `actual.contains(rule.resource_type)` | `parser::matches_scope` (equality) |
//! | frameworks | a set of names | a name → rule-count map |
//!
//! Run both over one fixture — two resources (`conf`: file, `pkg`: package)
//! and one `require: owner` rule scoped to `file` — and both print "1 of 2",
//! pointing at OPPOSITE resources. This module said `conf` was covered and
//! `pkg` was not; the CLI said `conf` violated and `pkg` was clean. An MCP verb
//! wired to one while its documentation named the other is not a second
//! renderer, it is a second answer.
//!
//! So there is now one calculation with both halves, and [`PolicyCoverage`] is
//! the ONE type both surfaces serialise —
//! `mcp::types::PolicyCoverageOutput` is a type alias for it, not a copy of its
//! fields. `forjar policy-coverage --json` prints
//! `serde_json::to_value(&coverage)` and so does the `policy-coverage` verb, so
//! the two documents cannot disagree without failing to compile.
//! `tests/falsification_verb_pending_discharge.rs` asserts the equality against
//! the real binary anyway, because "cannot disagree" is a claim and claims get
//! falsifiers.
//!
//! The scope matcher is now `parser::matches_scope` — the one the policy
//! ENGINE uses. The substring match this module used before would report a
//! `network` resource as covered by a rule scoped `resource_type: work`, which
//! the engine would then never evaluate against it: coverage that reports
//! enforcement which does not happen is worse than no coverage report.

use crate::core::parser::{evaluate_policies_full, matches_scope};
use crate::core::types::{ForjarConfig, PolicyCheckResult, PolicyRuleType};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// A policy-coverage report: the resource side and the rule side of one
/// evaluation, in one document.
///
/// Every derived quantity is a FIELD, not a method. A method would have to be
/// materialised by a renderer, and a renderer that materialises is a second
/// place for the answer to live — which is the defect this type exists to
/// close.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(schemars::JsonSchema))]
pub struct PolicyCoverage {
    // ── the resource side: what is in scope of any rule ──────────────
    /// Resources declared in the config.
    pub total_resources: usize,
    /// Resources matched by the scope of at least one policy rule.
    pub covered_resources: usize,
    /// `covered_resources / total_resources` as a percentage. 100.0 when the
    /// config declares no resources — an empty config is not "0% covered".
    pub coverage_percent: f64,
    /// Whether every resource is in the scope of at least one rule.
    pub fully_covered: bool,
    /// Resource ids no rule's scope matches, sorted.
    pub uncovered: Vec<String>,
    /// How many rules scope to each resource, for the resources at least one
    /// rule scopes to. Absence from this map is what `uncovered` lists.
    pub per_resource: BTreeMap<String, usize>,

    // ── the rule side: what the rules actually said ──────────────────
    /// Policy rules declared in the config.
    pub total_rules: usize,
    /// Rules that produced at least one violation.
    pub rules_triggered: usize,
    /// Ids of rules that produced no violation, in declaration order. A rule
    /// here is either satisfied everywhere or scoped to nothing — the
    /// `by_resource_scope` and `uncovered` fields are how a reader tells those
    /// apart.
    pub untriggered_rules: Vec<String>,
    /// Resources that produced no violation. NOT the same as
    /// `covered_resources`: a resource with no rule scoped to it is clean and
    /// uncovered at once, which is the case a coverage report exists to find.
    pub clean_resources: usize,
    /// Rule count by rule type (require, deny, warn, assert, limit).
    pub by_type: BTreeMap<String, usize>,
    /// Rule count by effective severity (error, warning, info).
    pub by_severity: BTreeMap<String, usize>,
    /// Rule count by `resource_type` scope; `*` for rules that scope to every
    /// resource type.
    pub by_resource_scope: BTreeMap<String, usize>,
    /// Rule count by compliance framework named in `compliance:`.
    pub compliance_frameworks: BTreeMap<String, usize>,
}

/// Compute policy coverage for a config.
///
/// Pure: it reads the parsed config and nothing else — no filesystem, no
/// network, no process. That is what lets the `policy-coverage` verb declare
/// `Effects::ReadOnly` truthfully.
pub fn compute_coverage(config: &ForjarConfig) -> PolicyCoverage {
    let result = evaluate_policies_full(config);
    let per_resource = scoped_rule_counts(config);

    let total_resources = config.resources.len();
    let covered_resources = per_resource.len();
    // SORTED, not left in `resources` declaration order: this document is one
    // an agent diffs between calls, and a reordered list reads as a change.
    let mut uncovered: Vec<String> = config
        .resources
        .keys()
        .filter(|id| !per_resource.contains_key(*id))
        .cloned()
        .collect();
    uncovered.sort();

    let (rules_triggered, untriggered_rules) = trigger_split(config, &result);

    PolicyCoverage {
        total_resources,
        covered_resources,
        coverage_percent: percent(covered_resources, total_resources),
        fully_covered: uncovered.is_empty(),
        uncovered,
        per_resource,
        total_rules: config.policies.len(),
        rules_triggered,
        untriggered_rules,
        clean_resources: clean_resource_count(&result),
        by_type: tally(config, |r| policy_type_name(&r.rule_type)),
        by_severity: tally(config, |r| {
            format!("{:?}", r.effective_severity()).to_lowercase()
        }),
        by_resource_scope: tally(config, |r| {
            r.resource_type.as_deref().unwrap_or("*").to_string()
        }),
        compliance_frameworks: framework_tally(config),
    }
}

/// `covered / total` as a percentage; 100.0 for an empty config.
fn percent(covered: usize, total: usize) -> f64 {
    if total == 0 {
        return 100.0;
    }
    (covered as f64 / total as f64) * 100.0
}

/// How many rules scope to each resource. Resources no rule scopes to are
/// ABSENT rather than present with 0 — `covered_resources` is this map's
/// length, and a zero entry would inflate it.
fn scoped_rule_counts(config: &ForjarConfig) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for rule in &config.policies {
        for (id, resource) in &config.resources {
            if matches_scope(rule, resource) {
                *counts.entry(id.clone()).or_insert(0) += 1;
            }
        }
    }
    counts
}

/// `(rules that fired, ids of rules that did not)`.
///
/// BOTH sides use `display_id()`. The version this replaced read the raw
/// `policy_id: Option<String>` off each violation and compared it with the
/// rules' `display_id()`, so a rule declared without an explicit `id:` was
/// reported as untriggered no matter how many resources it failed — in a
/// report that counted those same failures under `clean_resources` one field
/// away.
fn trigger_split(config: &ForjarConfig, result: &PolicyCheckResult) -> (usize, Vec<String>) {
    let triggered: BTreeSet<String> = result.violations.iter().map(|v| v.display_id()).collect();
    let untriggered: Vec<String> = config
        .policies
        .iter()
        .map(|r| r.display_id())
        .filter(|id| !triggered.contains(id))
        .collect();
    (triggered.len(), untriggered)
}

/// Resources that produced no violation.
fn clean_resource_count(result: &PolicyCheckResult) -> usize {
    let violating: BTreeSet<&str> = result
        .violations
        .iter()
        .map(|v| v.resource_id.as_str())
        .collect();
    result.resources_checked.saturating_sub(violating.len())
}

/// Count the rules by whatever key `key` reads off each of them.
fn tally<F>(config: &ForjarConfig, key: F) -> BTreeMap<String, usize>
where
    F: Fn(&crate::core::types::PolicyRule) -> String,
{
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    for rule in &config.policies {
        *out.entry(key(rule)).or_insert(0) += 1;
    }
    out
}

/// Count the rules that name each compliance framework. A rule naming two
/// controls of ONE framework counts once for it — the question is "how many
/// rules back this framework", not "how many controls are cited".
fn framework_tally(config: &ForjarConfig) -> BTreeMap<String, usize> {
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    for rule in &config.policies {
        let named: BTreeSet<&str> = rule
            .compliance
            .iter()
            .map(|c| c.framework.as_str())
            .collect();
        for f in named {
            *out.entry(f.to_string()).or_insert(0) += 1;
        }
    }
    out
}

fn policy_type_name(rt: &PolicyRuleType) -> String {
    match rt {
        PolicyRuleType::Require => "require".into(),
        PolicyRuleType::Deny => "deny".into(),
        PolicyRuleType::Warn => "warn".into(),
        PolicyRuleType::Assert => "assert".into(),
        PolicyRuleType::Limit => "limit".into(),
    }
}

/// The report as JSON — the ONE document both `forjar policy-coverage --json`
/// and the `policy-coverage` MCP verb return.
pub fn coverage_to_json(cov: &PolicyCoverage) -> serde_json::Value {
    serde_json::to_value(cov).unwrap_or(serde_json::Value::Null)
}

/// Format the resource side of the report as human-readable text.
///
/// The rule side is rendered by `cli::policy_coverage::print_table`, which is a
/// terminal layout rather than a summary line. Both read the same
/// [`PolicyCoverage`].
pub fn format_coverage(cov: &PolicyCoverage) -> String {
    let mut lines = vec![format!(
        "Policy Coverage: {:.1}% ({}/{})",
        cov.coverage_percent, cov.covered_resources, cov.total_resources
    )];

    if !cov.by_type.is_empty() {
        lines.push("  Policies by type:".into());
        for (t, count) in &cov.by_type {
            lines.push(format!("    {t}: {count}"));
        }
    }

    if !cov.compliance_frameworks.is_empty() {
        let fws: Vec<&str> = cov
            .compliance_frameworks
            .keys()
            .map(String::as_str)
            .collect();
        lines.push(format!("  Frameworks: {}", fws.join(", ")));
    }

    if !cov.uncovered.is_empty() {
        lines.push(format!("  Uncovered ({}):", cov.uncovered.len()));
        for id in &cov.uncovered {
            lines.push(format!("    - {id}"));
        }
    }

    lines.join("\n")
}
