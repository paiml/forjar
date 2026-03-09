//! FJ-014/051: Tripwire hasher and anomaly detection falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-014: BLAKE3 state hashing
//!   - hash_string: determinism, blake3 prefix, length invariant
//!   - hash_file: file content hashing with temp files
//!   - hash_directory: recursive directory hashing
//!   - composite_hash: multi-component hash combination
//! - FJ-051: Anomaly detection
//!   - isolation_score: rank-based anomaly scoring
//!   - ewma_zscore: exponentially weighted z-score
//!   - detect_anomalies: full anomaly pipeline
//!   - AdwinDetector: adaptive windowing drift detection
//!
//! Usage: cargo test --test falsification_tripwire_hasher_anomaly

use forjar::tripwire::anomaly::{
    detect_anomalies, ewma_zscore, isolation_score, AdwinDetector, DriftStatus,
};
use forjar::tripwire::hasher::{composite_hash, hash_directory, hash_file, hash_string};

// ============================================================================
// FJ-014: hash_string
// ============================================================================

#[test]
fn hash_string_deterministic() {
    let h1 = hash_string("hello world");
    let h2 = hash_string("hello world");
    assert_eq!(h1, h2);
}

#[test]
fn hash_string_blake3_prefix() {
    let h = hash_string("test");
    assert!(h.starts_with("blake3:"), "hash must start with blake3:");
}

#[test]
fn hash_string_length_71() {
    // "blake3:" (7 chars) + 64 hex chars = 71
    let h = hash_string("anything");
    assert_eq!(h.len(), 71, "blake3 hash string must be 71 chars");
}

#[test]
fn hash_string_different_inputs_different_hashes() {
    let h1 = hash_string("alpha");
    let h2 = hash_string("beta");
    assert_ne!(h1, h2);
}

#[test]
fn hash_string_empty_produces_valid_hash() {
    let h = hash_string("");
    assert!(h.starts_with("blake3:"));
    assert_eq!(h.len(), 71);
}

// ============================================================================
// FJ-014: hash_file
// ============================================================================

#[test]
fn hash_file_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    std::fs::write(&path, "file content").unwrap();

    let h1 = hash_file(&path).unwrap();
    let h2 = hash_file(&path).unwrap();
    assert_eq!(h1, h2);
    assert!(h1.starts_with("blake3:"));
}

#[test]
fn hash_file_matches_string_hash() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("match.txt");
    let content = "matching content";
    std::fs::write(&path, content).unwrap();

    let file_hash = hash_file(&path).unwrap();
    let string_hash = hash_string(content);
    assert_eq!(
        file_hash, string_hash,
        "file hash should match string hash of same content"
    );
}

#[test]
fn hash_file_different_content_different_hash() {
    let dir = tempfile::tempdir().unwrap();
    let p1 = dir.path().join("a.txt");
    let p2 = dir.path().join("b.txt");
    std::fs::write(&p1, "aaa").unwrap();
    std::fs::write(&p2, "bbb").unwrap();

    assert_ne!(hash_file(&p1).unwrap(), hash_file(&p2).unwrap());
}

#[test]
fn hash_file_missing_returns_error() {
    let result = hash_file(std::path::Path::new("/nonexistent/file.txt"));
    assert!(result.is_err());
}

// ============================================================================
// FJ-014: hash_directory
// ============================================================================

#[test]
fn hash_directory_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
    std::fs::write(dir.path().join("b.txt"), "world").unwrap();

    let h1 = hash_directory(dir.path()).unwrap();
    let h2 = hash_directory(dir.path()).unwrap();
    assert_eq!(h1, h2);
    assert!(h1.starts_with("blake3:"));
}

#[test]
fn hash_directory_different_content_different_hash() {
    let d1 = tempfile::tempdir().unwrap();
    let d2 = tempfile::tempdir().unwrap();
    std::fs::write(d1.path().join("file.txt"), "version-1").unwrap();
    std::fs::write(d2.path().join("file.txt"), "version-2").unwrap();

    assert_ne!(
        hash_directory(d1.path()).unwrap(),
        hash_directory(d2.path()).unwrap()
    );
}

