//! CLI handler for `forjar score` — recipe quality grading.

use crate::core::scoring;
use crate::core::types::{ProvenanceEvent, TimestampedEvent};
use std::path::Path;

/// Execute the `forjar score` command.
pub(crate) fn cmd_score(
    file: &Path,
    status: &str,
    idempotency: &str,
    budget_ms: u64,
    json: bool,
    state_dir: &Path,
) -> Result<(), String> {
    // FJ-3020: Build runtime data from events.jsonl
    let runtime = build_runtime_data(state_dir);

    let input = scoring::ScoringInput {
        status: status.to_string(),
        idempotency: idempotency.to_string(),
        budget_ms,
        runtime,
        raw_yaml: None, // compute_from_file reads the file
    };

    let result = scoring::compute_from_file(file, &input)?;

    if json {
        let dims: Vec<String> = result
            .dimensions
            .iter()
            .map(|d| {
                format!(
                    "{{\"code\":\"{}\",\"name\":\"{}\",\"score\":{},\"weight\":{}}}",
                    d.code, d.name, d.score, d.weight
                )
            })
            .collect();
        println!(
            "{{\"composite\":{},\"grade\":\"{}\",\"static_grade\":\"{}\",\"runtime_grade\":{},\"hard_fail\":{},\"dimensions\":[{}]}}",
            result.composite,
            result.grade,
            result.static_grade,
            result.runtime_grade.map_or("null".to_string(), |g| format!("\"{g}\"")),
            result.hard_fail,
            dims.join(","),
        );
    } else {
        print!("{}", scoring::format_score_report(&result));
    }

    // Exit 0 for A-C static grade, exit 1 for D-F
    if result.static_grade == 'D' || result.static_grade == 'F' {
        Err(format!("grade {} — below threshold", result.grade))
    } else {
        Ok(())
    }
}

/// FJ-3020: Build RuntimeData from events.jsonl across all machines.
///
/// Reads all machine directories under state_dir, finds apply_completed events,
/// and constructs RuntimeData from the most recent 1-2 apply runs.
fn build_runtime_data(state_dir: &Path) -> Option<scoring::RuntimeData> {
    if !state_dir.exists() {
        return None;
    }

    // Collect all apply_completed events across all machines
    let mut apply_events: Vec<ApplyEvent> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let events_path = entry.path().join("events.jsonl");
            if events_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&events_path) {
                    for line in content.lines() {
                        if let Ok(te) = serde_json::from_str::<TimestampedEvent>(line) {
                            if let ProvenanceEvent::ApplyCompleted {
                                resources_converged,
                                resources_unchanged: _,
                                resources_failed,
                                total_seconds,
                                ..
                            } = &te.event
                            {
                                apply_events.push(ApplyEvent {
                                    ts: te.ts.clone(),
                                    resources_converged: *resources_converged,
                                    resources_failed: *resources_failed,
                                    total_seconds: *total_seconds,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    if apply_events.is_empty() {
        return None;
    }

    // Sort by timestamp (most recent last)
    apply_events.sort_by(|a, b| a.ts.cmp(&b.ts));

    let first = apply_events.last()?; // most recent
    let second = if apply_events.len() >= 2 {
        Some(&apply_events[apply_events.len() - 2])
    } else {
        None
    };

    // Check if state.lock.yaml exists for any machine
    let state_lock_written = state_dir
        .read_dir()
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .any(|e| e.path().join("state.lock.yaml").exists())
        })
        .unwrap_or(false);

    Some(scoring::RuntimeData {
        validate_pass: true, // if we got here, config parsed
        plan_pass: true,     // if apply ran, plan passed
        first_apply_pass: first.resources_failed == 0,
        all_resources_converged: first.resources_converged > 0 && first.resources_failed == 0,
        first_apply_ms: (first.total_seconds * 1000.0) as u64,
        second_apply_pass: second.map(|s| s.resources_failed == 0).unwrap_or(false),
        zero_changes_on_reapply: second
            .map(|s| s.resources_converged == 0 && s.resources_failed == 0)
            .unwrap_or(false),
        hash_stable: state_lock_written, // approximate: if lock exists, hashes are stored
        state_lock_written,
        warning_count: 0,
        changed_on_reapply: second.map(|s| s.resources_converged).unwrap_or(0),
        second_apply_ms: second
            .map(|s| (s.total_seconds * 1000.0) as u64)
            .unwrap_or(0),
    })
}

/// Extracted apply event data for runtime scoring.
struct ApplyEvent {
    ts: String,
    resources_converged: u32,
    resources_failed: u32,
    total_seconds: f64,
}
