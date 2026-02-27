//! Trends and predictions.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use std::sync::atomic::Ordering;
use std::collections::HashMap;


/// FJ-662: Show how often each resource changes
fn count_changes_from_logs(
    state_dir: &Path,
    targets: &[&String],
) -> Vec<(String, usize)> {
    let mut change_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for m in targets {
        let log_path = state_dir.join(format!("{}.events.jsonl", m));
        if !log_path.exists() { continue; }
        let content = std::fs::read_to_string(&log_path).unwrap_or_default();
        for line in content.lines() {
            if let Ok(event) = serde_yaml_ng::from_str::<
                std::collections::HashMap<String, serde_yaml_ng::Value>,
            >(line) {
                if let Some(serde_yaml_ng::Value::String(resource)) = event.get("resource") {
                    *change_counts.entry(resource.clone()).or_insert(0) += 1;
                }
            }
        }
    }
    let mut sorted: Vec<_> = change_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted
}

pub(crate) fn cmd_status_change_frequency(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = if let Some(m) = machine {
        machines.iter().filter(|x| x.as_str() == m).collect()
    } else {
        machines.iter().collect()
    };
    let sorted = count_changes_from_logs(state_dir, &targets);

    if json {
        print!("{{\"frequencies\":[");
        for (i, (name, count)) in sorted.iter().enumerate() {
            if i > 0 { print!(","); }
            print!(r#"{{"resource":"{}","changes":{}}}"#, name, count);
        }
        println!("]}}");
    } else if sorted.is_empty() {
        println!("No change history found in event logs");
    } else {
        println!("Resource change frequency:");
        for (name, count) in &sorted {
            println!("  {} — {} changes", name, count);
        }
    }
    Ok(())
}


/// FJ-692: Show duration of last apply per resource
fn collect_apply_durations(
    state_dir: &Path,
    targets: &[&String],
) -> Vec<(String, Vec<(String, f64)>)> {
    let mut result = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if let Ok(data) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&data) {
                let resources: Vec<(String, f64)> = lock.resources.iter()
                    .map(|(name, rl)| (name.clone(), rl.duration_seconds.unwrap_or(0.0)))
                    .collect();
                result.push((m.to_string(), resources));
            }
        }
    }
    result
}

pub(crate) fn cmd_status_last_apply_duration(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let data = collect_apply_durations(state_dir, &targets);
    if json {
        let mut entries = Vec::new();
        for (m, resources) in &data {
            for (name, dur) in resources {
                entries.push(format!(
                    "{{\"machine\":\"{}\",\"resource\":\"{}\",\"duration_seconds\":{}}}",
                    m, name, dur
                ));
            }
        }
        println!("{{\"last_apply_duration\":[{}]}}", entries.join(","));
    } else {
        println!("Last apply duration per resource:");
        for (m, resources) in &data {
            println!("  Machine: {}", m);
            for (name, dur) in resources {
                println!("    {} — {:.3}s", name, dur);
            }
        }
    }
    Ok(())
}


