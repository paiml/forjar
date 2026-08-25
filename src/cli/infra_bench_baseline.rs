//! Baseline loading and verdict for `forjar bench --compare`.
//!
//! Dogfood #208 (family #211, `bench-compare-flag-is-a-noop`): `--compare`
//! parsed fine and then silently did nothing whenever the baseline was
//! missing or in an unexpected shape — output was structurally identical to a
//! run without the flag, even against a baseline that made the current run a
//! ~180x regression. A flag that names an input must fail when that input
//! cannot be read.

use std::path::{Path, PathBuf};

/// Default location of the stored baseline.
pub(crate) const DEFAULT_BASELINE_PATH: &str = "benchmarks/RESULTS.md";

/// Percentage slower than baseline that counts as a regression.
pub(crate) const REGRESSION_PCT: f64 = 50.0;

/// One row parsed from the baseline table.
#[derive(Debug)]
pub(crate) struct BaselineEntry {
    /// Benchmark name, as printed in the first column.
    pub(crate) name: String,
    /// Recorded average, in microseconds.
    pub(crate) avg_us: f64,
}

/// Path of the baseline file used by `--compare`.
pub(crate) fn default_baseline_path() -> PathBuf {
    PathBuf::from(DEFAULT_BASELINE_PATH)
}

/// Load the baseline table, or explain why it cannot be used.
///
/// Refuses rather than degrading to "no comparison": a missing file, an
/// unreadable path (including a directory), and a file without a parseable
/// `BENCH-TABLE-START` block are all errors.
pub(crate) fn load_baseline_from(path: &Path) -> Result<Vec<BaselineEntry>, String> {
    if !path.exists() {
        return Err(format!(
            "--compare: no baseline at {} — run `forjar bench` and record its table there first",
            path.display()
        ));
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("--compare: cannot read baseline {}: {e}", path.display()))?;
    let entries = parse_baseline_table(&content);
    if entries.is_empty() {
        return Err(format!(
            "--compare: baseline {} has no parseable rows — expected a markdown table between \
             <!-- BENCH-TABLE-START --> and <!-- BENCH-TABLE-END --> with columns \
             | Operation | Target | Last Run | p50 | p95 | Status |",
            path.display()
        ));
    }
    Ok(entries)
}

/// True for a line inside the table that carries data: a markdown row that is
/// neither the `| Operation | ... |` header nor the `---` separator beneath it.
fn is_baseline_data_row(line: &str) -> bool {
    line.starts_with('|') && !line.contains("---") && !line.contains("Operation")
}

/// Parse one `| Operation | Target | Last Run | p50 | p95 | Status |` row.
/// `None` when the row has too few columns, or its "Last Run" cell is not a
/// duration literal — a malformed row is skipped, never guessed at.
fn parse_baseline_row(line: &str) -> Option<BaselineEntry> {
    let cols: Vec<&str> = line.split('|').collect();
    if cols.len() < 5 {
        return None;
    }
    Some(BaselineEntry {
        name: cols[1].trim().to_string(),
        avg_us: parse_duration_to_us(cols[3].trim())?,
    })
}

/// Parse the rows between the BENCH-TABLE markers.
pub(crate) fn parse_baseline_table(content: &str) -> Vec<BaselineEntry> {
    let mut entries = Vec::new();
    let mut in_table = false;
    for line in content.lines() {
        if line.contains("BENCH-TABLE-START") {
            in_table = true;
            continue;
        }
        if line.contains("BENCH-TABLE-END") {
            break;
        }
        if in_table && is_baseline_data_row(line) {
            entries.extend(parse_baseline_row(line));
        }
    }
    entries
}

/// Parse a duration literal such as `148.7µs`, `1.2ms` or `2s` to microseconds.
pub(crate) fn parse_duration_to_us(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some(v) = s.strip_suffix("µs").or_else(|| s.strip_suffix("us")) {
        v.trim().parse().ok()
    } else if let Some(v) = s.strip_suffix("ms") {
        v.trim().parse::<f64>().ok().map(|v| v * 1000.0)
    } else if let Some(v) = s.strip_suffix('s') {
        v.trim().parse::<f64>().ok().map(|v| v * 1_000_000.0)
    } else {
        None
    }
}

