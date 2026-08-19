//! FJ-051: ML-inspired drift anomaly detection (aprender-compatible).
//!
//! Provides statistical anomaly detection for infrastructure drift patterns
//! using algorithms inspired by the aprender crate:
//!
//! - **ADWIN** (Adaptive Windowing): detects concept drift in streaming data
//! - **Isolation scoring**: anomaly scores based on isolation depth
//! - **Z-score with EWM**: exponentially weighted z-score for recent bias
//!
//! These are pure-Rust implementations that don't require the aprender crate
//! at runtime — they operate on forjar's event log data.

use serde::{Deserialize, Serialize};

/// Drift status from anomaly detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftStatus {
    /// No anomaly detected.
    Stable,
    /// Marginal deviation — monitor closely.
    Warning,
    /// Significant anomaly — investigate.
    Drift,
}

/// Statistics from drift detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftStats {
    /// Number of observations in the window.
    pub n_samples: u64,
    /// Error rate (fraction of drift events).
    pub error_rate: f64,
    /// Mean of observations.
    pub mean: f64,
    /// Standard deviation of observations.
    pub std_dev: f64,
    /// Current drift status.
    pub status: DriftStatus,
}

/// ADWIN-inspired adaptive windowing detector.
///
/// Maintains a sliding window of observations and detects when the distribution
/// shifts significantly. Based on Bifet & Gavalda 2007.
#[derive(Debug, Clone)]
pub struct AdwinDetector {
    /// Confidence parameter (smaller = more sensitive). Default: 0.002.
    delta: f64,
    /// Observations in the window.
    window: Vec<f64>,
    /// Maximum window size.
    max_window: usize,
    /// Running sum.
    sum: f64,
    /// Current status.
    status: DriftStatus,
}

impl AdwinDetector {
    /// Create a new ADWIN detector with default sensitivity.
    pub fn new() -> Self {
        Self::with_delta(0.002)
    }

    /// Create with custom sensitivity (smaller delta = more sensitive).
    pub fn with_delta(delta: f64) -> Self {
        Self {
            delta,
            window: Vec::new(),
            max_window: 1000,
            sum: 0.0,
            status: DriftStatus::Stable,
        }
    }

    /// Add an observation (e.g., 1.0 for drift event, 0.0 for normal).
    pub fn add_element(&mut self, value: f64) {
        self.window.push(value);
        self.sum += value;

        // Trim window if too large
        if self.window.len() > self.max_window {
            self.sum -= self.window.remove(0);
        }

        self.status = self.detect_change();
    }

    /// Check for distribution change using ADWIN criterion.
    fn detect_change(&self) -> DriftStatus {
        let n = self.window.len();
        if n < 10 {
            return DriftStatus::Stable;
        }

        // Try splits at different points
        let mut max_cut = 0.0;
        for split in (n / 4)..=(3 * n / 4) {
            let left: f64 = self.window[..split].iter().sum();
            let right: f64 = self.window[split..].iter().sum();

            let n_left = split as f64;
            let n_right = (n - split) as f64;

            let mean_left = left / n_left;
            let mean_right = right / n_right;

            let diff = (mean_left - mean_right).abs();

            // ADWIN bound: epsilon = sqrt((1/2m) * ln(2/delta))
            let m = 2.0 / (1.0 / n_left + 1.0 / n_right);
            let epsilon = ((1.0 / (2.0 * m)) * (2.0_f64 / self.delta).ln()).sqrt();

            if diff > epsilon {
                let cut = diff / epsilon;
                if cut > max_cut {
                    max_cut = cut;
                }
            }
        }

        if max_cut > 2.0 {
            DriftStatus::Drift
        } else if max_cut > 1.0 {
            DriftStatus::Warning
        } else {
            DriftStatus::Stable
        }
    }

    /// Get current detection stats.
    pub fn stats(&self) -> DriftStats {
        let n = self.window.len() as u64;
        let mean = if n > 0 { self.sum / n as f64 } else { 0.0 };
        let variance = if n > 1 {
            self.window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64
        } else {
            0.0
        };

        DriftStats {
            n_samples: n,
            error_rate: mean,
            mean,
            std_dev: variance.sqrt(),
            status: self.status.clone(),
        }
    }

    /// Current status.
    pub fn status(&self) -> &DriftStatus {
        &self.status
    }

    /// Reset the detector.
    pub fn reset(&mut self) {
        self.window.clear();
        self.sum = 0.0;
        self.status = DriftStatus::Stable;
    }
}

