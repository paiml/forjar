//! Policy-derived corrections to a forjar.yaml (paiml/forjar#356, F-MCP-004).
//!
//! An agent asked to "fix the permissions" on a config has to guess two things:
//! WHICH value is correct, and WHERE in the file to write it. It gets the first
//! from nowhere and the second from a text search. This module removes both
//! guesses: the value comes from the policy rule that flagged the violation
//! (see [`fixes`]), and the location comes from a byte-range anchor that
//! refuses rather than guesses (see [`crate::core::yaml_edit`]).
//!
//! **It computes; it does not write.** [`remediate`] returns the corrected
//! document as a string. No filesystem write happens on any transport, which is
//! what lets `forjar_remediate` stay `Effects::ReadOnly` — and that is not a
//! nicety. `verb serve` prints, at runtime, on any non-loopback bind: *"it has
//! NO authentication. Every forjar verb is read-only, so this exposes
//! configuration, not control."* A mutating remediate would turn that printed
//! sentence into a falsehood and an unauthenticated TCP port into a
//! config-rewrite endpoint. A caller that wants the file changed already has
//! everything it needs: the corrected bytes.
//!
//! **Scope, stated rather than silently empty.** v1 evaluates `config.policies`
//! only. Compliance packs (`core::compliance_pack`) also carry
//! structurally-fixable `Assert` checks, but `RuleEvalResult` records no
//! resource id, so a pack failure cannot be anchored to a location in the
//! document. A CIS-gated project therefore gets zero remediations from this
//! verb, and [`Report::scope_note`] says so instead of implying the config was
//! clean.

pub mod fixes;

#[cfg(test)]
mod tests_remediate;

use crate::core::config_hash::config_hash;
use crate::core::parser::{resource_field_value, violating_pairs};
use crate::core::types::{ForjarConfig, PolicyRule};
use crate::core::yaml_edit::{self, verify, AnchorError};
use std::collections::{BTreeMap, BTreeSet};

/// One correction that was applied to the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalarFix {
    /// The rule that determined the value.
    pub policy_id: String,
    /// The resource whose field was rewritten.
    pub resource_id: String,
    /// The field that was rewritten.
    pub field: String,
    /// The value before the edit, as the parser resolved it.
    pub from: Option<String>,
    /// The value written, verbatim from the rule.
    pub to: String,
    /// 1-based line of the edited value.
    pub line: usize,
}

/// One violation that is still present, and why it was not fixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unfixable {
    /// The rule that flagged it.
    pub policy_id: String,
    /// The resource it was flagged on.
    pub resource_id: String,
    /// The rule's own message.
    pub message: String,
    /// `error`, `warning` or `info`.
    pub severity: String,
    /// `assert`, `deny`, `warn`, `require` or `limit`.
    pub rule_type: String,
    /// The rule's prose `remediation:` hint, if it carries one.
    pub remediation_hint: Option<String>,
    /// Why forjar did not fix it. The most valuable field here.
    pub reason: String,
}

/// What `remediate` computed.
#[derive(Debug, Clone)]
pub struct Report {
    /// Corrections applied, sorted by `(resource, field)`.
    pub applied: Vec<ScalarFix>,
    /// The corrected document. Equal to the input when nothing was applied.
    pub updated_yaml: String,
    /// Violations still present after the corrections, RE-EVALUATED.
    pub remaining: Vec<Unfixable>,
    /// Whether the document changed.
    pub changed: bool,
    /// Content hash of the config before.
    pub hash_before: String,
    /// Content hash of the config after.
    pub hash_after: String,
    /// What this verb did NOT look at, when that is load-bearing.
    pub scope_note: Option<String>,
}

/// A fix that has been derived but not yet anchored.
struct Candidate {
    rule_index: usize,
    resource_id: String,
    field: String,
    value: String,
}

/// Keyed by `(resource_id, rule.display_id_at(index))`.
///
/// `display_id_at`, never `display_id`: the latter derives an un-id'd rule's
/// name from a slug of its `message:`, so two such rules sharing a message
/// wrote to ONE key here and the later `record()` silently replaced the
/// earlier's reason (paiml/forjar#369).
type ReasonMap = BTreeMap<(String, String), String>;