#[test]
fn hash_directory_includes_subdirs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub/nested.txt"), "nested").unwrap();

    let h_with = hash_directory(dir.path()).unwrap();

    // Remove nested file and check hash differs
    std::fs::remove_file(dir.path().join("sub/nested.txt")).unwrap();
    std::fs::remove_dir(dir.path().join("sub")).unwrap();

    let h_without = hash_directory(dir.path()).unwrap();
    assert_ne!(h_with, h_without);
}

#[test]
fn hash_directory_empty_valid() {
    let dir = tempfile::tempdir().unwrap();
    let h = hash_directory(dir.path()).unwrap();
    assert!(h.starts_with("blake3:"));
}

// ============================================================================
// FJ-014: composite_hash
// ============================================================================

#[test]
fn composite_hash_deterministic() {
    let h1 = composite_hash(&["a", "b", "c"]);
    let h2 = composite_hash(&["a", "b", "c"]);
    assert_eq!(h1, h2);
}

#[test]
fn composite_hash_order_matters() {
    let h1 = composite_hash(&["a", "b"]);
    let h2 = composite_hash(&["b", "a"]);
    assert_ne!(h1, h2, "component order must affect hash");
}

#[test]
fn composite_hash_different_components_different_hash() {
    let h1 = composite_hash(&["x"]);
    let h2 = composite_hash(&["y"]);
    assert_ne!(h1, h2);
}

#[test]
fn composite_hash_empty_valid() {
    let h = composite_hash(&[]);
    assert!(h.starts_with("blake3:"));
    assert_eq!(h.len(), 71);
}

#[test]
fn composite_hash_single_component() {
    let h = composite_hash(&["only"]);
    assert!(h.starts_with("blake3:"));
    // Should differ from hash_string("only") because composite adds null separator
    assert_ne!(h, hash_string("only"));
}

// ============================================================================
// FJ-051: isolation_score
// ============================================================================

#[test]
fn isolation_score_empty_returns_zero() {
    assert_eq!(isolation_score(&[], 5.0), 0.0);
}

#[test]
fn isolation_score_target_matches_population_low() {
    let values = vec![10.0, 10.0, 10.0, 10.0, 10.0];
    let score = isolation_score(&values, 10.0);
    assert!(
        score < 0.1,
        "matching target should have low score, got {score}"
    );
}

#[test]
fn isolation_score_outlier_high() {
    let values = vec![1.0, 1.0, 1.0, 1.0, 1.0, 100.0];
    let score = isolation_score(&values, 100.0);
    assert!(score > 0.5, "outlier should have high score, got {score}");
}

#[test]
fn isolation_score_range_0_to_1() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    for target in [0.0, 3.0, 10.0, 100.0] {
        let score = isolation_score(&values, target);
        assert!(
            (0.0..=1.0).contains(&score),
            "score must be in [0,1], got {score} for target {target}"
        );
    }
}

#[test]
fn isolation_score_single_value_match() {
    let score = isolation_score(&[42.0], 42.0);
    assert!(
        score < 0.1,
        "single matching value should be low, got {score}"
    );
}

// ============================================================================
// FJ-051: ewma_zscore
// ============================================================================

#[test]
fn ewma_zscore_empty_returns_zero() {
    assert_eq!(ewma_zscore(&[], 5.0, 0.3), 0.0);
}

#[test]
fn ewma_zscore_constant_series_low() {
    let values = vec![10.0; 20];
    let z = ewma_zscore(&values, 10.0, 0.3);
    assert!(
        z < 1.0,
        "constant series with matching target should have low z, got {z}"
    );
}

#[test]
fn ewma_zscore_outlier_high() {
    // Need variance in the series for a meaningful z-score
    let values: Vec<f64> = (0..20).map(|i| 10.0 + (i as f64 * 0.1)).collect();
    let z = ewma_zscore(&values, 50.0, 0.3);
    assert!(z > 1.0, "outlier target should have high z-score, got {z}");
}

#[test]
fn ewma_zscore_alpha_sensitivity() {
    let mut values = vec![1.0; 10];
    values.extend(vec![5.0; 10]); // shift
                                  // Higher alpha = more recent bias
    let z_low = ewma_zscore(&values, 1.0, 0.1);
    let z_high = ewma_zscore(&values, 1.0, 0.9);
    // Both should be positive since 1.0 is far from the recent 5.0 trend
    assert!(z_low > 0.0 && z_high > 0.0);
}

