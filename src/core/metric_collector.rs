//! FJ-3105: System metric collector — reads /proc for CPU, memory, disk, load.
//!
//! All functions return `Option` and silently fail on non-Linux systems.

use std::collections::HashMap;

/// Collect system metrics from /proc (Linux only).
///
/// Returns a map of metric name to current value.
pub fn collect_system_metrics() -> HashMap<String, f64> {
    let mut metrics = HashMap::new();

    if let Some(cpu) = read_cpu_percent() {
        metrics.insert("cpu_percent".into(), cpu);
    }

    if let Some((used_pct, available_mb)) = read_memory_info() {
        metrics.insert("memory_percent".into(), used_pct);
        metrics.insert("memory_available_mb".into(), available_mb);
    }

    if let Some(disk_pct) = read_disk_usage("/") {
        metrics.insert("disk_percent".into(), disk_pct);
    }

    if let Some(load) = read_load_average() {
        metrics.insert("load_1m".into(), load);
    }

    metrics
}

/// Read overall CPU usage from /proc/stat (non-idle ratio since boot).
///
/// Parses the first `cpu` line and computes the fraction of non-idle time.
pub fn read_cpu_percent() -> Option<f64> {
    let content = std::fs::read_to_string("/proc/stat").ok()?;
    parse_cpu_percent(&content)
}

/// Parse CPU percentage from /proc/stat content.
pub(crate) fn parse_cpu_percent(content: &str) -> Option<f64> {
    let line = content.lines().find(|l| l.starts_with("cpu "))?;
    let fields: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|f| f.parse().ok())
        .collect();
    // fields: user, nice, system, idle, iowait, irq, softirq, steal
    if fields.len() < 4 {
        return None;
    }
    let idle = fields[3] + fields.get(4).copied().unwrap_or(0);
    let total: u64 = fields.iter().sum();
    if total == 0 {
        return None;
    }
    let busy = total - idle;
    Some((busy as f64 / total as f64) * 100.0)
}

/// Read memory info from /proc/meminfo.
///
/// Returns (used_percent, available_mb).
pub fn read_memory_info() -> Option<(f64, f64)> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    parse_memory_info(&content)
}

/// Parse memory info from /proc/meminfo content.
pub(crate) fn parse_memory_info(content: &str) -> Option<(f64, f64)> {
    let total_kb = parse_meminfo_field(content, "MemTotal:")?;
    let available_kb = parse_meminfo_field(content, "MemAvailable:")?;
    if total_kb == 0 {
        return None;
    }
    let used_kb = total_kb.saturating_sub(available_kb);
    let used_pct = (used_kb as f64 / total_kb as f64) * 100.0;
    let available_mb = available_kb as f64 / 1024.0;
    Some((used_pct, available_mb))
}

/// Extract a numeric kB field from /proc/meminfo.
fn parse_meminfo_field(content: &str, key: &str) -> Option<u64> {
    content
        .lines()
        .find(|l| l.starts_with(key))
        .and_then(|line| line.split_whitespace().nth(1).and_then(|v| v.parse().ok()))
}

/// Read disk usage by parsing `df -P <path>` output.
///
/// Returns the percentage of disk used.
pub fn read_disk_usage(path: &str) -> Option<f64> {
    let output = std::process::Command::new("df")
        .args(["-P", path])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_disk_usage_output(&stdout)
}

/// Parse disk usage percentage from `df -P` output.
pub(crate) fn parse_disk_usage_output(output: &str) -> Option<f64> {
    // Skip header line, take first data line
    let data_line = output.lines().nth(1)?;
    // Fields: Filesystem 1024-blocks Used Available Capacity Mounted
    let pct_field = data_line.split_whitespace().nth(4)?;
    let pct_str = pct_field.trim_end_matches('%');
    pct_str.parse().ok()
}

/// Read 1-minute load average from /proc/loadavg.
pub fn read_load_average() -> Option<f64> {
    let content = std::fs::read_to_string("/proc/loadavg").ok()?;
    parse_load_average(&content)
}