/// FJ-517: Status trend — show status trend over last N applies.
pub(crate) fn cmd_status_trend(
    state_dir: &Path,
    machine: Option<&str>,
    n: usize,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let machines: Vec<String> = if let Some(m) = machine {
        machines.into_iter().filter(|nm| nm == m).collect()
    } else {
        machines
    };

    let mut trend_data: Vec<(String, String, String)> = Vec::new(); // (timestamp, machine, summary)

    for m in &machines {
        let events_path = state_dir.join(format!("{}.events.jsonl", m));
        if !events_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&events_path).unwrap_or_default();
        let lines: Vec<&str> = content.lines().collect();
        let start = if lines.len() > n { lines.len() - n } else { 0 };

        for line in &lines[start..] {
            let parsed: serde_json::Value =
                serde_json::from_str(line).unwrap_or(serde_json::Value::Null);
            let ts = parsed["timestamp"].as_str().unwrap_or("?").to_string();
            let status = parsed["status"].as_str().unwrap_or("?").to_string();
            let resource = parsed["resource"].as_str().unwrap_or("?").to_string();
            trend_data.push((ts, m.clone(), format!("{}={}", resource, status)));
        }
    }

    // Sort by timestamp
    trend_data.sort_by(|a, b| a.0.cmp(&b.0));
    let last_n: Vec<_> = if trend_data.len() > n {
        trend_data[trend_data.len() - n..].to_vec()
    } else {
        trend_data
    };

    if json {
        let entries: Vec<String> = last_n
            .iter()
            .map(|(ts, m, s)| {
                format!(
                    r#"{{"timestamp":"{}","machine":"{}","event":"{}"}}"#,
                    ts, m, s
                )
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else if last_n.is_empty() {
        println!("No trend data available.");
    } else {
        println!("Status trend (last {} events):\n", last_n.len());
        for (ts, m, s) in &last_n {
            println!("  [{}] {} — {}", ts, m, s);
        }
    }
    Ok(())
}


/// FJ-522: Prediction — predict next failure based on historical patterns.
fn collect_risk_scores_for_machine(
    state_dir: &Path,
    m: &str,
    risk_scores: &mut Vec<(String, String, f64, String)>,
) {
    let events_path = state_dir.join(format!("{}.events.jsonl", m));
    if !events_path.exists() {
        return;
    }
    let content = std::fs::read_to_string(&events_path).unwrap_or_default();
    let mut failure_count: HashMap<String, usize> = HashMap::new();
    let mut total_count: HashMap<String, usize> = HashMap::new();

    for line in content.lines() {
        let parsed: serde_json::Value =
            serde_json::from_str(line).unwrap_or(serde_json::Value::Null);
        let resource = parsed["resource"].as_str().unwrap_or("").to_string();
        let status = parsed["status"].as_str().unwrap_or("");
        if resource.is_empty() {
            continue;
        }
        *total_count.entry(resource.clone()).or_insert(0) += 1;
        if status == "Failed" || status == "Drifted" {
            *failure_count.entry(resource.clone()).or_insert(0) += 1;
        }
    }

    for (resource, total) in &total_count {
        let failures = failure_count.get(resource).copied().unwrap_or(0);
        if failures > 0 && *total >= 3 {
            let rate = failures as f64 / *total as f64;
            let reason = format!(
                "{}/{} events failed ({:.0}%)",
                failures, total, rate * 100.0
            );
            risk_scores.push((m.to_string(), resource.clone(), rate, reason));
        }
    }
}

fn risk_level_label(score: f64) -> String {
    if score > 0.5 {
        red("HIGH")
    } else if score > 0.25 {
        yellow("MEDIUM")
    } else {
        "LOW".to_string()
    }
}

pub(crate) fn cmd_status_prediction(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let machines: Vec<String> = if let Some(m) = machine {
        machines.into_iter().filter(|n| n == m).collect()
    } else {
        machines
    };

    let mut risk_scores: Vec<(String, String, f64, String)> = Vec::new();
    for m in &machines {
        collect_risk_scores_for_machine(state_dir, m, &mut risk_scores);
    }
    risk_scores.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    if json {
        let entries: Vec<String> = risk_scores
            .iter()
            .map(|(m, r, s, reason)| {
                format!(
                    r#"{{"machine":"{}","resource":"{}","risk_score":{:.3},"reason":"{}"}}"#,
                    m, r, s, reason
                )
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else if risk_scores.is_empty() {
        println!(
            "{} No failure patterns detected. All resources stable.",
            green("✓")
        );
    } else {
        println!("Failure prediction (by historical failure rate):\n");
        for (m, r, score, reason) in &risk_scores {
            let level = risk_level_label(*score);
            println!("  [{}] {}:{} — {}", level, m, r, reason);
        }
    }
    Ok(())
}


// ── FJ-472: status --histogram ──

pub(crate) fn cmd_status_histogram(state_dir: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let all_machines = discover_machines(state_dir);
    let machines: Vec<String> = if let Some(m) = machine {
        all_machines.into_iter().filter(|n| n == m).collect()
    } else {
        all_machines
    };
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for m in &machines {
        if let Some(lock) = state::load_lock(state_dir, m).map_err(|e| e.to_string())? {
            for rl in lock.resources.values() {
                let status_str = format!("{:?}", rl.status);
                *counts.entry(status_str).or_insert(0) += 1;
            }
        }
    }
    if json {
        let result = serde_json::json!({ "histogram": counts });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else {
        println!("Resource Status Histogram");
        println!("{}", "─".repeat(40));
        let max_count = counts.values().copied().max().unwrap_or(1);
        let mut sorted: Vec<_> = counts.iter().collect();
        sorted.sort_by_key(|(k, _)| (*k).clone());
        for (status, count) in &sorted {
            let bar_width = (*count * 30) / max_count.max(1);
            let bar: String = "█".repeat(bar_width);
            println!("  {:12} {:>4} {}", status, count, bar);
        }
    }
    Ok(())
}


/// FJ-512: MTTR — mean time to recovery per resource.
pub(crate) fn cmd_status_mttr(state_dir: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let machines: Vec<String> = if let Some(m) = machine {
        machines.into_iter().filter(|n| n == m).collect()
    } else {
        machines
    };

    let mut recovery_times: Vec<(String, String, f64)> = Vec::new(); // (machine, resource, hours)

    for m in &machines {
        let events_path = state_dir.join(format!("{}.events.jsonl", m));
        if !events_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&events_path).unwrap_or_default();
        let mut failure_start: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut total_recovery: std::collections::HashMap<String, (f64, usize)> =
            std::collections::HashMap::new();

        for line in content.lines() {
            let parsed: serde_json::Value =
                serde_json::from_str(line).unwrap_or(serde_json::Value::Null);
            let resource = parsed["resource"].as_str().unwrap_or("").to_string();
            let status = parsed["status"].as_str().unwrap_or("");
            let ts = parsed["timestamp"].as_str().unwrap_or("").to_string();

            if resource.is_empty() {
                continue;
            }

            if status == "Failed" || status == "Drifted" {
                failure_start.entry(resource.clone()).or_insert(ts);
            } else if status == "Converged" {
                if let Some(start_ts) = failure_start.remove(&resource) {
                    // Estimate recovery time from timestamp difference
                    let hours = estimate_hours_between(&start_ts, &ts);
                    let entry = total_recovery.entry(resource.clone()).or_insert((0.0, 0));
                    entry.0 += hours;
                    entry.1 += 1;
                }
            }
        }

        for (resource, (total, count)) in &total_recovery {
            let avg = total / (*count as f64);
            recovery_times.push((m.clone(), resource.clone(), avg));
        }
    }

    recovery_times.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    if json {
        let entries: Vec<String> = recovery_times
            .iter()
            .map(|(m, r, h)| {
                format!(
                    r#"{{"machine":"{}","resource":"{}","mttr_hours":{:.2}}}"#,
                    m, r, h
                )
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else if recovery_times.is_empty() {
        println!("{} No recovery events found.", green("✓"));
    } else {
        println!("Mean Time To Recovery:\n");
        for (m, r, h) in &recovery_times {
            println!("  {}:{} — {:.2}h MTTR", m, r, h);
        }
    }
    Ok(())
}

