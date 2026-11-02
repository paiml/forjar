//! forjar#613: `plan` says when a versioned binary on the box is not its pin.
//!
//! The planner is lock-relative: it prints `Create` or `NoOp` for a
//! `github_release` and says nothing about the version on the box. paiml/infra
//! pinned ollama to `v0.33.2` on a box running `0.34.2`, and plan's answer gave
//! no hint that the apply would downgrade it. This pass asks the box, using the
//! same read-only probe `forjar drift` uses (`tripwire::drift::version_pin`),
//! for every versioned binary the plan covers, and prints the verdicts beside
//! the plan. It changes no action, because a plan that turned version drift into
//! `Update` would schedule the downgrade it is warning about.

use super::colors::{bold, red, yellow};
use crate::core::types::{ExecutionPlan, ForjarConfig, PlanAction};
use crate::tripwire::drift::version_pin;

/// Whether `plan` asks the box about version pins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlanVersionPins {
    /// Not asked. Only for internal callers that plan without a target.
    Skip,
    /// Ask the live binary, and (unless `--offline`) the latest upstream release.
    Check {
        /// Compare each pin with the repo's latest GitHub release.
        upstream: bool,
    },
}

/// One verdict row.
pub(crate) struct PinRow {
    machine: String,
    resource_id: String,
    verdict: &'static str,
    detail: String,
}

/// Measure every versioned binary the plan covers. `None` when skipped.
pub(crate) fn measure(
    config: &ForjarConfig,
    plan: &ExecutionPlan,
    mode: PlanVersionPins,
) -> Option<Vec<PinRow>> {
    let PlanVersionPins::Check { upstream } = mode else {
        return None;
    };
    let resolved = crate::core::resolver::resolve_all(
        &config.resources,
        &config.params,
        &config.machines,
        &config.secrets,
    );
    let mut per_machine: indexmap::IndexMap<&str, indexmap::IndexMap<String, _>> =
        indexmap::IndexMap::new();
    for c in plan
        .changes
        .iter()
        .filter(|c| c.action != PlanAction::Destroy)
    {
        let Some(r) = resolved.get(&c.resource_id) else {
            continue;
        };
        if version_pin::is_versioned_binary(r) {
            per_machine
                .entry(c.machine.as_str())
                .or_default()
                .insert(c.resource_id.clone(), r.clone());
        }
    }
    let mut rows = Vec::new();
    for (machine_name, resources) in per_machine {
        let Some(machine) = config.machines.get(machine_name) else {
            rows.extend(resources.keys().map(|id| PinRow {
                machine: machine_name.to_string(),
                resource_id: id.clone(),
                verdict: "NOT CHECKED",
                detail: format!("machine {machine_name} is not declared, so nothing was asked"),
            }));
            continue;
        };
        let v = version_pin::check_version_pins(machine_name, machine, &resources, upstream);
        for f in v.findings {
            rows.push(PinRow {
                machine: machine_name.to_string(),
                verdict: if f.is_unmeasured() {
                    "UNMEASURED"
                } else {
                    "DRIFTED"
                },
                resource_id: f.resource_id,
                detail: f.detail,
            });
        }
        for (id, why) in v.not_checked {
            rows.push(PinRow {
                machine: machine_name.to_string(),
                resource_id: id,
                verdict: "NOT CHECKED",
                detail: why,
            });
        }
    }
    Some(rows)
}

/// The text section. Printed iff there is something to say: a plan with no
/// versioned binaries, or with every pin matching, adds no banner.
pub(crate) fn print_text(rows: &[PinRow]) {
    if rows.is_empty() {
        return;
    }
    println!(
        "\n{}",
        bold("Version pins (live --version vs declared pin):")
    );
    for r in rows {
        let tag = match r.verdict {
            "DRIFTED" => red(r.verdict),
            _ => yellow(r.verdict),
        };
        println!("  {tag} {} ({}): {}", r.resource_id, r.machine, r.detail);
    }
}

/// The `--json` field. Always an array when measured, so `[]` means "asked,
/// all pins match" and an absent key means "not asked".
pub(crate) fn to_json(rows: &[PinRow]) -> serde_json::Value {
    serde_json::Value::Array(
        rows.iter()
            .map(|r| {
                serde_json::json!({
                    "machine": r.machine,
                    "resource_id": r.resource_id,
                    "verdict": r.verdict,
                    "detail": r.detail,
                })
            })
            .collect(),
    )
}
