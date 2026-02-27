//! Time and duration parsing helpers.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;


pub(crate) fn chrono_date() -> String {
    // Simple date without chrono dependency
    let output = std::process::Command::new("date").arg("+%Y-%m-%d").output();
    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => "unknown".to_string(),
    }
}


/// Compact timestamp for snapshot names (YYYYMMDD-HHMMSS).
pub(crate) fn chrono_now_compact() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple Unix timestamp — good enough for unique naming
    format!("{}", now)
}


/// FJ-284: Parse a human duration string like "24h", "7d", "30m" into seconds.
pub(crate) fn parse_duration_secs(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.len() < 2 {
        return Err(format!("invalid duration: '{}'", s));
    }
    let (num, unit) = s.split_at(s.len() - 1);
    let n: u64 = num
        .parse()
        .map_err(|_| format!("invalid duration number: '{}'", num))?;
    match unit {
        "s" => Ok(n),
        "m" => Ok(n * 60),
        "h" => Ok(n * 3600),
        "d" => Ok(n * 86400),
        _ => Err(format!("unknown duration unit '{}' (use s/m/h/d)", unit)),
    }
}


pub(crate) fn parse_duration_string(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty duration string".to_string());
    }
    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: u64 = num_str
        .parse()
        .map_err(|_| format!("invalid duration: {}", s))?;
    match unit {
        "s" => Ok(num),
        "m" => Ok(num * 60),
        "h" => Ok(num * 3600),
        "d" => Ok(num * 86400),
        _ => Err(format!(
            "unknown duration unit '{}'. Use s, m, h, or d",
            unit
        )),
    }
}


/// Estimate hours between two ISO-ish timestamp strings.
pub(crate) fn estimate_hours_between(start: &str, end: &str) -> f64 {
    // Simple extraction: parse "YYYY-MM-DDTHH:MM:SS" prefix
    let parse_secs = |s: &str| -> Option<i64> {
        if s.len() < 19 {
            return None;
        }
        let hours: i64 = s[11..13].parse().ok()?;
        let mins: i64 = s[14..16].parse().ok()?;
        let secs: i64 = s[17..19].parse().ok()?;
        let day: i64 = s[8..10].parse().ok()?;
        Some(day * 86400 + hours * 3600 + mins * 60 + secs)
    };
    match (parse_secs(start), parse_secs(end)) {
        (Some(s), Some(e)) => {
            let diff = (e - s).max(0) as f64;
            diff / 3600.0
        }
        _ => 1.0, // default 1 hour if unparseable
    }
}

