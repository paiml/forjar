//! Which policy violations have a determined fix, and what that fix is.
//!
//! **The target value is READ FROM THE RULE. It is never invented.** The
//! feature request that asked for this named its example as "auto-correct file
//! permissions from 0777 to 0644". `0644` is a fact about someone's policy, not
//! a constant forjar is entitled to know — hardcoding it would relocate the
//! agent's guess into the tool, which is the exact failure the feature exists
//! to prevent.
//!
//! That has a consequence the request did not notice: **only `assert` rules are
//! auto-fixable.** An `assert` says "this field must equal X", so X is the fix.
//! Every other rule type names a constraint with no value behind it:
//!
//! | type      | says                              | fix |
//! |-----------|-----------------------------------|-----|
//! | `assert`  | field **must equal** X            | write X |
//! | `deny`    | field **must not equal** X        | none — X is what to avoid, not what to write |
//! | `warn`    | as `deny`, advisory               | none |
//! | `require` | field **must be set**             | none — no value is named |
//! | `limit`   | a list must stay within bounds    | none — no scalar to set |
//!
//! `deny mode == "0777"` is the most natural way to write the request's own
//! example, and it is NOT fixable. Saying so, with the reason, is the most
//! valuable thing this module produces.

use crate::core::types::{PolicyRule, PolicyRuleType, Resource};

/// The field to write and the value to write into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    /// Resource field name.
    pub field: String,
    /// The value, taken verbatim from the rule.
    pub value: String,
}

/// The scalar fields a remediation may rewrite, and how each is set.
///
/// A deliberately small, named table. `content`, `command`, `path`, `source`
/// and `image` are excluded because they are semantic and may be block
/// scalars; `type` is excluded because changing a resource's type is not a
/// remediation, it is a different resource.
type FieldSetter = fn(&mut Resource, &str);

const SETTABLE: &[(&str, FieldSetter)] = &[
    ("mode", |r, v| r.mode = Some(v.to_string())),
    ("owner", |r, v| r.owner = Some(v.to_string())),
    ("group", |r, v| r.group = Some(v.to_string())),
    ("state", |r, v| r.state = Some(v.to_string())),
    ("provider", |r, v| r.provider = Some(v.to_string())),
];

/// The fix a rule determines, or the reason it determines none.
pub fn derive(rule: &PolicyRule) -> Result<TargetSpec, String> {
    match rule.rule_type {
        PolicyRuleType::Assert => derive_assert(rule),
        PolicyRuleType::Deny | PolicyRuleType::Warn => Err(
            "a deny/warn rule names a value that is FORBIDDEN, not one that is required — \
             there is no target to write, and forjar will not invent one"
                .to_string(),
        ),
        PolicyRuleType::Require => Err(
            "a require rule names a field that must be set, not the value to set it to".to_string(),
        ),
        PolicyRuleType::Limit => {
            Err("a limit rule bounds the size of a list; there is no scalar to set".to_string())
        }
    }
}

fn derive_assert(rule: &PolicyRule) -> Result<TargetSpec, String> {
    let field = rule
        .condition_field
        .as_deref()
        .ok_or("an assert rule with no condition_field names no field to set")?;
    let value = rule
        .condition_value
        .as_deref()
        .ok_or("an assert rule with no condition_value names no value to write")?;
    if !is_settable(field) {
        return Err(format!(
            "`{field}` is not one of the scalar fields forjar will rewrite ({})",
            settable_fields().join(", ")
        ));
    }
    Ok(TargetSpec {
        field: field.to_string(),
        value: value.to_string(),
    })
}

/// Whether a field may be rewritten by a remediation.
pub fn is_settable(field: &str) -> bool {
    SETTABLE.iter().any(|(name, _)| *name == field)
}

/// The settable field names, for error messages.
pub fn settable_fields() -> Vec<&'static str> {
    SETTABLE.iter().map(|(name, _)| *name).collect()
}

/// Set a settable field on a resource. `false` when the field is not settable.
pub fn set_field(resource: &mut Resource, field: &str, value: &str) -> bool {
    match SETTABLE.iter().find(|(name, _)| *name == field) {
        Some((_, set)) => {
            set(resource, value);
            true
        }
        None => false,
    }
}
