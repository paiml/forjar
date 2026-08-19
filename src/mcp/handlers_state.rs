//! MCP handlers that read STATE rather than config: status, trace, anomaly.
//!
//! Split out of `handlers.rs` to keep both files under the 500-line limit.

use super::handlers::{AnomalyHandler, StatusHandler, TraceHandler};
use super::types::*;
use crate::core::state;
use crate::core::types;
use crate::tripwire::{anomaly, tracer};
use pforge_runtime::Handler;

#[async_trait::async_trait]
impl Handler for StatusHandler {
    type Input = StatusInput;
    type Output = StatusOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let state_dir =
            super::paths::resolve_state_dir_opt(input.path.as_deref(), input.state_dir.as_deref());

        let mut machines = Vec::new();

        // FJ-2729: a machine's state is a DIRECTORY —
        // `state/<machine>/state.lock.yaml` (see `state::lock_file_path`). This
        // scanned for files with a `.json` extension, which forjar has never
        // written, so the handler returned an empty machine list on every
        // project. Verified on the published 1.12.0 binary: the CLI printed
        // `Machine: local (localhost)` while MCP returned `{"machines": []}`.
        //
        // Presence of the lock is the test, so a stray directory is not
        // reported as a machine.
        if state_dir.exists() {
            let entries = std::fs::read_dir(&state_dir)
                .map_err(|e| pforge_runtime::Error::Handler(e.to_string()))?;

            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    if let Some(ref m) = input.machine {
                        if &name != m {
                            continue;
                        }
                    }

                    let Some(lock) = state::load_lock(&state_dir, &name).ok().flatten() else {
                        continue;
                    };

                    machines.push(MachineStatusOutput {
                        name,
                        resource_count: lock.resources.len(),
                    });
                }
            }
        }
        machines.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(StatusOutput { machines })
    }
}

#[async_trait::async_trait]
impl Handler for TraceHandler {
    type Input = TraceInput;
    type Output = TraceOutput;
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> pforge_runtime::Result<Self::Output> {
        let state_dir =
            super::paths::resolve_state_dir_opt(input.path.as_deref(), input.state_dir.as_deref());

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
        let state_dir =
            super::paths::resolve_state_dir_opt(input.path.as_deref(), input.state_dir.as_deref());
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
                                let key = format!("{name}:{resource}");
                                metrics.entry(key).or_insert((0, 0, 0)).0 += 1;
                            }
                            types::ProvenanceEvent::ResourceFailed { ref resource, .. } => {
                                let key = format!("{name}:{resource}");
                                metrics.entry(key).or_insert((0, 0, 0)).1 += 1;
                            }
                            types::ProvenanceEvent::DriftDetected { ref resource, .. } => {
                                let key = format!("{name}:{resource}");
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
