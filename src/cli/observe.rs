//! Observability.
//!
//! `anomaly` lives in [`super::observe_anomaly`]; it is re-exported here so the
//! existing `use super::observe::*` importers keep working.

use super::helpers::*;
use super::helpers_state::*;
use super::print_helpers::*;
use crate::core::{executor, planner, resolver, types};
use crate::tripwire::tracer;
use std::path::Path;

pub(crate) use super::observe_anomaly::cmd_anomaly;

/// View trace provenance data from apply runs (FJ-050).
/// Output trace spans as JSON.
pub(super) fn output_trace_json(all_spans: &[(String, tracer::TraceSpan)]) -> Result<(), String> {
    let json_spans: Vec<serde_json::Value> = all_spans
        .iter()
        .map(|(machine, span)| {
            serde_json::json!({
                "machine": machine,
                "trace_id": span.trace_id,
                "span_id": span.span_id,
                "parent_span_id": span.parent_span_id,
                "name": span.name,
                "start_time": span.start_time,
                "duration_us": span.duration_us,
                "exit_code": span.exit_code,
                "resource_type": span.resource_type,
                "action": span.action,
                "content_hash": span.content_hash,
                "logical_clock": span.logical_clock,
            })
        })
        .collect();
    let report = serde_json::json!({
        "traces": all_spans.iter().map(|(_, s)| &s.trace_id).collect::<std::collections::HashSet<_>>().len(),
        "spans": json_spans,
    });
    let output = serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
    println!("{output}");
    Ok(())
}

/// Format a duration in microseconds to a human-readable string.
pub(super) fn format_duration_us(us: u64) -> String {
    if us > 1_000_000 {
        format!("{:.1}s", us as f64 / 1_000_000.0)
    } else if us > 1_000 {
        format!("{:.1}ms", us as f64 / 1_000.0)
    } else if us > 0 {
        format!("{us}us")
    } else {
        "0".to_string()
    }
}

/// Print trace spans grouped by trace_id in text format.
pub(super) fn print_trace_text(all_spans: &[(String, tracer::TraceSpan)]) {
    let mut by_trace: std::collections::HashMap<&str, Vec<&(String, tracer::TraceSpan)>> =
        std::collections::HashMap::new();
    for item in all_spans {
        by_trace.entry(&item.1.trace_id).or_default().push(item);
    }

    for (trace_id, spans) in &by_trace {
        println!("Trace: {}  ({} spans)", trace_id, spans.len());
        for (machine, span) in spans.iter() {
            let duration = format_duration_us(span.duration_us);
            let status = if span.exit_code == 0 { "ok" } else { "FAIL" };
            println!(
                "  [{:>3}] {} {} — {} {} ({})",
                span.logical_clock, machine, span.name, span.action, status, duration
            );
        }
        println!();
    }
}

pub(crate) fn cmd_trace(
    state_dir: &Path,
    machine_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;

    let mut all_spans = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine_filter {
            if name != filter {
                continue;
            }
        }
        if !entry.path().is_dir() {
            continue;
        }
        if let Ok(spans) = tracer::read_trace(state_dir, &name) {
            for span in spans {
                all_spans.push((name.clone(), span));
            }
        }
    }

    if all_spans.is_empty() {
        if json {
            println!("{{\"traces\":0,\"spans\":[]}}");
        } else {
            println!("No trace data found.");
        }
        return Ok(());
    }

    all_spans.sort_by_key(|(_, span)| span.logical_clock);

    if json {
        output_trace_json(&all_spans)?;
    } else {
        print_trace_text(&all_spans);
    }

    Ok(())
}

/// FJ-264: Export JSON Schema for forjar.yaml.
/// FJ-267: Watch config file for changes and auto-plan (or auto-apply).
pub(crate) fn cmd_watch(
    file: &Path,
    state_dir: &Path,
    interval_secs: u64,
    auto_apply: bool,
    yes: bool,
) -> Result<(), String> {
    if auto_apply && !yes {
        return Err("--apply requires --yes to confirm automatic apply".to_string());
    }

    let interval = std::time::Duration::from_secs(interval_secs.max(1));
    let mut last_content: Vec<u8> = Vec::new();

    println!(
        "Watching {} (poll every {}s, {}). Ctrl-C to stop.",
        file.display(),
        interval_secs,
        if auto_apply {
            "auto-apply"
        } else {
            "plan-only"
        }
    );

    loop {
        let content = match std::fs::read(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("watch: cannot read {}: {}", file.display(), e);
                std::thread::sleep(interval);
                continue;
            }
        };

        if content != last_content {
            if !last_content.is_empty() {
                println!("\n{} changed — re-planning...", file.display());
            }
            last_content = content;
            handle_watch_change(file, state_dir, auto_apply);
        }

        std::thread::sleep(interval);
    }
}

/// Handle a detected config change during watch: re-plan and optionally auto-apply.
pub(crate) fn handle_watch_change(file: &Path, state_dir: &Path, auto_apply: bool) {
    let config = match parse_and_validate(file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("watch: parse error: {e}");
            return;
        }
    };
    let execution_order = match resolver::build_execution_order(&config) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("watch: resolve error: {e}");
            return;
        }
    };
    let locks = load_all_locks(state_dir, &config);
    let plan = planner::plan(&config, &execution_order, &locks, None);
    print_plan(
        &plan,
        None,
        Some(&config),
        super::print_helpers::unconsulted_observations(&locks),
    );

    if auto_apply && (plan.to_create > 0 || plan.to_update > 0 || plan.to_destroy > 0) {
        run_watch_apply(&config, state_dir);
    }
}

/// Execute an auto-apply during watch mode.
fn run_watch_apply(config: &types::ForjarConfig, state_dir: &Path) {
    println!("\nAuto-applying...");
    let cfg = executor::ApplyConfig {
        config,
        state_dir,
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: Some(crate::core::types::generate_run_id()),
        refresh: false,
        force_tag: None,
    };
    match executor::apply(&cfg) {
        Ok(results) => {
            let total_converged: u32 = results.iter().map(|r| r.resources_converged).sum();
            let total_unchanged: u32 = results.iter().map(|r| r.resources_unchanged).sum();
            println!(
                "{}",
                green(&format!(
                    "Apply complete: {total_converged} converged, {total_unchanged} unchanged."
                ))
            );
        }
        Err(e) => eprintln!("{}", red(&format!("Apply error: {e}"))),
    }
}
