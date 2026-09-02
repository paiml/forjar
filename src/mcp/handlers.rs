//! MCP handler implementations for forjar tools.

use pforge_runtime::Handler;
use std::path::PathBuf;

use crate::core::{parser, planner, quality_gate, resolver, state};

use super::types::*;

// ── Handler structs ─────────────────────────────────────────────────

/// MCP handler for config validation.
pub struct ValidateHandler;
/// MCP handler for execution planning.
pub struct PlanHandler;
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
/// MCP handler for policy-derived config corrections.
pub struct RemediateHandler;

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

        let parsed = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

        // forjar#372: this verb is published with `readOnlyHint: true`, which
        // `src/verb/spec.rs` defines as "safe for an agent to call unattended".
        // Three ordinary config keys reach a subprocess from inside
        // `planner::plan` — `ambient_inputs`, a `sops`/`op` secrets provider,
        // and an `output_equivalence` normaliser — so an agent asked to inspect
        // an untrusted repository executed whatever that repository declared,
        // with no flag involved. Strip them BEFORE anything reads the config,
        // and disclose the result below. The CLI is unchanged: `forjar plan`
        // still probes, because the operator chose that config themselves.
        let (mut config, unattended_skipped) = crate::core::unattended::sanitize_config(&parsed);

        // FJ-2729: mirror `cli::plan`. Phony resources are goal-only, so a bulk
        // plan must not report them — otherwise an agent reading this tool sees
        // a converged project as permanently pending. GH-214: an explicitly
        // selected resource counts as a goal, so `resource: <phony>` survives.
        let goals: Vec<String> = input.resource.iter().cloned().collect();
        crate::cli::strip_unrequested_phony_for_mcp(&mut config, &goals);

        // GH-214 (#208), contracts/selector-scope-v1.yaml INV-SELECT-ONCE:
        // select ONCE, before the plan is summarised. The old code applied the
        // `resource` selector as a post-hoc `changes.retain(..)` AFTER the
        // planner had counted the UNFILTERED set, so a filtered plan returned
        // one change alongside `to_create: 2`, and an id that exists nowhere in
        // the config returned `changes: []` with `to_create: 2` and no error —
        // while the sibling `forjar_show` errors on exactly that input and the
        // sibling `tag` selector (applied inside the planner) got its counts
        // right. Reject an unknown id, then narrow the execution order so every
        // projection — body AND counters — is derived from the selected set.
        let mut order =
            resolver::build_execution_order(&config).map_err(pforge_runtime::Error::Handler)?;
        if let Some(ref r) = input.resource {
            if !config.resources.contains_key(r) {
                return Err(pforge_runtime::Error::Handler(format!(
                    "Resource '{r}' not found"
                )));
            }
            order.retain(|id| id == r);
        }

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
        let changes: Vec<PlannedChangeOutput> = exec_plan
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

        // forjar#342: the MCP/HTTP/verb transports all serialise this one
        // `PlanOutput` (verb/registry.rs), so the disclosure reaches three
        // surfaces here. `locks` above is the same map the CLI counts over, and
        // neither surface narrows it by `-r`, so the two agree by construction.
        let unconsulted = crate::cli::unconsulted_observations_for_mcp(&locks);
        // forjar#372: the two blind spots compose into the one string a
        // consumer reads — what this plan did not CONSULT, and what it did not
        // EXECUTE.
        let disclosure = crate::core::unattended::merge_disclosures(
            crate::cli::scope_disclosure_for_mcp(unconsulted),
            crate::core::unattended::disclosure(&unattended_skipped),
        );
        Ok(PlanOutput {
            to_create: exec_plan.to_create,
            to_update: exec_plan.to_update,
            to_destroy: exec_plan.to_destroy,
            unchanged: exec_plan.unchanged,
            changes,
            lock_relative: true,
            unconsulted_observations: unconsulted,
            unattended_skipped,
            disclosure,
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

        // FJQ: the gate lives in core, NOT here. This handler used to run its
        // own bashrs loop with no heredoc filter and no SC1 exclusion, while
        // `cli/lint.rs` ran a second loop that applied both — the same verb
        // giving two answers depending on which transport asked. One call,
        // one verdict, rendered identically by both.
        //
        // `policy_dir` is `None` here and is not a field of `LintInput`: a pack
        // rule of `type: script` runs `sh -c`, and this verb publishes
        // `readOnlyHint: true`. See `LintInput` for the measurement.
        let thresholds = quality_gate::GateThresholds {
            max_cyclomatic: input.max_cyclomatic,
            policy_dir: None,
            complexity_is_error: false,
        };
        let yaml_text = std::fs::read_to_string(&path).ok();
        let report = quality_gate::evaluate(&config, yaml_text.as_deref(), &thresholds);

        warnings.extend(report.render());
        let error_count = report.error_count();
        let gate_passed = report.passed();
        Ok(LintOutput {
            warning_count: warnings.len(),
            warnings,
            error_count,
            gate_passed,
            error_code: (!gate_passed).then(|| quality_gate::QUALITY_GATE_ERROR_CODE.to_string()),
            sarif: report.to_sarif(&input.path),
            findings: report.findings.iter().map(finding_output).collect(),
        })
    }
}