// ============================================================================
// FJ-051: detect_anomalies
// ============================================================================

#[test]
fn detect_anomalies_empty_input() {
    let result = detect_anomalies(&[], 1);
    assert!(result.is_empty());
}

#[test]
fn detect_anomalies_below_min_events_filtered() {
    let metrics = vec![("res-1".into(), 1, 0, 0)]; // only 1 event
    let result = detect_anomalies(&metrics, 5);
    assert!(result.is_empty(), "below min_events should be filtered");
}

#[test]
fn detect_anomalies_drift_events_flagged() {
    let metrics = vec![("stable".into(), 10, 0, 0), ("drifting".into(), 10, 0, 3)];
    let result = detect_anomalies(&metrics, 1);
    assert!(!result.is_empty());
    let drifting = result.iter().find(|f| f.resource == "drifting");
    assert!(
        drifting.is_some(),
        "resource with drift events should be flagged"
    );
    assert!(drifting.unwrap().score >= 0.7);
}

#[test]
fn detect_anomalies_high_failure_rate_flagged() {
    // Need enough resources to make isolation scoring meaningful
    let metrics = vec![
        ("healthy-1".into(), 100, 0, 0),
        ("healthy-2".into(), 90, 1, 0),
        ("healthy-3".into(), 95, 0, 0),
        ("failing".into(), 2, 50, 0),
    ];
    let result = detect_anomalies(&metrics, 1);
    let failing = result.iter().find(|f| f.resource == "failing");
    assert!(failing.is_some(), "high failure resource should be flagged");
}

#[test]
fn detect_anomalies_sorted_by_score_descending() {
    let metrics = vec![
        ("low".into(), 10, 1, 0),
        ("high".into(), 1, 50, 5),
        ("mid".into(), 5, 10, 1),
    ];
    let result = detect_anomalies(&metrics, 1);
    for w in result.windows(2) {
        assert!(
            w[0].score >= w[1].score,
            "results should be sorted descending"
        );
    }
}

// ============================================================================
// FJ-051: AdwinDetector
// ============================================================================

#[test]
fn adwin_new_stable() {
    let detector = AdwinDetector::new();
    assert_eq!(*detector.status(), DriftStatus::Stable);
}

#[test]
fn adwin_stable_stream_stays_stable() {
    let mut detector = AdwinDetector::new();
    for _ in 0..50 {
        detector.add_element(0.0); // all normal
    }
    assert_eq!(*detector.status(), DriftStatus::Stable);
}

#[test]
fn adwin_drift_detected_on_shift() {
    let mut detector = AdwinDetector::with_delta(0.01);
    // Phase 1: all zeros (stable)
    for _ in 0..50 {
        detector.add_element(0.0);
    }
    // Phase 2: all ones (shift)
    for _ in 0..50 {
        detector.add_element(1.0);
    }
    let status = detector.status();
    assert!(
        *status == DriftStatus::Drift || *status == DriftStatus::Warning,
        "distribution shift should be detected, got {status:?}"
    );
}

#[test]
fn adwin_stats_valid() {
    let mut detector = AdwinDetector::new();
    for i in 0..20 {
        detector.add_element(i as f64);
    }
    let stats = detector.stats();
    assert_eq!(stats.n_samples, 20);
    assert!(stats.mean > 0.0);
    assert!(stats.std_dev > 0.0);
}

#[test]
fn adwin_reset_clears_state() {
    let mut detector = AdwinDetector::new();
    for _ in 0..50 {
        detector.add_element(1.0);
    }
    detector.reset();
    let stats = detector.stats();
    assert_eq!(stats.n_samples, 0);
    assert_eq!(*detector.status(), DriftStatus::Stable);
}

#[test]
fn adwin_default_same_as_new() {
    let d1 = AdwinDetector::new();
    let d2 = AdwinDetector::default();
    assert_eq!(*d1.status(), *d2.status());
    assert_eq!(d1.stats().n_samples, d2.stats().n_samples);
}

#[test]
fn adwin_few_samples_always_stable() {
    let mut detector = AdwinDetector::new();
    // Less than 10 samples should always be stable
    for _ in 0..5 {
        detector.add_element(1.0);
    }
    for _ in 0..4 {
        detector.add_element(0.0);
    }
    assert_eq!(*detector.status(), DriftStatus::Stable);
}