/// Compute the corrections `config`'s own policies determine, and return the
/// corrected document. Never writes.
///
/// `policy_ids` filters by [`PolicyRule::display_id_at`] — the rule's explicit
/// `id:`, or `RULE-<index>-<slug>` when it declares none. `None` or empty is
/// all. It filtered by [`PolicyRule::display_id`] until paiml/forjar#369, and
/// that string is not an identity: two un-id'd rules sharing a `message:`
/// generate the same one, so naming it applied BOTH — there was no string that
/// selected one of the two.
pub fn remediate(
    source_text: &str,
    config: &ForjarConfig,
    policy_ids: Option<&[String]>,
) -> Result<Report, String> {
    let hash_before = config_hash(config)?;
    let mut reasons: ReasonMap = BTreeMap::new();
    let candidates = derive_candidates(config, policy_ids, &mut reasons);

    let mut updated = config.clone();
    let outcome = apply_all(source_text, config, &candidates, &mut updated, &mut reasons);

    let hash_after = config_hash(&updated)?;
    Ok(Report {
        changed: !outcome.applied.is_empty(),
        remaining: remaining_violations(&updated, policy_ids, &reasons),
        applied: outcome.applied,
        updated_yaml: outcome.text,
        hash_before,
        hash_after,
        scope_note: scope_note(config),
    })
}

/// Every violation, paired with the fix its rule determines — or with the
/// reason that rule determines none.
fn derive_candidates(
    config: &ForjarConfig,
    policy_ids: Option<&[String]>,
    reasons: &mut ReasonMap,
) -> Vec<Candidate> {
    let mut out = Vec::new();
    for (rule_index, resource_id) in violating_pairs(config) {
        let rule = &config.policies[rule_index];
        let policy_id = rule.display_id_at(rule_index);
        if !selected(rule, rule_index, policy_ids) {
            record(
                reasons,
                &resource_id,
                policy_id,
                "not selected by policy_ids",
            );
            continue;
        }
        match fixes::derive(rule) {
            Ok(spec) => out.push(Candidate {
                rule_index,
                resource_id,
                field: spec.field,
                value: spec.value,
            }),
            Err(reason) => record(reasons, &resource_id, policy_id, &reason),
        }
    }
    // Determinism is a property, not an accident of iteration order.
    out.sort_by(|a, b| {
        (&a.resource_id, &a.field, &a.value).cmp(&(&b.resource_id, &b.field, &b.value))
    });
    drop_conflicts(config, out, reasons)
}

/// Two rules that demand DIFFERENT values for the same field are a
/// contradiction in the policy set, not a remediation.
///
/// Refuse both. The alternative is that sort order picks the winner, and "the
/// rule that sorts first wins" is not a property anyone can rely on — worse,
/// the loser would then be reported with the mismatch reason below, which
/// blames a recipe or a template for a disagreement between two policies.
fn drop_conflicts(
    config: &ForjarConfig,
    candidates: Vec<Candidate>,
    reasons: &mut ReasonMap,
) -> Vec<Candidate> {
    let policy_id = |c: &Candidate| config.policies[c.rule_index].display_id_at(c.rule_index);
    let mut demanded: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for c in &candidates {
        demanded
            .entry((c.resource_id.clone(), c.field.clone()))
            .or_default()
            .insert(c.value.clone());
    }
    let key = |c: &Candidate| (c.resource_id.clone(), c.field.clone());
    let (keep, conflicting): (Vec<_>, Vec<_>) = candidates
        .into_iter()
        .partition(|c| demanded[&key(c)].len() == 1);
    for c in conflicting {
        let values: Vec<String> = demanded[&key(&c)]
            .iter()
            .map(|v| format!("`{v}`"))
            .collect();
        let reason = format!(
            "{} policy rules demand different values for `{}.{}` ({}) — forjar will not \
             choose between them",
            values.len(),
            c.resource_id,
            c.field,
            values.join(" and ")
        );
        record(reasons, &c.resource_id, policy_id(&c), &reason);
    }
    keep
}

