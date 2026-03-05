//! FJ-1452: Configuration drift prediction.
//!
//! Analyzes historical event logs to predict which resources are most
//! likely to drift. Computes drift rate, mean time between drifts,
//! trend direction, and risk score for each resource.

use super::helpers::*;
use std::collections::HashMap;
use std::path::Path;

struct DriftPrediction {
    resource: String,
    machine: String,
    drift_count: usize,
    total_events: usize,
    drift_rate: f64,
    mean_time_between_drifts: f64,
    trend: &'static str,
    risk_score: f64,
}

struct DriftPredictReport {
    predictions: Vec<DriftPrediction>,
    total_analyzed: usize,
    high_risk_count: usize,
}

struct EventRecord {
    resource: String,
    machine: String,
    action: String,
    timestamp: f64,
}

fn parse_events(state_dir: &Path) -> Vec<EventRecord> {
    let mut events = Vec::new();
    collect_events_recursive(state_dir, &mut events);
    events.sort_by(|a, b| {
        a.timestamp
            .partial_cmp(&b.timestamp)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    events
}

fn collect_events_recursive(dir: &Path, events: &mut Vec<EventRecord>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_events_recursive(&path, events);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            parse_events_file(&path, events);
        }
    }
}

fn parse_events_file(path: &Path, events: &mut Vec<EventRecord>) {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    // Derive machine name from filename: <machine>.events.jsonl or events.jsonl
    let machine = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .strip_suffix(".events")
        .unwrap_or("default")
        .to_string();

    for line in content.lines() {
        let parsed: serde_json::Value =
            serde_json::from_str(line).unwrap_or(serde_json::Value::Null);
        let resource = parsed["resource"].as_str().unwrap_or("").to_string();
        if resource.is_empty() {
            continue;
        }
        // Support both "action" (synthetic test format) and "event" (real forjar format)
        let action = parsed["action"]
            .as_str()
            .or_else(|| parsed["event"].as_str())
            .unwrap_or("apply")
            .to_string();
        // Support numeric timestamps and ISO 8601 strings
        let ts = parsed["timestamp"]
            .as_f64()
            .or_else(|| parsed["ts"].as_f64())
            .or_else(|| parse_iso_timestamp(parsed["timestamp"].as_str()))
            .or_else(|| parse_iso_timestamp(parsed["ts"].as_str()))
            .unwrap_or(0.0);
        let m = parsed["machine"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| machine.clone());
        events.push(EventRecord {
            resource,
            machine: m,
            action,
            timestamp: ts,
        });
    }
}

/// Parse ISO 8601 timestamp string to seconds since epoch.
fn parse_iso_timestamp(s: Option<&str>) -> Option<f64> {
    let s = s?;
    // Handle "YYYY-MM-DDTHH:MM:SSZ" and "YYYY-MM-DDTHH:MM:SS+00:00" formats
    // Simple parser: split on non-digit boundaries, compute seconds
    let s = s.trim_end_matches('Z');
    let s = if let Some(idx) = s.rfind('+') {
        &s[..idx]
    } else if let Some(idx) = s.rfind('-') {
        // Careful: don't split on date hyphens, only timezone offset
        if idx > 19 {
            &s[..idx]
        } else {
            s
        }
    } else {
        s
    };
    let parts: Vec<&str> = s.split(|c: char| !c.is_ascii_digit()).collect();
    if parts.len() < 6 {
        return None;
    }
    let y: i64 = parts[0].parse().ok()?;
    let mo: i64 = parts[1].parse().ok()?;
    let d: i64 = parts[2].parse().ok()?;
    let h: i64 = parts[3].parse().ok()?;
    let mi: i64 = parts[4].parse().ok()?;
    let se: i64 = parts[5].parse().ok()?;
    // Approximate epoch seconds (good enough for drift comparison)
    let days = (y - 1970) * 365 + (y - 1969) / 4 + month_days(mo) + d - 1;
    Some((days * 86400 + h * 3600 + mi * 60 + se) as f64)
}

fn month_days(month: i64) -> i64 {
    const CUMULATIVE: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    if (1..=12).contains(&month) {
        CUMULATIVE[(month - 1) as usize]
    } else {
        0
    }
}