/// Summary of a bench run, used to derive the process exit status.
#[derive(Debug)]
pub(crate) struct BenchOutcome {
    /// Number of benchmarks that met their target.
    pub(crate) passed: usize,
    /// Number of benchmarks run.
    pub(crate) total: usize,
    /// Human-readable descriptions of baseline regressions.
    pub(crate) regressions: Vec<String>,
}

/// Map a bench outcome to a CLI result.
///
/// Dogfood #208 (`bench-iterations-zero-nan-and-exit-zero`): the pass/fail
/// tally never reached the exit code, so `✗ 0/6 targets met` exited 0 and any
/// CI job gating on `forjar bench` passed unconditionally.
pub(crate) fn bench_verdict(outcome: &BenchOutcome) -> Result<(), String> {
    if !outcome.regressions.is_empty() {
        return Err(format!(
            "benchmark regression vs baseline: {}",
            outcome.regressions.join("; ")
        ));
    }
    if outcome.passed < outcome.total {
        return Err(format!(
            "{}/{} benchmark targets met",
            outcome.passed, outcome.total
        ));
    }
    Ok(())
}

/// True when `bench` routed and produced a verdict (pass, or a targets-met
/// failure). Debug builds miss the release-tuned budgets, so tests must not
/// assert `is_ok()` — see dogfood #208.
#[cfg(test)]
pub(crate) fn bench_routed(result: &Result<(), String>) -> bool {
    match result {
        Ok(()) => true,
        Err(e) => e.contains("targets met"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_TABLE: &str = "<!-- BENCH-TABLE-START -->\n\
| Operation | Target | Last Run | p50 | p95 | Status |\n\
|---|---|---|---|---|---|\n\
| validate (3m, 20r) | < 10ms | 148.7µs | 139.0µs | 214.0µs | pass |\n\
<!-- BENCH-TABLE-END -->\n";

    #[test]
    fn missing_baseline_is_refused_not_ignored() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let err = load_baseline_from(&tmp.path().join("nope.md")).expect_err("must refuse");
        assert!(err.contains("no baseline"), "{err}");
    }

    #[test]
    fn unparseable_baseline_is_refused() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("RESULTS.md");
        std::fs::write(&p, "| validate (3m, 20r) | 1.0µs |\n").expect("write");
        let err = load_baseline_from(&p).expect_err("must refuse");
        assert!(err.contains("no parseable rows"), "{err}");
    }

    #[test]
    fn directory_baseline_is_refused() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("RESULTS.md");
        std::fs::create_dir(&p).expect("mkdir");
        assert!(load_baseline_from(&p).is_err());
    }

    #[test]
    fn good_baseline_parses() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("RESULTS.md");
        std::fs::write(&p, GOOD_TABLE).expect("write");
        let entries = load_baseline_from(&p).expect("parse");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "validate (3m, 20r)");
        assert!((entries[0].avg_us - 148.7).abs() < 1e-6);
    }

    #[test]
    fn verdict_fails_on_unmet_targets() {
        let outcome = BenchOutcome {
            passed: 0,
            total: 6,
            regressions: Vec::new(),
        };
        let err = bench_verdict(&outcome).expect_err("0/6 must not be success");
        assert!(err.contains("0/6"), "{err}");
    }

    #[test]
    fn verdict_fails_on_regression() {
        let outcome = BenchOutcome {
            passed: 6,
            total: 6,
            regressions: vec!["validate (3m, 20r) +18000%".to_string()],
        };
        assert!(bench_verdict(&outcome).is_err());
    }

    #[test]
    fn verdict_passes_when_all_targets_met() {
        let outcome = BenchOutcome {
            passed: 6,
            total: 6,
            regressions: Vec::new(),
        };
        assert!(bench_verdict(&outcome).is_ok());
    }

    #[test]
    fn duration_parser_handles_units() {
        assert_eq!(parse_duration_to_us("2s"), Some(2_000_000.0));
        assert_eq!(parse_duration_to_us("1.5ms"), Some(1500.0));
        assert_eq!(parse_duration_to_us("3µs"), Some(3.0));
        assert_eq!(parse_duration_to_us("nope"), None);
    }
}
