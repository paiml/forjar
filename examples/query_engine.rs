//! FJ-2001: Query engine types — results, health summaries, timing stats.
//!
//! ```bash
//! cargo run --example query_engine
//! ```

use forjar::core::types::{
    ChurnMetric, HealthSummary, MachineHealthRow, QueryOutputFormat, QueryParams, QueryResult,
    TimingStats,
};

fn main() {
    // Simulate FTS5 search results
    let results = vec![
        QueryResult {
            resource_id: "bash-aliases".into(),
            machine: "intel".into(),
            resource_type: "file".into(),
            status: "converged".into(),
            generation: 12,
            applied_at: "2026-02-16T16:32:55Z".into(),
            duration_secs: 0.54,
            state_hash: Some("blake3:a1b2c3d4".into()),
            path: Some("/home/user/.bash_aliases".into()),
            fts_rank: Some(-2.5),
        },
        QueryResult {
            resource_id: "bashrc-hook".into(),
            machine: "intel".into(),
            resource_type: "file".into(),
            status: "converged".into(),
            generation: 12,
            applied_at: "2026-02-16T16:40:20Z".into(),
            duration_secs: 0.27,
            state_hash: Some("blake3:e5f6a7b8".into()),
            path: Some("/home/user/.bashrc.d/forjar.sh".into()),
            fts_rank: Some(-1.8),
        },
    ];

    println!("=== forjar query \"bash\" ===");
    println!();
    println!(
        " {:<16} {:<8} {:<8} {:<10} {:>4}  {:<22} {:>8}",
        "RESOURCE", "MACHINE", "TYPE", "STATUS", "GEN", "APPLIED", "DURATION"
    );
    for r in &results {
        println!(
            " {:<16} {:<8} {:<8} {:<10} {:>4}  {:<22} {:>7.2}s",
            r.resource_id,
            r.machine,
            r.resource_type,
            r.status,
            r.generation,
            r.applied_at,
            r.duration_secs,
        );
    }
    println!();

    // Timing enrichment
    let durations: Vec<f64> = vec![0.12, 0.18, 0.27, 0.32, 0.41, 0.54, 0.67, 0.89, 1.2, 2.1];
    if let Some(stats) = TimingStats::from_sorted(&durations) {
        println!("Timing: {}", stats.format_compact());
    }
    println!();

    // Churn enrichment
    let churn = ChurnMetric {
        resource_id: "bash-aliases".into(),
        changed_gens: 2,
        total_gens: 12,
    };
    println!(
        "Churn: {}/{} gens ({:.0}%)",
        churn.changed_gens,
        churn.total_gens,
        churn.churn_pct()
    );
    println!();

    // Health summary
    let health = HealthSummary {
        machines: vec![
            MachineHealthRow {
                name: "intel".into(),
                total: 17,
                converged: 17,
                drifted: 0,
                failed: 0,
                last_apply: "2026-02-16T16:44:39Z".into(),
                generation: 12,
            },
            MachineHealthRow {
                name: "jetson".into(),
                total: 8,
                converged: 7,
                drifted: 1,
                failed: 0,
                last_apply: "2026-03-01T09:12:00Z".into(),
                generation: 5,
            },
            MachineHealthRow {
                name: "lambda".into(),
                total: 7,
                converged: 7,
                drifted: 0,
                failed: 0,
                last_apply: "2026-03-03T13:44:15Z".into(),
                generation: 3,
            },
        ],
    };

    println!("=== forjar query --health ===");
    println!();
    print!("{}", health.format_table());
    println!();

    // Query params
    let params = QueryParams {
        keywords: Some("bash".into()),
        machine: Some("intel".into()),
        format: QueryOutputFormat::Table,
        history: true,
        timing: true,
        ..Default::default()
    };
    println!("Query params: {:?}", params);

    // JSON output mode
    println!();
    println!("=== JSON output ===");
    println!("{}", serde_json::to_string_pretty(&results).unwrap());
}
