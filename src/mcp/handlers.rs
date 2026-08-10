//! MCP handler implementations for forjar tools.

use pforge_runtime::Handler;
use std::path::PathBuf;

use crate::core::{codegen, parser, planner, resolver, state};
use crate::tripwire::drift;

use super::types::*;

// ── Handler structs ─────────────────────────────────────────────────

/// MCP handler for config validation.
pub struct ValidateHandler;
/// MCP handler for execution planning.
pub struct PlanHandler;
/// MCP handler for drift detection.
pub struct DriftHandler;
/// MCP handler for recipe linting.
pub struct LintHandler;
/// MCP handler for dependency graph generation.
pub struct GraphHandler;
/// MCP handler for resolved config display.
pub struct ShowHandler;
/// MCP handler for lock file status.
pub struct StatusHandler;
/// MCP handler for trace provenance.
pub struct TraceHandler;
/// MCP handler for anomaly detection.
pub struct AnomalyHandler;

// ── Handler trait implementations ───────────────────────────────────

#[async_trait::async_trait]
impl Handler for ValidateHandler {
    type Input = ValidateInput;
    type Output = ValidateOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);
        match parser::parse_and_validate(&path) {
            Ok(config) => Ok(ValidateOutput {
                valid: true,
                resource_count: config.resources.len(),
                machine_count: config.machines.len(),
                errors: vec![],
            }),
            Err(e) => Ok(ValidateOutput {
                valid: false,
                resource_count: 0,
                machine_count: 0,
                errors: e.lines().map(|l| l.to_string()).collect(),
            }),
        }
    }
}

#[async_trait::async_trait]
impl Handler for PlanHandler {
    type Input = PlanInput;
    type Output = PlanOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);
        let state_dir = super::paths::resolve_state_dir(&path, input.state_dir.as_deref());

        let mut config =
            parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

        // FJ-2729: mirror `cli::plan`. Phony resources are goal-only, so a bulk
        // plan must not report them — otherwise an agent reading this tool sees
        // a converged project as permanently pending.
        crate::cli::strip_unrequested_phony_for_mcp(&mut config);

        let order =
            resolver::build_execution_order(&config).map_err(pforge_runtime::Error::Handler)?;

        // Load locks for all machines
        let mut locks = std::collections::HashMap::new();
        for machine_name in config.machines.keys() {
            if let Ok(Some(lock)) = state::load_lock(&state_dir, machine_name) {
                locks.insert(machine_name.clone(), lock);
            }
        }

        let exec_plan = planner::plan(&config, &order, &locks, input.tag.as_deref());

        // FJ-2729: `exec_plan.changes` carries EVERY resource with its action,
        // including NoOp — `cli::plan` filters those out before counting
        // (plan.rs:262). The MCP handler did not, so it reported all 6
        // resources of a fully converged project as pending changes while the
        // CLI reported "0 to change". Verified on the published 1.12.0 binary.
        let mut changes: Vec<PlannedChangeOutput> = exec_plan
            .changes
            .iter()
            .filter(|c| c.action != crate::core::types::PlanAction::NoOp)
            .map(|c| PlannedChangeOutput {
                resource_id: c.resource_id.clone(),
                machine: c.machine.clone(),
                action: c.action.to_string(),
                description: c.description.clone(),
            })
            .collect();

        // Apply resource filter if specified
        if let Some(ref r) = input.resource {
            changes.retain(|c| c.resource_id == *r);
        }

        Ok(PlanOutput {
            to_create: exec_plan.to_create,
            to_update: exec_plan.to_update,
            to_destroy: exec_plan.to_destroy,
            unchanged: exec_plan.unchanged,
            changes,
        })
    }
}

#[async_trait::async_trait]
impl Handler for DriftHandler {
    type Input = DriftInput;
    type Output = DriftOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);
        let state_dir = super::paths::resolve_state_dir(&path, input.state_dir.as_deref());

        let config = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

        let mut findings = Vec::new();
        // GH-208: machines we could not compare, so a caller can tell "clean"
        // apart from "not looked at".
        let mut unchecked: Vec<String> = Vec::new();

        for machine_name in config.machines.keys() {
            if let Some(ref m) = input.machine {
                if machine_name != m {
                    continue;
                }
            }

            // GH-208: `if let Ok(Some(..))` discarded BOTH `Err` (state could
            // not be read) and `Ok(None)` (machine never applied), so "I did not
            // compare anything" was reported as `{"drifted": false}` — a clean
            // bill of health for a machine that was never inspected. The CLI
            // exits 1 with "cannot read state dir" on the same input. drift is
            // the tripwire tool; a false clean is the worst outcome it has.
            let lock_data = match state::load_lock(&state_dir, machine_name) {
                Ok(Some(l)) => l,
                Ok(None) => {
                    // Genuinely no state for this machine: nothing to compare,
                    // and that is not drift. Skip it, but say so.
                    unchecked.push(format!("{machine_name}: no state recorded (never applied)"));
                    continue;
                }
                Err(e) => {
                    return Err(pforge_runtime::Error::Handler(format!(
                        "cannot read state for machine '{machine_name}' in {}: {e}",
                        state_dir.display()
                    )));
                }
            };
            {
                let drift_findings = drift::detect_drift(&lock_data);
                for f in drift_findings {
                    findings.push(DriftFindingOutput {
                        resource: f.resource_id.clone(),
                        expected_hash: f.expected_hash.clone(),
                        actual_hash: f.actual_hash.clone(),
                        detail: f.detail.clone(),
                    });
                }
            }
        }

        let drifted = !findings.is_empty();
        Ok(DriftOutput {
            drifted,
            findings,
            unchecked,
        })
    }
}

