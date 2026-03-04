//! FJ-1421: Runtime invariant monitors.
//!
//! `forjar invariants` generates and evaluates runtime invariant monitors
//! from declared policies and resource state. Invariants like "port 22 never
//! open on prod" are expressed as policy rules and verified against state.

use super::helpers::*;
use std::path::Path;

/// An invariant to monitor at runtime.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Invariant {
    pub id: String,
    pub category: String,
    pub expression: String,
    pub scope: String,
    pub status: InvariantStatus,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum InvariantStatus {
    Satisfied,
    Violated,
    Unknown,
}

impl std::fmt::Display for InvariantStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvariantStatus::Satisfied => write!(f, "SATISFIED"),
            InvariantStatus::Violated => write!(f, "VIOLATED"),
            InvariantStatus::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Invariant evaluation report.
#[derive(Debug, serde::Serialize)]
pub struct InvariantReport {
    pub invariants: Vec<Invariant>,
    pub total: usize,
    pub satisfied: usize,
    pub violated: usize,
    pub unknown: usize,
}

/// Generate and evaluate runtime invariants from config.
pub fn cmd_invariants(file: &Path, state_dir: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut invariants = Vec::new();

    collect_policy_invariants(&config, &mut invariants);
    collect_resource_invariants(&config, state_dir, &mut invariants);

    let report = build_report(invariants);

    if json {
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{output}");
    } else {
        print_invariant_report(&report);
    }

    if report.violated > 0 {
        Err(format!("{} invariant(s) violated", report.violated))
    } else {
        Ok(())
    }
}

fn build_report(invariants: Vec<Invariant>) -> InvariantReport {
    let total = invariants.len();
    let satisfied = invariants
        .iter()
        .filter(|i| matches!(i.status, InvariantStatus::Satisfied))
        .count();
    let violated = invariants
        .iter()
        .filter(|i| matches!(i.status, InvariantStatus::Violated))
        .count();
    InvariantReport {
        invariants,
        total,
        satisfied,
        violated,
        unknown: total - satisfied - violated,
    }
}

fn collect_policy_invariants(
    config: &crate::core::types::ForjarConfig,
    invariants: &mut Vec<Invariant>,
) {
    for policy in &config.policies {
        if let Some(ref field) = policy.field {
            invariants.push(Invariant {
                id: format!("policy-require-{field}"),
                category: "policy".to_string(),
                expression: format!("all resources have field '{field}'"),
                scope: "global".to_string(),
                status: check_field_invariant(config, field),
            });
        }
        if let (Some(ref cf), Some(ref cv)) = (&policy.condition_field, &policy.condition_value) {
            invariants.push(Invariant {
                id: format!("policy-deny-{cf}"),
                category: "security".to_string(),
                expression: format!("no resource has {cf} == '{cv}'"),
                scope: "global".to_string(),
                status: check_deny_condition(config, cf, cv),
            });
        }
    }
}

fn collect_resource_invariants(
    config: &crate::core::types::ForjarConfig,
    state_dir: &Path,
    invariants: &mut Vec<Invariant>,
) {
    for (id, res) in &config.resources {
        collect_service_invariant(id, res, invariants);
        collect_path_invariant(id, res, invariants);
        collect_state_invariants(id, res, state_dir, invariants);
    }
}

fn collect_service_invariant(
    id: &str,
    res: &crate::core::types::Resource,
    invariants: &mut Vec<Invariant>,
) {
    if res.resource_type != crate::core::types::ResourceType::Service {
        return;
    }
    invariants.push(Invariant {
        id: format!("{id}-has-name"),
        category: "completeness".to_string(),
        expression: format!("service '{id}' has a defined name"),
        scope: id.to_string(),
        status: if res.name.is_some() {
            InvariantStatus::Satisfied
        } else {
            InvariantStatus::Violated
        },
    });
}

fn collect_path_invariant(
    id: &str,
    res: &crate::core::types::Resource,
    invariants: &mut Vec<Invariant>,
) {
    let Some(ref path) = res.path else { return };
    invariants.push(Invariant {
        id: format!("{id}-absolute-path"),
        category: "safety".to_string(),
        expression: format!("resource '{id}' uses absolute path"),
        scope: id.to_string(),
        status: if path.starts_with('/') {
            InvariantStatus::Satisfied
        } else {
            InvariantStatus::Violated
        },
    });
}

fn collect_state_invariants(
    id: &str,
    res: &crate::core::types::Resource,
    state_dir: &Path,
    invariants: &mut Vec<Invariant>,
) {
    for machine in res.machine.to_vec() {
        let lock_path = state_dir.join(&machine).join("state.lock.yaml");
        invariants.push(Invariant {
            id: format!("{id}-{machine}-state-exists"),
            category: "state".to_string(),
            expression: format!("state lock exists for '{id}' on '{machine}'"),
            scope: format!("{id}@{machine}"),
            status: if lock_path.exists() {
                InvariantStatus::Satisfied
            } else {
                InvariantStatus::Unknown
            },
        });
    }
}

fn check_field_invariant(
    config: &crate::core::types::ForjarConfig,
    field: &str,
) -> InvariantStatus {
    let all_have = config.resources.values().all(|r| match field {
        "tags" => !r.tags.is_empty(),
        "depends_on" => !r.depends_on.is_empty(),
        "name" => r.name.is_some(),
        "path" => r.path.is_some(),
        _ => true,
    });
    if all_have {
        InvariantStatus::Satisfied
    } else {
        InvariantStatus::Violated
    }
}

fn check_deny_condition(
    config: &crate::core::types::ForjarConfig,
    field: &str,
    value: &str,
) -> InvariantStatus {
    let any_match = config.resources.values().any(|r| {
        let field_val = match field {
            "content" => r.content.as_deref(),
            "command" => r.command.as_deref(),
            "path" => r.path.as_deref(),
            _ => None,
        };
        field_val == Some(value)
    });
    if any_match {
        InvariantStatus::Violated
    } else {
        InvariantStatus::Satisfied
    }
}

fn print_invariant_report(report: &InvariantReport) {
    println!("Runtime Invariant Report");
    println!("========================");
    println!(
        "Total: {} | Satisfied: {} | Violated: {} | Unknown: {}",
        report.total, report.satisfied, report.violated, report.unknown
    );
    println!();
    for inv in &report.invariants {
        let icon = match inv.status {
            InvariantStatus::Satisfied => "OK ",
            InvariantStatus::Violated => "ERR",
            InvariantStatus::Unknown => "???",
        };
        println!("[{icon}] {}: {} ({})", inv.id, inv.expression, inv.category);
    }
}