impl Default for AdwinDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Isolation-based anomaly score for resource metrics.
///
/// Inspired by aprender's IsolationForest. Computes an anomaly score
/// based on how "isolated" a resource's metrics are from the population.
/// Higher score = more anomalous.
pub fn isolation_score(values: &[f64], target: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let std_dev = if values.len() > 1 {
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
        variance.sqrt()
    } else {
        1.0
    };

    if std_dev < f64::EPSILON {
        return if (target - mean).abs() < f64::EPSILON {
            0.0
        } else {
            1.0
        };
    }

    // Rank-based isolation: what fraction of the population is closer to the mean?
    // This is robust to outliers inflating std_dev.
    let distance = (target - mean).abs();
    let closer_count = values
        .iter()
        .filter(|&&v| (v - mean).abs() < distance)
        .count();
    let rank_score = closer_count as f64 / n;

    // Also compute z-score for magnitude
    let z = distance / std_dev;

    // Combine: rank gives relative position, z gives magnitude
    // Use the higher of the two signals
    let z_score = 1.0 - 1.0 / (1.0 + (z / 2.0).powi(2));
    rank_score.max(z_score)
}

/// Exponentially weighted moving average z-score.
///
/// Gives more weight to recent observations, making it sensitive to
/// recent drift while being robust to historical patterns.
pub fn ewma_zscore(values: &[f64], target: f64, alpha: f64) -> f64 {
    let Some(&first) = values.first() else {
        return 0.0;
    };

    // Compute EWMA mean
    let mut ewma = first;
    for &v in values.get(1..).unwrap_or(&[]) {
        ewma = alpha * v + (1.0 - alpha) * ewma;
    }

    // Compute EWMA variance
    let mut ewma_var = 0.0;
    let mut ewma_mean = first;
    for &v in values.get(1..).unwrap_or(&[]) {
        ewma_mean = alpha * v + (1.0 - alpha) * ewma_mean;
        let diff = v - ewma_mean;
        ewma_var = alpha * diff * diff + (1.0 - alpha) * ewma_var;
    }

    let ewma_std = ewma_var.sqrt();
    if ewma_std < f64::EPSILON {
        return 0.0;
    }

    (target - ewma).abs() / ewma_std
}

/// Analyze resource event metrics for anomalies.
///
/// Takes per-resource metrics (converge_count, fail_count, drift_count)
/// and returns anomaly findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyFinding {
    /// Resource identifier (machine:resource).
    pub resource: String,
    /// Anomaly score (0.0-1.0, higher = more anomalous).
    pub score: f64,
    /// Drift status classification.
    pub status: DriftStatus,
    /// Human-readable reasons for the anomaly.
    pub reasons: Vec<String>,
}

/// Run anomaly detection on resource metrics.
pub fn detect_anomalies(
    metrics: &[(String, u32, u32, u32)], // (resource_id, converge, fail, drift)
    min_events: usize,
) -> Vec<AnomalyFinding> {
    let active: Vec<&(String, u32, u32, u32)> = metrics
        .iter()
        .filter(|(_, c, f, d)| (*c + *f + *d) as usize >= min_events)
        .collect();

    if active.is_empty() {
        return Vec::new();
    }

    // Collect converge rates for isolation scoring
    let converge_vals: Vec<f64> = active.iter().map(|(_, c, _, _)| *c as f64).collect();
    let fail_vals: Vec<f64> = active.iter().map(|(_, _, f, _)| *f as f64).collect();

    let mut findings = Vec::new();

    for (key, converge, fail, drift) in active.iter().map(|&&(ref k, c, f, d)| (k, c, f, d)) {
        let (max_score, reasons) =
            score_resource_metrics(converge, fail, drift, &converge_vals, &fail_vals);

        if !reasons.is_empty() {
            findings.push(AnomalyFinding {
                resource: key.clone(),
                score: max_score,
                status: classify_score(max_score),
                reasons,
            });
        }
    }

    // Sort by score descending
    findings.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    findings
}

/// Run duration-outlier detection over per-resource convergence durations.
///
/// Only resources with at least `min_events` samples are considered, so
/// `--min-events` is a real predicate here too.
pub fn detect_duration_anomalies(
    durations: &[(String, Vec<f64>)],
    min_events: usize,
) -> Vec<AnomalyFinding> {
    let mut findings = Vec::new();
    for (key, samples) in durations {
        if samples.len() < min_events.max(1) {
            continue;
        }
        if let Some((score, reason)) = duration_outlier(samples) {
            findings.push(AnomalyFinding {
                resource: key.clone(),
                score,
                status: classify_score(score),
                reasons: vec![reason],
            });
        }
    }
    findings
}