#[async_trait::async_trait]
impl Handler for LintHandler {
    type Input = LintInput;
    type Output = LintOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);

        let config = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

        let mut warnings = Vec::new();
        let mut error_count = 0;

        // Check for unused machines
        let mut used_machines = std::collections::HashSet::new();
        for resource in config.resources.values() {
            for m in resource.machine.iter() {
                used_machines.insert(m.to_owned());
            }
        }
        for name in config.machines.keys() {
            if !used_machines.contains(name) {
                warnings.push(format!(
                    "Machine '{name}' is defined but not referenced by any resource"
                ));
            }
        }

        // bashrs script lint
        for (id, resource) in &config.resources {
            for (kind, result) in [
                ("check", codegen::check_script(resource)),
                ("apply", codegen::apply_script(resource)),
                ("state_query", codegen::state_query_script(resource)),
            ] {
                let Ok(script) = result else { continue };
                let lint_result = crate::core::purifier::lint_script(&script);
                for d in &lint_result.diagnostics {
                    use bashrs::linter::Severity;
                    let prefix = match d.severity {
                        Severity::Error => {
                            error_count += 1;
                            "ERROR"
                        }
                        _ => "WARN",
                    };
                    warnings.push(format!(
                        "[{prefix}] {id}.{kind}: [{}] {}",
                        d.code, d.message
                    ));
                }
            }
        }

        let warning_count = warnings.len();
        Ok(LintOutput {
            warnings,
            warning_count,
            error_count,
        })
    }
}

#[async_trait::async_trait]
impl Handler for GraphHandler {
    type Input = GraphInput;
    type Output = GraphOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);
        let fmt = input.format.as_deref().unwrap_or("mermaid");

        let config = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

        let mut graph = String::new();
        match fmt {
            "dot" => {
                graph.push_str("digraph forjar {\n");
                graph.push_str("  rankdir=LR;\n");
                for (id, resource) in &config.resources {
                    let label = format!("{}\\n({})", id, resource.resource_type);
                    graph.push_str(&format!("  \"{id}\" [label=\"{label}\"];\n"));
                    for dep in &resource.depends_on {
                        graph.push_str(&format!("  \"{dep}\" -> \"{id}\";\n"));
                    }
                }
                graph.push_str("}\n");
            }
            _ => {
                graph.push_str("graph LR\n");
                for (id, resource) in &config.resources {
                    graph.push_str(&format!(
                        "  {}[\"{}\\n({})\"]\n",
                        id, id, resource.resource_type
                    ));
                    for dep in &resource.depends_on {
                        graph.push_str(&format!("  {dep} --> {id}\n"));
                    }
                }
            }
        }

        Ok(GraphOutput {
            graph,
            format: fmt.to_string(),
        })
    }
}

#[async_trait::async_trait]
impl Handler for ShowHandler {
    type Input = ShowInput;
    type Output = ShowOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let path = PathBuf::from(&input.path);

        let config = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

        let config_value = if let Some(ref r) = input.resource {
            if let Some(resource) = config.resources.get(r) {
                // Resolve templates for this resource
                let resolved = resolver::resolve_resource_templates(
                    resource,
                    &config.params,
                    &config.machines,
                )
                .unwrap_or_else(|_| resource.clone());
                serde_json::to_value(&resolved)
                    .map_err(|e| pforge_runtime::Error::Handler(e.to_string()))?
            } else {
                return Err(pforge_runtime::Error::Handler(format!(
                    "Resource '{r}' not found"
                )));
            }
        } else {
            serde_json::to_value(&config)
                .map_err(|e| pforge_runtime::Error::Handler(e.to_string()))?
        };

        Ok(ShowOutput {
            config: config_value,
        })
    }
}
