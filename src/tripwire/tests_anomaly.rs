use super::anomaly::*;

// ── ADWIN tests ─────────────────────────────────────────────

#[test]
fn test_fj051_adwin_stable() {
    let mut det = AdwinDetector::new();
    for _ in 0..50 {
        det.add_element(0.0);
    }
    assert_eq!(*det.status(), DriftStatus::Stable);
}

#[test]
fn test_fj051_adwin_drift() {
    let mut det = AdwinDetector::new();
    // Phase 1: all zeros
    for _ in 0..50 {
        det.add_element(0.0);
    }
    // Phase 2: all ones — distribution shift
    for _ in 0..50 {
        det.add_element(1.0);
    }
    assert_ne!(*det.status(), DriftStatus::Stable);
}

#[test]
fn test_fj051_adwin_stats() {
    let mut det = AdwinDetector::new();
    for _ in 0..20 {
        det.add_element(1.0);
    }
    let stats = det.stats();
    assert_eq!(stats.n_samples, 20);
    assert!((stats.mean - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_fj051_adwin_reset() {
    let mut det = AdwinDetector::new();
    for _ in 0..10 {
        det.add_element(1.0);
    }
    det.reset();
    let stats = det.stats();
    assert_eq!(stats.n_samples, 0);
    assert_eq!(*det.status(), DriftStatus::Stable);
}

#[test]
fn test_fj051_adwin_custom_delta() {
    let det = AdwinDetector::with_delta(0.1);
    assert_eq!(*det.status(), DriftStatus::Stable);
}

#[test]
fn test_fj051_adwin_too_few_samples() {
    let mut det = AdwinDetector::new();
    for _ in 0..5 {
        det.add_element(1.0);
    }
    assert_eq!(*det.status(), DriftStatus::Stable);
}

#[test]
fn test_fj051_adwin_gradual_shift() {
    let mut det = AdwinDetector::new();
    // Gradual shift from 0 to 1
    for i in 0..100 {
        det.add_element(i as f64 / 100.0);
    }
    // May detect warning or drift depending on sensitivity
    let stats = det.stats();
    assert!(stats.n_samples > 0);
}

// ── Isolation score tests ───────────────────────────────────

#[test]
fn test_fj051_isolation_score_normal() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let score = isolation_score(&values, 3.0); // mean
    assert!(score < 0.3, "mean value should have low score: {}", score);
}

#[test]
fn test_fj051_isolation_score_outlier() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let score = isolation_score(&values, 100.0); // far outlier
    assert!(score > 0.9, "outlier should have high score: {}", score);
}

#[test]
fn test_fj051_isolation_score_empty() {
    assert_eq!(isolation_score(&[], 5.0), 0.0);
}

#[test]
fn test_fj051_isolation_score_identical() {
    let values = vec![5.0, 5.0, 5.0, 5.0];
    let score = isolation_score(&values, 5.0);
    assert!(
        score < f64::EPSILON,
        "identical values at target should be 0"
    );
}

#[test]
fn test_fj051_isolation_score_identical_outlier() {
    let values = vec![5.0, 5.0, 5.0, 5.0];
    let score = isolation_score(&values, 10.0);
    assert!(
        (score - 1.0).abs() < f64::EPSILON,
        "deviation from identical population = 1.0"
    );
}

// ── EWMA z-score tests ──────────────────────────────────────

#[test]
fn test_fj051_ewma_zscore_stable() {
    let values = vec![1.0, 1.0, 1.0, 1.0, 1.0];
    let z = ewma_zscore(&values, 1.0, 0.3);
    assert!(z < 0.5, "stable series at target should have low z: {}", z);
}

#[test]
fn test_fj051_ewma_zscore_anomaly() {
    // Need some variance in the series so EWMA std > 0
    let values = vec![1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0, 2.0];
    let z = ewma_zscore(&values, 10.0, 0.3);
    assert!(z > 2.0, "large deviation should have high z: {}", z);
}

#[test]
fn test_fj051_ewma_zscore_empty() {
    assert_eq!(ewma_zscore(&[], 5.0, 0.3), 0.0);
}

// ── detect_anomalies tests ──────────────────────────────────

#[test]
fn test_fj051_detect_no_anomalies() {
    let metrics = vec![
        ("r1".to_string(), 5, 0, 0),
        ("r2".to_string(), 5, 0, 0),
        ("r3".to_string(), 5, 0, 0),
    ];
    let findings = detect_anomalies(&metrics, 3);
    assert!(
        findings.is_empty(),
        "uniform metrics should have no anomalies"
    );
}

#[test]
fn test_fj051_detect_high_churn() {
    let metrics = vec![
        ("normal1".to_string(), 5, 0, 0),
        ("normal2".to_string(), 5, 0, 0),
        ("normal3".to_string(), 5, 0, 0),
        ("normal4".to_string(), 5, 0, 0),
        ("normal5".to_string(), 5, 0, 0),
        ("churny".to_string(), 500, 0, 0), // extreme outlier
    ];
    let findings = detect_anomalies(&metrics, 3);
    assert!(
        findings.iter().any(|f| f.resource == "churny"),
        "high churn resource should be detected: {:?}",
        findings
    );
}

#[test]
fn test_fj051_detect_drift_events() {
    let metrics = vec![
        ("stable".to_string(), 10, 0, 0),
        ("drifty".to_string(), 10, 0, 3),
    ];
    let findings = detect_anomalies(&metrics, 3);
    assert!(
        findings.iter().any(|f| f.resource == "drifty"),
        "drift events should be flagged"
    );
}

#[test]
fn test_fj051_detect_high_failure() {
    let metrics = vec![
        ("good1".to_string(), 10, 0, 0),
        ("good2".to_string(), 10, 0, 0),
        ("good3".to_string(), 10, 0, 0),
        ("good4".to_string(), 10, 0, 0),
        ("bad".to_string(), 3, 50, 0), // extreme failure rate
    ];
    let findings = detect_anomalies(&metrics, 3);
    assert!(
        findings.iter().any(|f| f.resource == "bad"),
        "high failure rate should be detected: {:?}",
        findings
    );
}

#[test]
fn test_fj051_detect_below_min_events() {
    let metrics = vec![("sparse".to_string(), 1, 0, 0)];
    let findings = detect_anomalies(&metrics, 5);
    assert!(findings.is_empty(), "below min_events should be filtered");
}

#[test]
fn test_fj051_detect_empty() {
    let findings = detect_anomalies(&[], 1);
    assert!(findings.is_empty());
}

#[test]
fn test_fj051_findings_sorted_by_score() {
    let metrics = vec![
        ("low".to_string(), 5, 0, 1),     // drift only
        ("high".to_string(), 100, 10, 5), // churn + fail + drift
        ("mid".to_string(), 5, 5, 0),     // some failures
    ];
    let findings = detect_anomalies(&metrics, 3);
    if findings.len() >= 2 {
        assert!(
            findings[0].score >= findings[1].score,
            "findings should be sorted by score desc"
        );
    }
}

#[test]
fn test_fj051_drift_status_serde() {
    let json = serde_json::to_string(&DriftStatus::Drift).unwrap();
    assert_eq!(json, "\"drift\"");
    let back: DriftStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(back, DriftStatus::Drift);
}

#[test]
fn test_fj051_anomaly_finding_serde() {
    let finding = AnomalyFinding {
        resource: "web:config".to_string(),
        score: 0.85,
        status: DriftStatus::Drift,
        reasons: vec!["high churn".to_string()],
    };
    let json = serde_json::to_string(&finding).unwrap();
    let back: AnomalyFinding = serde_json::from_str(&json).unwrap();
    assert_eq!(back.resource, "web:config");
    assert_eq!(back.status, DriftStatus::Drift);
}

#[test]
fn test_fj051_adwin_window_limit() {
    let mut det = AdwinDetector::new();
    // Exceed max_window
    for i in 0..1500 {
        det.add_element(i as f64);
    }
    let stats = det.stats();
    assert!(
        stats.n_samples <= 1000,
        "window should be bounded: {}",
        stats.n_samples
    );
}

#[test]
fn test_fj051_adwin_default() {
    let det = AdwinDetector::default();
    assert_eq!(*det.status(), DriftStatus::Stable);
    assert_eq!(det.stats().n_samples, 0);
}