/// Project a gate finding onto its wire shape.
fn finding_output(f: &quality_gate::GateFinding) -> GateFindingOutput {
    GateFindingOutput {
        rule_id: f.rule_id.clone(),
        level: f.level.sarif_level().to_string(),
        resource: f.resource_id.clone(),
        message: f.message.clone(),
        yaml_line: f.yaml_line,
        script_kind: f.script_kind.map(str::to_string),
        script_line: f.script_line,
    }
}

#[async_trait::async_trait]
impl Handler for GraphHandler {
    type Input = GraphInput;
    type Output = GraphOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        use crate::cli::graph_core::GraphFormat;

        let path = PathBuf::from(&input.path);
        let requested = input.format.as_deref().unwrap_or("mermaid");

        // GH-212 (#208): the `format` field of the response is part of this
        // tool's output contract, so it must name the renderer that ACTUALLY
        // ran. It used to be the caller's raw string echoed over a Mermaid
        // payload (`{"graph": "graph LR …", "format": "svg"}`), and any
        // unrecognised value — including "BOGUS" — silently fell through to
        // Mermaid with `isError: false`, where `forjar graph --format BOGUS`
        // exits 1. Parse through the CLI's own parser (one message, one
        // supported set) and REFUSE what this surface cannot render rather
        // than substituting a different format under the requested label.
        let fmt = match crate::cli::graph_core::parse_graph_format(requested)
            .map_err(pforge_runtime::Error::Handler)?
        {
            GraphFormat::Mermaid => "mermaid",
            GraphFormat::Dot => "dot",
            // Honest refusal: these two are implemented as CLI printers only.
            // Returning Mermaid under their name would be worse than an error.
            other @ (GraphFormat::Ascii | GraphFormat::Svg) => {
                let name = match other {
                    GraphFormat::Ascii => "ascii",
                    _ => "svg",
                };
                return Err(pforge_runtime::Error::Handler(format!(
                    "graph format '{name}' is not implemented for the forjar_graph MCP tool \
                     (CLI only): use mermaid or dot"
                )));
            }
        };

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

        let mut config =
            parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

        // GH-212 (#208): resolve ONCE, before the fan-out. The whole-config
        // branch used to serialise the freshly parsed Config, so it answered
        // with `{{params.sandbox}}/hello.txt` — a literal path that exists
        // nowhere on disk — while the `resource` branch of the SAME tool
        // resolved templates and `forjar show --json` resolved them too. The
        // tool advertises itself as "Show fully resolved config with templates
        // expanded"; both branches must honour that. Mirrors `cli::show`.
        for (id, resource) in config.resources.iter_mut() {
            *resource =
                resolver::resolve_resource_templates(resource, &config.params, &config.machines)
                    .map_err(|e| {
                        pforge_runtime::Error::Handler(format!(
                            "cannot resolve templates for resource '{id}': {e}"
                        ))
                    })?;
        }

        let config_value = if let Some(ref r) = input.resource {
            if let Some(resource) = config.resources.get(r) {
                serde_json::to_value(resource)
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