/// Parse 1-minute load average from /proc/loadavg content.
pub(crate) fn parse_load_average(content: &str) -> Option<f64> {
    content
        .split_whitespace()
        .next()
        .and_then(|v| v.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cpu_percent_normal() {
        let content = "cpu  100 20 30 800 10 5 3 2 0 0\ncpu0 50 10 15 400 5 3 1 1 0 0\n";
        let pct = parse_cpu_percent(content).unwrap();
        // total = 100+20+30+800+10+5+3+2 = 970
        // idle = 800+10 = 810
        // busy = 160
        // pct = 160/970 * 100 = ~16.49
        assert!((pct - 16.49).abs() < 0.1, "got {pct}");
    }

    #[test]
    fn parse_cpu_percent_minimal() {
        let content = "cpu  50 0 50 100\n";
        let pct = parse_cpu_percent(content).unwrap();
        // total = 200, idle = 100, busy = 100
        assert!((pct - 50.0).abs() < 0.01);
    }

    #[test]
    fn parse_cpu_percent_all_idle() {
        let content = "cpu  0 0 0 1000 0 0 0 0\n";
        let pct = parse_cpu_percent(content).unwrap();
        assert!((pct - 0.0).abs() < 0.01);
    }

    #[test]
    fn parse_cpu_percent_no_cpu_line() {
        let content = "intr 12345\n";
        assert!(parse_cpu_percent(content).is_none());
    }

    #[test]
    fn parse_cpu_percent_too_few_fields() {
        let content = "cpu  10 20\n";
        assert!(parse_cpu_percent(content).is_none());
    }

    #[test]
    fn parse_cpu_percent_zero_total() {
        let content = "cpu  0 0 0 0 0 0 0 0\n";
        assert!(parse_cpu_percent(content).is_none());
    }

    #[test]
    fn parse_memory_info_normal() {
        let content = "MemTotal:       16384000 kB\nMemFree:         2000000 kB\nMemAvailable:    8192000 kB\n";
        let (used_pct, avail_mb) = parse_memory_info(content).unwrap();
        // used = 16384000 - 8192000 = 8192000
        // pct = 8192000 / 16384000 * 100 = 50
        assert!((used_pct - 50.0).abs() < 0.01, "used: {used_pct}");
        // avail = 8192000 / 1024 = 8000
        assert!((avail_mb - 8000.0).abs() < 0.01, "avail: {avail_mb}");
    }

    #[test]
    fn parse_memory_info_zero_total() {
        let content = "MemTotal:       0 kB\nMemAvailable:   0 kB\n";
        assert!(parse_memory_info(content).is_none());
    }

    #[test]
    fn parse_memory_info_missing_available() {
        let content = "MemTotal:       16384000 kB\nMemFree:         2000000 kB\n";
        assert!(parse_memory_info(content).is_none());
    }

    #[test]
    fn parse_memory_info_missing_total() {
        let content = "MemAvailable:    8192000 kB\n";
        assert!(parse_memory_info(content).is_none());
    }

    #[test]
    fn parse_disk_usage_output_normal() {
        let output = "Filesystem     1024-blocks      Used Available Capacity Mounted on\n/dev/sda1       100000000  60000000  40000000      60% /\n";
        let pct = parse_disk_usage_output(output).unwrap();
        assert!((pct - 60.0).abs() < 0.01);
    }

    #[test]
    fn parse_disk_usage_output_full() {
        let output = "Filesystem     1024-blocks      Used Available Capacity Mounted on\n/dev/sda1       100000000  99000000   1000000      99% /\n";
        let pct = parse_disk_usage_output(output).unwrap();
        assert!((pct - 99.0).abs() < 0.01);
    }

    #[test]
    fn parse_disk_usage_output_empty() {
        let output = "Filesystem     1024-blocks      Used Available Capacity Mounted on\n";
        assert!(parse_disk_usage_output(output).is_none());
    }

    #[test]
    fn parse_disk_usage_output_no_header() {
        assert!(parse_disk_usage_output("").is_none());
    }

    #[test]
    fn parse_load_average_normal() {
        let content = "0.52 0.45 0.31 1/234 5678\n";
        let load = parse_load_average(content).unwrap();
        assert!((load - 0.52).abs() < 0.001);
    }

    #[test]
    fn parse_load_average_high() {
        let content = "12.34 8.00 5.50 3/500 9999\n";
        let load = parse_load_average(content).unwrap();
        assert!((load - 12.34).abs() < 0.001);
    }

    #[test]
    fn parse_load_average_empty() {
        assert!(parse_load_average("").is_none());
    }

    #[test]
    fn parse_load_average_invalid() {
        assert!(parse_load_average("not-a-number rest").is_none());
    }

    #[test]
    fn collect_system_metrics_returns_map() {
        // On Linux CI this should populate; on non-Linux it's empty.
        let metrics = collect_system_metrics();
        // We just verify it doesn't panic and returns a HashMap.
        assert!(metrics.len() <= 5);
    }

    #[test]
    fn parse_cpu_percent_with_iowait() {
        // 8 fields: user nice system idle iowait irq softirq steal
        let content = "cpu  1000 200 300 5000 500 100 50 20 0 0\n";
        let pct = parse_cpu_percent(content).unwrap();
        // total = 1000+200+300+5000+500+100+50+20 = 7170
        // idle = 5000+500 = 5500
        // busy = 1670
        // pct = 1670/7170 * 100 = ~23.29
        assert!((pct - 23.29).abs() < 0.1, "got {pct}");
    }

    #[test]
    fn parse_memory_info_high_usage() {
        let content =
            "MemTotal:       8000000 kB\nMemFree:        100000 kB\nMemAvailable:   500000 kB\n";
        let (used_pct, avail_mb) = parse_memory_info(content).unwrap();
        // used = 8000000 - 500000 = 7500000
        // pct = 7500000 / 8000000 * 100 = 93.75
        assert!((used_pct - 93.75).abs() < 0.01);
        // avail = 500000 / 1024 ~ 488.28
        assert!((avail_mb - 488.28).abs() < 0.1);
    }

    #[test]
    fn parse_disk_usage_output_zero_percent() {
        let output = "Filesystem     1024-blocks      Used Available Capacity Mounted on\n/dev/sda1       100000000         0 100000000       0% /\n";
        let pct = parse_disk_usage_output(output).unwrap();
        assert!((pct - 0.0).abs() < 0.01);
    }
}
