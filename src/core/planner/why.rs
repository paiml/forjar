//! FJ-1379: `--why` change explanation — per-resource reason for planned action.

use super::{default_state, hash_desired_state};
use crate::core::types::*;
use crate::tripwire::hasher;

/// A structured explanation of why a resource is changing.
#[derive(Debug, Clone)]
pub struct ChangeReason {
    pub resource_id: String,
    pub machine: String,
    pub action: PlanAction,
    pub reasons: Vec<String>,
}

/// Explain why a resource on a machine has the given planned action.
pub fn explain_why(
    resource_id: &str,
    resource: &Resource,
    machine_name: &str,
    locks: &std::collections::HashMap<String, StateLock>,
) -> ChangeReason {
    let state = resource
        .state
        .as_deref()
        .unwrap_or_else(|| default_state(&resource.resource_type));

    if state == "absent" {
        return explain_absent(resource_id, machine_name, locks);
    }

    explain_present(resource_id, resource, machine_name, locks)
}

fn explain_absent(
    resource_id: &str,
    machine_name: &str,
    locks: &std::collections::HashMap<String, StateLock>,
) -> ChangeReason {
    let base = ChangeReason {
        resource_id: resource_id.to_string(),
        machine: machine_name.to_string(),
        action: PlanAction::NoOp,
        reasons: vec![],
    };

    if let Some(lock) = locks.get(machine_name) {
        if lock.resources.contains_key(resource_id) {
            return ChangeReason {
                action: PlanAction::Destroy,
                reasons: vec!["state: absent — resource exists in lock, will be removed".into()],
                ..base
            };
        }
    }

    ChangeReason {
        reasons: vec!["state: absent — resource not in lock, nothing to destroy".into()],
        ..base
    }
}

fn explain_present(
    resource_id: &str,
    resource: &Resource,
    machine_name: &str,
    locks: &std::collections::HashMap<String, StateLock>,
) -> ChangeReason {
    let mut base = ChangeReason {
        resource_id: resource_id.to_string(),
        machine: machine_name.to_string(),
        action: PlanAction::Create,
        reasons: vec![],
    };

    let lock = match locks.get(machine_name) {
        Some(l) => l,
        None => {
            base.reasons
                .push("no lock file for machine — first apply".into());
            return base;
        }
    };
    let rl = match lock.resources.get(resource_id) {
        Some(r) => r,
        None => {
            base.reasons
                .push("resource not in lock — new resource".into());
            return base;
        }
    };

    // Resource exists in lock — check status
    if rl.status == ResourceStatus::Failed {
        return ChangeReason {
            action: PlanAction::Update,
            reasons: vec!["previous apply failed — will retry".into()],
            ..base
        };
    }
    if rl.status == ResourceStatus::Drifted {
        return ChangeReason {
            action: PlanAction::Update,
            reasons: vec!["resource drifted from desired state".into()],
            ..base
        };
    }

    // Check hash
    let desired_hash = hash_desired_state(resource);
    if rl.hash == desired_hash {
        return ChangeReason {
            action: PlanAction::NoOp,
            reasons: vec![format!("hash unchanged ({})", truncate_hash(&desired_hash))],
            ..base
        };
    }

    // Hash changed — explain which fields differ
    let mut reasons = vec![format!(
        "hash changed: {} -> {}",
        truncate_hash(&rl.hash),
        truncate_hash(&desired_hash)
    )];

    // Try to identify changed fields
    let field_diffs = diff_resource_fields(resource, rl);
    reasons.extend(field_diffs);

    ChangeReason {
        action: PlanAction::Update,
        reasons,
        ..base
    }
}

/// Truncate a hash for display (first 16 chars).
fn truncate_hash(hash: &str) -> String {
    if hash.len() > 24 {
        format!("{}...", &hash[..24])
    } else {
        hash.to_string()
    }
}

/// Compare resource fields against stored lock details to find what changed.
fn diff_resource_fields(resource: &Resource, rl: &ResourceLock) -> Vec<String> {
    let mut diffs = Vec::new();

    // Check content change
    if let Some(ref content) = resource.content {
        let desired_content_hash = hasher::hash_string(content);
        if let Some(serde_yaml_ng::Value::String(stored)) = rl.details.get("content_hash") {
            if *stored != desired_content_hash {
                diffs.push("content changed".into());
            }
        }
    }

    // Check path change
    if let Some(ref path) = resource.path {
        if let Some(serde_yaml_ng::Value::String(stored)) = rl.details.get("path") {
            if stored != path {
                diffs.push(format!("path changed: {stored} -> {path}"));
            }
        }
    }

    // Check version change
    if let Some(ref version) = resource.version {
        if let Some(serde_yaml_ng::Value::String(stored)) = rl.details.get("version") {
            if stored != version {
                diffs.push(format!("version changed: {stored} -> {version}"));
            }
        }
    }

    // Check packages change
    if !resource.packages.is_empty() {
        if let Some(serde_yaml_ng::Value::String(stored)) = rl.details.get("packages") {
            let current = resource.packages.join(",");
            if *stored != current {
                diffs.push(format!("packages changed: {stored} -> {current}"));
            }
        }
    }

    if diffs.is_empty() {
        diffs.push("configuration fields changed (hash mismatch)".into());
    }

    diffs
}

/// Format a ChangeReason for human display.
pub fn format_why(reason: &ChangeReason) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "{} on {} -> {:?}",
        reason.resource_id, reason.machine, reason.action
    ));
    for r in &reason.reasons {
        lines.push(format!("  - {r}"));
    }
    lines.join("\n")
}
