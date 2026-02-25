//! Demonstrate ML-inspired anomaly detection (FJ-051).
//!
//! Shows ADWIN adaptive windowing, isolation scoring, EWMA z-score,
//! and the detect_anomalies bulk analysis function.
//!
//! Usage: cargo run --example anomaly_detection

use forjar::tripwire::anomaly::{self, AdwinDetector, DriftStatus};

fn main() {
    println!("=== Anomaly Detection Example ===\n");

    // ── ADWIN Adaptive Windowing ────────────────────────────────
    println!("--- ADWIN Detector ---\n");

    let mut detector = AdwinDetector::new();

    // Phase 1: stable period (all zeros = no drift)
    for _ in 0..50 {
        detector.add_element(0.0);
    }
    println!("After 50 stable observations: {:?}", detector.status());

    // Phase 2: sudden shift (all ones = drift events)
    for _ in 0..50 {
        detector.add_element(1.0);
    }
    println!("After 50 drift observations:  {:?}", detector.status());

    let stats = detector.stats();
    println!(
        "  n_samples={}, mean={:.2}, std_dev={:.2}\n",
        stats.n_samples, stats.mean, stats.std_dev
    );

    // ── Isolation Scoring ───────────────────────────────────────
    println!("--- Isolation Scoring ---\n");

    // Normal population of converge counts
    let population = vec![5.0, 6.0, 4.0, 5.0, 7.0, 5.0, 6.0, 4.0];

    // Score a normal value
    let normal_score = anomaly::isolation_score(&population, 5.0);
    println!("Score for normal value (5):    {:.3}", normal_score);

    // Score an outlier
    let outlier_score = anomaly::isolation_score(&population, 100.0);
    println!("Score for outlier value (100): {:.3}", outlier_score);

    // Score an extreme outlier
    let extreme_score = anomaly::isolation_score(&population, 1000.0);
    println!("Score for extreme value (1000): {:.3}\n", extreme_score);

    // ── EWMA Z-Score ────────────────────────────────────────────
    println!("--- EWMA Z-Score ---\n");

    let history = vec![1.0, 2.0, 1.5, 2.5, 1.0, 2.0, 1.5];
    let alpha = 0.3; // smoothing factor

    let z_normal = anomaly::ewma_zscore(&history, 1.5, alpha);
    println!("Z-score for target=1.5 (normal): {:.2}", z_normal);

    let z_anomaly = anomaly::ewma_zscore(&history, 20.0, alpha);
    println!("Z-score for target=20.0 (anomaly): {:.2}\n", z_anomaly);

    // ── Bulk Anomaly Detection ──────────────────────────────────
    println!("--- Bulk Anomaly Detection ---\n");

    // Simulate resource metrics: (resource_id, converge, fail, drift)
    let metrics = vec![
        ("web:nginx-config".to_string(), 5, 0, 0),     // normal
        ("web:app-config".to_string(), 5, 0, 0),       // normal
        ("web:ssl-cert".to_string(), 5, 0, 0),         // normal
        ("db:pg-config".to_string(), 5, 0, 0),         // normal
        ("db:backup-cron".to_string(), 5, 0, 0),       // normal
        ("cache:redis-config".to_string(), 200, 0, 0), // high churn!
        ("api:service".to_string(), 3, 30, 0),         // high failure!
        ("web:firewall".to_string(), 10, 0, 3),        // drift events!
    ];

    let findings = anomaly::detect_anomalies(&metrics, 3);

    if findings.is_empty() {
        println!("No anomalies detected.");
    } else {
        println!("{} anomaly(ies) detected:\n", findings.len());
        for finding in &findings {
            let status = match finding.status {
                DriftStatus::Drift => "DRIFT",
                DriftStatus::Warning => "WARNING",
                DriftStatus::Stable => "STABLE",
            };
            println!(
                "  {} [{}] score={:.2}",
                finding.resource, status, finding.score
            );
            for reason in &finding.reasons {
                println!("    - {}", reason);
            }
        }
    }

    // ── ADWIN with custom sensitivity ───────────────────────────
    println!("\n--- Sensitivity Comparison ---\n");

    let mut sensitive = AdwinDetector::with_delta(0.0001); // very sensitive
    let mut relaxed = AdwinDetector::with_delta(0.1); // less sensitive

    // Feed identical data
    for _ in 0..30 {
        sensitive.add_element(0.0);
        relaxed.add_element(0.0);
    }
    for _ in 0..30 {
        sensitive.add_element(0.5); // moderate shift
        relaxed.add_element(0.5);
    }

    println!("Sensitive (delta=0.0001): {:?}", sensitive.status());
    println!("Relaxed   (delta=0.1):   {:?}", relaxed.status());

    println!("\n=== Anomaly Detection Example Complete ===");
}
