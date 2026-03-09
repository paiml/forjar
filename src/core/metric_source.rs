//! FJ-3105: Metric threshold polling event source.
//!
//! Defines metric thresholds and evaluates them against current values
//! to produce MetricThreshold events for the rules engine.

use serde::{Deserialize, Serialize};

/// A metric threshold definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricThreshold {
    /// Metric name (e.g., "cpu_percent", "disk_used_gb").
    pub name: String,
    /// Comparison operator.
    pub operator: ThresholdOp,
    /// Threshold value.
    pub value: f64,
    /// How many consecutive violations before firing.
    #[serde(default = "default_consecutive")]
    pub consecutive: u32,
}

fn default_consecutive() -> u32 {
    1
}

/// Threshold comparison operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdOp {
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Gte,
    /// Less than.
    Lt,
    /// Less than or equal.
    Lte,
    /// Equal (within epsilon).
    Eq,
}

impl std::fmt::Display for ThresholdOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gt => write!(f, ">"),
            Self::Gte => write!(f, ">="),
            Self::Lt => write!(f, "<"),
            Self::Lte => write!(f, "<="),
            Self::Eq => write!(f, "=="),
        }
    }
}

/// Evaluate a metric value against a threshold.
pub fn evaluate_threshold(threshold: &MetricThreshold, value: f64) -> bool {
    match threshold.operator {
        ThresholdOp::Gt => value > threshold.value,
        ThresholdOp::Gte => value >= threshold.value,
        ThresholdOp::Lt => value < threshold.value,
        ThresholdOp::Lte => value <= threshold.value,
        ThresholdOp::Eq => (value - threshold.value).abs() < f64::EPSILON,
    }
}

/// Tracker for consecutive threshold violations.
#[derive(Debug, Clone, Default)]
pub struct ThresholdTracker {
    counts: std::collections::HashMap<String, u32>,
}

impl ThresholdTracker {
    /// Record a threshold evaluation result. Returns true if the
    /// consecutive violation count has been reached.
    pub fn record(&mut self, name: &str, violated: bool, required: u32) -> bool {
        if violated {
            let count = self.counts.entry(name.to_string()).or_insert(0);
            *count += 1;
            *count >= required
        } else {
            self.counts.remove(name);
            false
        }
    }

    /// Get the current consecutive violation count for a metric.
    pub fn count(&self, name: &str) -> u32 {
        self.counts.get(name).copied().unwrap_or(0)
    }

    /// Reset all counters.
    pub fn reset(&mut self) {
        self.counts.clear();
    }
}

/// Result of evaluating a set of metric thresholds.
#[derive(Debug, Clone)]
pub struct MetricEvalResult {
    /// Metric name.
    pub name: String,
    /// Current value.
    pub current: f64,
    /// Threshold value.
    pub threshold: f64,
    /// Operator.
    pub operator: ThresholdOp,
    /// Whether the threshold was violated.
    pub violated: bool,
    /// Whether the consecutive count was reached (should fire).
    pub should_fire: bool,
}