fn compute_predictions(events: &[EventRecord], machine_filter: Option<&str>) -> DriftPredictReport {
    // Group events by (resource, machine)
    let mut groups: HashMap<(String, String), Vec<&EventRecord>> = HashMap::new();
    for ev in events {
        if let Some(m) = machine_filter {
            if ev.machine != m {
                continue;
            }
        }
        groups
            .entry((ev.resource.clone(), ev.machine.clone()))
            .or_default()
            .push(ev);
    }

    let mut predictions = Vec::new();

    for ((resource, machine), evts) in &groups {
        let total_events = evts.len();
        let drift_count = evts.iter().filter(|e| is_drift_event(&e.action)).count();

        let drift_rate = if total_events > 0 {
            drift_count as f64 / total_events as f64
        } else {
            0.0
        };

        let drift_timestamps: Vec<f64> = evts
            .iter()
            .filter(|e| is_drift_event(&e.action))
            .map(|e| e.timestamp)
            .collect();

        let mean_time_between = compute_mtbd(&drift_timestamps);
        let trend = compute_trend(&drift_timestamps);
        let risk_score = compute_risk(drift_rate, drift_count, trend);

        predictions.push(DriftPrediction {
            resource: resource.clone(),
            machine: machine.clone(),
            drift_count,
            total_events,
            drift_rate,
            mean_time_between_drifts: mean_time_between,
            trend,
            risk_score,
        });
    }

    predictions.sort_by(|a, b| {
        b.risk_score
            .partial_cmp(&a.risk_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total_analyzed = groups.len();
    let high_risk_count = predictions.iter().filter(|p| p.risk_score > 0.7).count();

    DriftPredictReport {
        predictions,
        total_analyzed,
        high_risk_count,
    }
}

fn is_drift_event(action: &str) -> bool {
    matches!(
        action,
        "drift" | "drift_detected" | "resource_drifted" | "remediate" | "changed"
    )
}

fn compute_mtbd(timestamps: &[f64]) -> f64 {
    if timestamps.len() < 2 {
        return 0.0;
    }
    let mut intervals: Vec<f64> = Vec::new();
    for w in timestamps.windows(2) {
        intervals.push(w[1] - w[0]);
    }
    intervals.iter().sum::<f64>() / intervals.len() as f64
}

fn compute_trend(timestamps: &[f64]) -> &'static str {
    if timestamps.len() < 3 {
        return "stable";
    }
    let mid = timestamps.len() / 2;
    let first_half = &timestamps[..mid];
    let second_half = &timestamps[mid..];
    let avg_first = if first_half.is_empty() {
        0.0
    } else {
        first_half.len() as f64
    };
    let avg_second = if second_half.is_empty() {
        0.0
    } else {
        second_half.len() as f64
    };

    if avg_second > avg_first * 1.3 {
        "increasing"
    } else if avg_second < avg_first * 0.7 {
        "decreasing"
    } else {
        "stable"
    }
}

fn compute_risk(drift_rate: f64, drift_count: usize, trend: &str) -> f64 {
    let base = drift_rate * 0.5 + (drift_count as f64 * 0.05).min(0.3);
    let trend_multiplier = match trend {
        "increasing" => 1.3,
        "decreasing" => 0.7,
        _ => 1.0,
    };
    (base * trend_multiplier).min(1.0)
}

pub(crate) fn cmd_drift_predict(
    state_dir: &Path,
    machine: Option<&str>,
    limit: usize,
    json: bool,
) -> Result<(), String> {
    let events = parse_events(state_dir);
    let mut report = compute_predictions(&events, machine);
    if limit > 0 {
        report.predictions.truncate(limit);
    }

    if json {
        print_drift_json(&report);
    } else {
        print_drift_text(&report);
    }
    Ok(())
}

fn print_drift_json(r: &DriftPredictReport) {
    let items: Vec<String> = r
        .predictions
        .iter()
        .map(|p| {
            format!(
                r#"{{"resource":"{}","machine":"{}","drift_count":{},"total_events":{},"drift_rate":{:.3},"mtbd":{:.1},"trend":"{}","risk":{:.3}}}"#,
                p.resource, p.machine, p.drift_count, p.total_events,
                p.drift_rate, p.mean_time_between_drifts, p.trend, p.risk_score
            )
        })
        .collect();
    println!(
        r#"{{"total_analyzed":{},"high_risk":{},"predictions":[{}]}}"#,
        r.total_analyzed,
        r.high_risk_count,
        items.join(","),
    );
}

fn print_drift_text(r: &DriftPredictReport) {
    println!("{}\n", bold("Drift Prediction Report"));
    println!("  Analyzed:   {} resource(s)", r.total_analyzed);
    println!("  High risk:  {}\n", r.high_risk_count);

    for p in &r.predictions {
        let risk_str = format!("{:.1}%", p.risk_score * 100.0);
        let risk_colored = if p.risk_score > 0.7 {
            red(&risk_str)
        } else if p.risk_score > 0.3 {
            yellow(&risk_str)
        } else {
            green(&risk_str)
        };
        println!(
            "  {} on {} — risk {} | drifts {}/{} | trend {} | mtbd {:.0}s",
            p.resource,
            p.machine,
            risk_colored,
            p.drift_count,
            p.total_events,
            p.trend,
            p.mean_time_between_drifts,
        );
    }

    if r.predictions.is_empty() {
        println!(
            "  {} No drift events found in state directory",
            dim("Note:")
        );
    }
}