/// Score one resource's metrics against the population, building reasons.
///
/// Returns `(max_score, reasons)` where reasons is empty when no signal fired.
fn score_resource_metrics(
    converge: u32,
    fail: u32,
    drift: u32,
    converge_vals: &[f64],
    fail_vals: &[f64],
) -> (f64, Vec<String>) {
    let mut reasons = Vec::new();
    let mut max_score = 0.0_f64;

    // Isolation score for converge frequency
    let churn_score = isolation_score(converge_vals, converge as f64);
    if churn_score > 0.6 {
        reasons.push(format!(
            "high churn (isolation={churn_score:.2}, {converge} converges)"
        ));
        max_score = max_score.max(churn_score);
    }

    // Isolation score for failure frequency
    let fail_score = isolation_score(fail_vals, fail as f64);
    if fail_score > 0.5 && fail > 1 {
        let fail_rate = fail as f64 / (converge + fail).max(1) as f64;
        reasons.push(format!(
            "high failure rate ({:.0}%, isolation={:.2})",
            fail_rate * 100.0,
            fail_score
        ));
        max_score = max_score.max(fail_score);
    }

    // Dogfood #208 (anomaly-never-detects-anything): the two scores above are
    // POPULATION-RELATIVE. With a single resource under analysis — the normal
    // case for a small config — the population has one member, isolation is ~0,
    // and a resource that failed 3 of 6 applies produced "No anomalies
    // detected". A failure rate is anomalous on its own evidence, so add an
    // absolute detector that does not need peers to compare against.
    if let Some((score, reason)) = absolute_failure_rate(converge, fail) {
        reasons.push(reason);
        max_score = max_score.max(score);
    }

    // Any drift events are always flagged
    if drift > 0 {
        reasons.push(format!("{drift} drift event(s)"));
        max_score = max_score.max(0.7);
    }

    (max_score, reasons)
}

/// Population-independent failure-rate detector.
///
/// Returns `(score, reason)` when at least 3 attempts were made and at least a
/// quarter of them failed. Score is `0.5 + rate/2`, so a 50% failure rate lands
/// in WARNING and a 100% failure rate in DRIFT.
pub fn absolute_failure_rate(converge: u32, fail: u32) -> Option<(f64, String)> {
    let attempts = converge + fail;
    if fail == 0 || attempts < 3 {
        return None;
    }
    let rate = fail as f64 / attempts as f64;
    if rate < 0.25 {
        return None;
    }
    Some((
        0.5 + rate / 2.0,
        format!(
            "failure rate {:.0}% ({fail}/{attempts} applies failed)",
            rate * 100.0
        ),
    ))
}

/// Robust duration-outlier detector (Iglewicz–Hoaglin modified z-score).
///
/// Dogfood #208: a 130x duration outlier went unreported. A plain
/// mean + k·stddev test cannot see it — the outlier itself inflates the
/// stddev — so use median/MAD, which the outlier cannot move.
///
/// Returns `(score, reason)` for the most extreme sample when it exceeds the
/// threshold. Needs at least 4 samples to have an opinion.
pub fn duration_outlier(durations: &[f64]) -> Option<(f64, String)> {
    if durations.len() < 4 {
        return None;
    }
    let median = median_of(durations);
    let deviations: Vec<f64> = durations.iter().map(|d| (d - median).abs()).collect();
    let mad = median_of(&deviations);

    let worst = durations
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |a, b| a.max(b));
    if worst <= median {
        return None;
    }

    let (magnitude, detail) = if mad < 1e-9 {
        // Degenerate spread: fall back to a ratio test against the median.
        if median <= 0.0 || worst < 3.0 * median {
            return None;
        }
        (worst / median, format!("{:.0}x median", worst / median))
    } else {
        let mz = 0.6745 * (worst - median) / mad;
        if mz < 3.5 {
            return None;
        }
        (mz, format!("modified z={mz:.1}"))
    };

    let score = (0.5 + magnitude / 20.0).min(1.0);
    Some((
        score,
        format!("duration outlier {worst:.3}s vs median {median:.3}s ({detail})"),
    ))
}

/// Median of a slice (returns 0.0 for an empty slice).
fn median_of(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

/// Classify an anomaly score into a drift status.
fn classify_score(max_score: f64) -> DriftStatus {
    if max_score > 0.8 {
        DriftStatus::Drift
    } else if max_score > 0.5 {
        DriftStatus::Warning
    } else {
        DriftStatus::Stable
    }
}
