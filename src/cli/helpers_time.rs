//! Time and duration parsing helpers.

#[cfg(feature = "encryption")]
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
    format!("{now}")
}

/// Multiply a parsed duration value by its unit's seconds-per-unit, in seconds.
///
/// #28: returns `Err` on an unknown unit or on multiply overflow (instead of
/// panicking in debug / silently wrapping in release).
fn duration_unit_secs(num: u64, unit: char, raw: &str) -> Result<u64, String> {
    let per = match unit {
        's' => 1,
        'm' => 60,
        'h' => 3600,
        'd' => 86400,
        _ => return Err(format!("unknown duration unit '{unit}'. Use s, m, h, or d")),
    };
    num.checked_mul(per)
        .ok_or_else(|| format!("duration overflow: {raw}"))
}

/// FJ-284: Parse a human duration string like "24h", "7d", "30m" into seconds.
pub(crate) fn parse_duration_secs(s: &str) -> Result<u64, String> {
    let s = s.trim();
    // #28: split off the trailing unit by char (not byte) so a multi-byte
    // trailing char (e.g. "5µ") cannot land mid-character and panic in split_at.
    let unit = s
        .chars()
        .last()
        .ok_or_else(|| format!("invalid duration: '{s}'"))?;
    let num_str = &s[..s.len() - unit.len_utf8()];
    let n: u64 = num_str
        .parse()
        .map_err(|_| format!("invalid duration number: '{num_str}'"))?;
    duration_unit_secs(n, unit, s)
}

pub(crate) fn parse_duration_string(s: &str) -> Result<u64, String> {
    let s = s.trim();
    // #28: use char-aware splitting + checked_mul; a multi-byte trailing char or
    // an enormous value yields the existing Err string rather than a panic/wrap.
    let unit = s
        .chars()
        .last()
        .ok_or_else(|| "empty duration string".to_string())?;
    let num_str = &s[..s.len() - unit.len_utf8()];
    let num: u64 = num_str
        .parse()
        .map_err(|_| format!("invalid duration: {s}"))?;
    duration_unit_secs(num, unit, s)
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