fn selected(rule: &PolicyRule, index: usize, policy_ids: Option<&[String]>) -> bool {
    match policy_ids {
        None => true,
        Some([]) => true,
        Some(ids) => ids.contains(&rule.display_id_at(index)),
    }
}

fn record(reasons: &mut ReasonMap, resource_id: &str, policy_id: String, reason: &str) {
    reasons.insert((resource_id.to_string(), policy_id), reason.to_string());
}

/// The document and the fixes that survived anchoring AND verification.
struct Applied {
    text: String,
    applied: Vec<ScalarFix>,
}

/// Anchor and splice every candidate, then prove the result changed exactly the
/// intended paths.
fn apply_all(
    source_text: &str,
    config: &ForjarConfig,
    candidates: &[Candidate],
    updated: &mut ForjarConfig,
    reasons: &mut ReasonMap,
) -> Applied {
    let mut text = source_text.to_string();
    let mut applied: Vec<ScalarFix> = Vec::new();
    let mut expected: BTreeSet<Vec<String>> = BTreeSet::new();

    for c in candidates {
        match apply_one(&text, config, c) {
            Ok((next, fix)) => {
                expected.insert(vec![
                    "resources".to_string(),
                    c.resource_id.clone(),
                    c.field.clone(),
                ]);
                text = next;
                applied.push(fix);
            }
            Err(reason) => record(
                reasons,
                &c.resource_id,
                config.policies[c.rule_index].display_id_at(c.rule_index),
                &reason,
            ),
        }
    }

    match verified(source_text, &text, &expected) {
        Ok(()) => {
            commit_to_config(updated, &applied);
            Applied { text, applied }
        }
        // Fail closed: one unverifiable path discards the whole splice.
        Err(reason) => {
            for fix in &applied {
                reasons.insert(
                    (fix.resource_id.clone(), fix.policy_id.clone()),
                    reason.clone(),
                );
            }
            Applied {
                text: source_text.to_string(),
                applied: Vec::new(),
            }
        }
    }
}

/// Anchor one candidate and splice it.
fn apply_one(
    text: &str,
    config: &ForjarConfig,
    c: &Candidate,
) -> Result<(String, ScalarFix), String> {
    let rule = &config.policies[c.rule_index];
    let resource = config
        .resources
        .get(&c.resource_id)
        .ok_or_else(|| "the resource is not in the resolved config".to_string())?;
    let path = ["resources", c.resource_id.as_str(), c.field.as_str()];
    let span = yaml_edit::find_scalar(text, &path).map_err(|e| anchor_reason(e, config, c))?;

    let in_text = yaml_edit::unquote(yaml_edit::scalar_text(text, &span));
    let resolved = resource_field_value(resource, &c.field);
    if resolved.as_deref() != Some(in_text.as_str()) {
        return Err(format!(
            "the document says `{in_text}` where the resolved config says `{}` — the value \
             is produced by a recipe or a {{{{template}}}} expansion, so editing the literal \
             would not change it",
            resolved.as_deref().unwrap_or("<unset>")
        ));
    }
    let emitted = emit_in_style(yaml_edit::scalar_text(text, &span), &c.value)?;
    Ok((
        yaml_edit::splice(text, &span, &emitted),
        ScalarFix {
            policy_id: rule.display_id_at(c.rule_index),
            resource_id: c.resource_id.clone(),
            field: c.field.clone(),
            from: resolved,
            to: c.value.clone(),
            line: span.line,
        },
    ))
}

/// Render the replacement in the quote style the document already used.
///
/// `emit_scalar` renders through serde, which reaches for single quotes for
/// anything that needs quoting. A document that wrote `mode: "0777"` should get
/// back `mode: "0644"`, not `mode: '0644'`: a diff line whose only difference is
/// the value is one an operator reads at a glance, and the quote-style churn
/// paiml/forjar#359 removed from `lint --fix` has no more business appearing
/// here than it had there.
///
/// It only ever ADDS the document's own double quotes, and only when the
/// candidate parses back to exactly the value asked for. A value carrying a
/// quote, a backslash or anything else serde would have had to escape falls
/// back to `emit_scalar` rather than being hand-quoted — hand-escaping YAML is
/// how a config file gets corrupted.
fn emit_in_style(existing: &str, value: &str) -> Result<String, String> {
    let emitted = yaml_edit::emit_scalar(value).map_err(|e| e.reason().to_string())?;
    if !existing.starts_with('"') || emitted.starts_with('"') {
        return Ok(emitted);
    }
    let candidate = format!("\"{value}\"");
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&candidate) {
        Ok(parsed) if parsed.as_str() == Some(value) => Ok(candidate),
        _ => Ok(emitted),
    }
}