/// Evaluate multiple metric thresholds against current values.
pub fn evaluate_metrics(
    thresholds: &[MetricThreshold],
    values: &std::collections::HashMap<String, f64>,
    tracker: &mut ThresholdTracker,
) -> Vec<MetricEvalResult> {
    thresholds
        .iter()
        .filter_map(|t| {
            values.get(&t.name).map(|&current| {
                let violated = evaluate_threshold(t, current);
                let should_fire = tracker.record(&t.name, violated, t.consecutive);
                MetricEvalResult {
                    name: t.name.clone(),
                    current,
                    threshold: t.value,
                    operator: t.operator.clone(),
                    violated,
                    should_fire,
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn threshold(name: &str, op: ThresholdOp, value: f64) -> MetricThreshold {
        MetricThreshold {
            name: name.into(),
            operator: op,
            value,
            consecutive: 1,
        }
    }

    #[test]
    fn gt_threshold() {
        let t = threshold("cpu", ThresholdOp::Gt, 80.0);
        assert!(evaluate_threshold(&t, 81.0));
        assert!(!evaluate_threshold(&t, 80.0));
        assert!(!evaluate_threshold(&t, 79.0));
    }

    #[test]
    fn gte_threshold() {
        let t = threshold("cpu", ThresholdOp::Gte, 80.0);
        assert!(evaluate_threshold(&t, 80.0));
        assert!(evaluate_threshold(&t, 81.0));
        assert!(!evaluate_threshold(&t, 79.0));
    }

    #[test]
    fn lt_threshold() {
        let t = threshold("disk_free", ThresholdOp::Lt, 10.0);
        assert!(evaluate_threshold(&t, 5.0));
        assert!(!evaluate_threshold(&t, 10.0));
        assert!(!evaluate_threshold(&t, 15.0));
    }

    #[test]
    fn lte_threshold() {
        let t = threshold("disk_free", ThresholdOp::Lte, 10.0);
        assert!(evaluate_threshold(&t, 10.0));
        assert!(evaluate_threshold(&t, 5.0));
        assert!(!evaluate_threshold(&t, 15.0));
    }

    #[test]
    fn eq_threshold() {
        let t = threshold("replicas", ThresholdOp::Eq, 3.0);
        assert!(evaluate_threshold(&t, 3.0));
        assert!(!evaluate_threshold(&t, 4.0));
    }

    #[test]
    fn tracker_single_violation() {
        let mut tracker = ThresholdTracker::default();
        assert!(tracker.record("cpu", true, 1));
    }

    #[test]
    fn tracker_consecutive_violations() {
        let mut tracker = ThresholdTracker::default();
        assert!(!tracker.record("cpu", true, 3));
        assert!(!tracker.record("cpu", true, 3));
        assert!(tracker.record("cpu", true, 3));
    }

    #[test]
    fn tracker_reset_on_ok() {
        let mut tracker = ThresholdTracker::default();
        tracker.record("cpu", true, 3);
        tracker.record("cpu", true, 3);
        // OK resets the counter
        tracker.record("cpu", false, 3);
        assert_eq!(tracker.count("cpu"), 0);
        // Must start over
        assert!(!tracker.record("cpu", true, 3));
    }

    #[test]
    fn tracker_count() {
        let mut tracker = ThresholdTracker::default();
        assert_eq!(tracker.count("cpu"), 0);
        tracker.record("cpu", true, 5);
        assert_eq!(tracker.count("cpu"), 1);
    }

    #[test]
    fn tracker_reset_all() {
        let mut tracker = ThresholdTracker::default();
        tracker.record("cpu", true, 5);
        tracker.record("mem", true, 5);
        tracker.reset();
        assert_eq!(tracker.count("cpu"), 0);
        assert_eq!(tracker.count("mem"), 0);
    }

    #[test]
    fn evaluate_multiple_metrics() {
        let thresholds = vec![
            threshold("cpu", ThresholdOp::Gt, 80.0),
            threshold("mem", ThresholdOp::Gt, 90.0),
            threshold("disk", ThresholdOp::Lt, 10.0),
        ];
        let mut values = std::collections::HashMap::new();
        values.insert("cpu".into(), 85.0);
        values.insert("mem".into(), 70.0);
        values.insert("disk".into(), 5.0);

        let mut tracker = ThresholdTracker::default();
        let results = evaluate_metrics(&thresholds, &values, &mut tracker);

        assert_eq!(results.len(), 3);
        assert!(results[0].violated); // cpu 85 > 80
        assert!(!results[1].violated); // mem 70 < 90
        assert!(results[2].violated); // disk 5 < 10
    }

    #[test]
    fn evaluate_missing_metric() {
        let thresholds = vec![threshold("missing", ThresholdOp::Gt, 50.0)];
        let values = std::collections::HashMap::new();
        let mut tracker = ThresholdTracker::default();
        let results = evaluate_metrics(&thresholds, &values, &mut tracker);
        assert!(results.is_empty());
    }

    #[test]
    fn threshold_op_display() {
        assert_eq!(ThresholdOp::Gt.to_string(), ">");
        assert_eq!(ThresholdOp::Gte.to_string(), ">=");
        assert_eq!(ThresholdOp::Lt.to_string(), "<");
        assert_eq!(ThresholdOp::Lte.to_string(), "<=");
        assert_eq!(ThresholdOp::Eq.to_string(), "==");
    }

    #[test]
    fn threshold_serde_roundtrip() {
        let t = MetricThreshold {
            name: "cpu_percent".into(),
            operator: ThresholdOp::Gt,
            value: 80.0,
            consecutive: 3,
        };
        let json = serde_json::to_string(&t).unwrap();
        let parsed: MetricThreshold = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "cpu_percent");
        assert_eq!(parsed.operator, ThresholdOp::Gt);
        assert_eq!(parsed.value, 80.0);
        assert_eq!(parsed.consecutive, 3);
    }
}
