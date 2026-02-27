//! MCP handler implementations for forjar tools.

use pforge_runtime::Handler;
use std::path::PathBuf;

use crate::core::{codegen, parser, planner, resolver, state, types};
use crate::tripwire::{anomaly, drift, tracer};

use super::types::*;

// ── Handler structs ─────────────────────────────────────────────────

pub struct ValidateHandler;
pub struct PlanHandler;
pub struct DriftHandler;
pub struct LintHandler;
pub struct GraphHandler;
pub struct ShowHandler;
pub struct StatusHandler;
pub struct TraceHandler;
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
        let state_dir = PathBuf::from(input.state_dir.as_deref().unwrap_or("state"));

        let config = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

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

        let mut changes: Vec<PlannedChangeOutput> = exec_plan
            .changes
            .iter()
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
        let state_dir = PathBuf::from(input.state_dir.as_deref().unwrap_or("state"));

        let config = parser::parse_and_validate(&path).map_err(pforge_runtime::Error::Handler)?;

        let mut findings = Vec::new();

        for machine_name in config.machines.keys() {
            if let Some(ref m) = input.machine {
                if machine_name != m {
                    continue;
                }
            }

            if let Ok(Some(lock_data)) = state::load_lock(&state_dir, machine_name) {
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
        Ok(DriftOutput { drifted, findings })
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
            for m in resource.machine.to_vec() {
                used_machines.insert(m);
            }
        }
        for name in config.machines.keys() {
            if !used_machines.contains(name) {
                warnings.push(format!(
                    "Machine '{}' is defined but not referenced by any resource",
                    name
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
                if let Ok(script) = result {
                    let lint_result = crate::core::purifier::lint_script(&script);
                    for d in &lint_result.diagnostics {
                        use bashrs::linter::Severity;
                        match d.severity {
                            Severity::Error => {
                                error_count += 1;
                                warnings.push(format!(
                                    "[ERROR] {}.{}: [{}] {}",
                                    id, kind, d.code, d.message
                                ));
                            }
                            _ => {
                                warnings.push(format!(
                                    "[WARN] {}.{}: [{}] {}",
                                    id, kind, d.code, d.message
                                ));
                            }
                        }
                    }
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
                    graph.push_str(&format!("  \"{}\" [label=\"{}\"];\n", id, label));
                    for dep in &resource.depends_on {
                        graph.push_str(&format!("  \"{}\" -> \"{}\";\n", dep, id));
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
                        graph.push_str(&format!("  {} --> {}\n", dep, id));
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
                    "Resource '{}' not found",
                    r
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

#[async_trait::async_trait]
impl Handler for StatusHandler {
    type Input = StatusInput;
    type Output = StatusOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let state_dir = PathBuf::from(input.state_dir.as_deref().unwrap_or("state"));

        let mut machines = Vec::new();

        if state_dir.exists() {
            let entries = std::fs::read_dir(&state_dir)
                .map_err(|e| pforge_runtime::Error::Handler(e.to_string()))?;

            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    if let Some(ref m) = input.machine {
                        if &name != m {
                            continue;
                        }
                    }

                    let resource_count = state::load_lock(&state_dir, &name)
                        .ok()
                        .flatten()
                        .map(|l| l.resources.len())
                        .unwrap_or(0);

                    machines.push(MachineStatusOutput {
                        name,
                        resource_count,
                    });
                }
            }
        }

        Ok(StatusOutput { machines })
    }
}

#[async_trait::async_trait]
impl Handler for TraceHandler {
    type Input = TraceInput;
    type Output = TraceOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let state_dir = PathBuf::from(input.state_dir.as_deref().unwrap_or("state"));

        let mut all_spans = Vec::new();

        if state_dir.exists() {
            let entries = std::fs::read_dir(&state_dir)
                .map_err(|e| pforge_runtime::Error::Handler(e.to_string()))?;

            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(ref filter) = input.machine {
                    if &name != filter {
                        continue;
                    }
                }
                if !entry.path().is_dir() {
                    continue;
                }

                if let Ok(spans) = tracer::read_trace(&state_dir, &name) {
                    for span in spans {
                        all_spans.push((name.clone(), span));
                    }
                }
            }
        }

        all_spans.sort_by_key(|(_, span)| span.logical_clock);

        let trace_count = {
            let ids: std::collections::HashSet<&str> =
                all_spans.iter().map(|(_, s)| s.trace_id.as_str()).collect();
            ids.len()
        };

        let spans = all_spans
            .into_iter()
            .map(|(machine, span)| TraceSpanOutput {
                machine,
                trace_id: span.trace_id,
                span_id: span.span_id,
                parent_span_id: span.parent_span_id,
                name: span.name,
                start_time: span.start_time,
                duration_us: span.duration_us,
                exit_code: span.exit_code,
                resource_type: span.resource_type,
                action: span.action,
                content_hash: span.content_hash,
                logical_clock: span.logical_clock,
            })
            .collect();

        Ok(TraceOutput { trace_count, spans })
    }
}

#[async_trait::async_trait]
impl Handler for AnomalyHandler {
    type Input = AnomalyInput;
    type Output = AnomalyOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let state_dir = PathBuf::from(input.state_dir.as_deref().unwrap_or("state"));
        let min_events = input.min_events.unwrap_or(3);

        let mut metrics: std::collections::HashMap<String, (u32, u32, u32)> =
            std::collections::HashMap::new();

        if state_dir.exists() {
            let entries = std::fs::read_dir(&state_dir)
                .map_err(|e| pforge_runtime::Error::Handler(e.to_string()))?;

            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(ref filter) = input.machine {
                    if &name != filter {
                        continue;
                    }
                }
                if !entry.path().is_dir() {
                    continue;
                }

                let log_path = entry.path().join("events.jsonl");
                if !log_path.exists() {
                    continue;
                }

                let content = std::fs::read_to_string(&log_path)
                    .map_err(|e| pforge_runtime::Error::Handler(e.to_string()))?;

                for line in content.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(te) = serde_json::from_str::<types::TimestampedEvent>(line) {
                        match te.event {
                            types::ProvenanceEvent::ResourceConverged { ref resource, .. } => {
                                let key = format!("{}:{}", name, resource);
                                metrics.entry(key).or_insert((0, 0, 0)).0 += 1;
                            }
                            types::ProvenanceEvent::ResourceFailed { ref resource, .. } => {
                                let key = format!("{}:{}", name, resource);
                                metrics.entry(key).or_insert((0, 0, 0)).1 += 1;
                            }
                            types::ProvenanceEvent::DriftDetected { ref resource, .. } => {
                                let key = format!("{}:{}", name, resource);
                                metrics.entry(key).or_insert((0, 0, 0)).2 += 1;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let metrics_vec: Vec<(String, u32, u32, u32)> = metrics
            .into_iter()
            .map(|(k, (c, f, d))| (k, c, f, d))
            .collect();

        let findings = anomaly::detect_anomalies(&metrics_vec, min_events);

        let output_findings = findings
            .iter()
            .map(|f| AnomalyFindingOutput {
                resource: f.resource.clone(),
                score: f.score,
                status: format!("{:?}", f.status),
                reasons: f.reasons.clone(),
            })
            .collect::<Vec<_>>();

        Ok(AnomalyOutput {
            anomaly_count: output_findings.len(),
            findings: output_findings,
        })
    }
}