/// A `NotFound` on a resource that came from an include is not a mystery —
/// name the file it came from.
fn anchor_reason(e: AnchorError, config: &ForjarConfig, c: &Candidate) -> String {
    let key = format!("resource:{}", c.resource_id);
    match (e, config.include_provenance.get(&key)) {
        (AnchorError::NotFound, Some(file)) => format!(
            "`{}` is defined in the included file `{file}`, not in this document",
            c.resource_id
        ),
        _ => e.reason().to_string(),
    }
}

/// The edit changed exactly the paths it meant to, and nothing else.
fn verified(before: &str, after: &str, expected: &BTreeSet<Vec<String>>) -> Result<(), String> {
    let changed = verify::changed_paths_of_text(before, after)?;
    if &changed == expected {
        return Ok(());
    }
    Err(format!(
        "the edit could not be verified: it changed {} instead of {} — every fix in this \
         batch was discarded",
        render(&changed),
        render(expected)
    ))
}

fn render(paths: &BTreeSet<Vec<String>>) -> String {
    if paths.is_empty() {
        return "nothing".to_string();
    }
    paths
        .iter()
        .map(|p| p.join("."))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Mirror the applied text edits into the in-memory config, so the
/// re-evaluation below sees the same document the caller will.
fn commit_to_config(config: &mut ForjarConfig, applied: &[ScalarFix]) {
    for fix in applied {
        if let Some(resource) = config.resources.get_mut(&fix.resource_id) {
            fixes::set_field(resource, &fix.field, &fix.to);
        }
    }
}

/// What is STILL violated, from a fresh evaluation of the corrected config.
///
/// Not bookkeeping: nothing is removed from a list here. If a fix did not work,
/// the violation simply reappears, which is the property the feature promises
/// ("satisfies the policy gate on a subsequent check") measured rather than
/// asserted.
fn remaining_violations(
    updated: &ForjarConfig,
    policy_ids: Option<&[String]>,
    reasons: &ReasonMap,
) -> Vec<Unfixable> {
    violating_pairs(updated)
        .into_iter()
        .map(|(rule_index, resource_id)| {
            let rule = &updated.policies[rule_index];
            Unfixable {
                reason: reason_for(reasons, &resource_id, rule_index, rule, policy_ids),
                policy_id: rule.display_id_at(rule_index),
                resource_id,
                message: rule.message.clone(),
                severity: format!("{:?}", rule.effective_severity()).to_lowercase(),
                rule_type: format!("{:?}", rule.rule_type).to_lowercase(),
                remediation_hint: rule.remediation.clone(),
            }
        })
        .collect()
}

fn reason_for(
    reasons: &ReasonMap,
    resource_id: &str,
    index: usize,
    rule: &PolicyRule,
    policy_ids: Option<&[String]>,
) -> String {
    if let Some(reason) = reasons.get(&(resource_id.to_string(), rule.display_id_at(index))) {
        return reason.clone();
    }
    if !selected(rule, index, policy_ids) {
        return "not selected by policy_ids".to_string();
    }
    match fixes::derive(rule) {
        Ok(_) => "the correction was written but the rule is still violated".to_string(),
        Err(reason) => reason,
    }
}

/// Say what was not looked at, when a project's real rules live somewhere this
/// verb does not read.
fn scope_note(config: &ForjarConfig) -> Option<String> {
    if !config.policies.is_empty() {
        return None;
    }
    Some(
        "this config declares no `policies:` block. Remediation reads inline policy rules \
         only — compliance packs record no resource id per failed check, so a pack failure \
         cannot be anchored to a location in the document"
            .to_string(),
    )
}
