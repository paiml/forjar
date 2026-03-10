//! FJ-2001: State ingest pipeline and query engine.
//!
//! Usage: cargo run --example ingest_query

use forjar::core::store::db;
use forjar::core::store::ingest::ingest_state_dir;
use forjar::core::store::query::{
    query_churn, query_drift, query_events, query_failures, query_health,
};
use std::path::Path;

fn main() {
    println!("Forjar: State Ingest & Query Engine");
    println!("{}", "=".repeat(45));

    // Create test state directory
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = tmp.path().join("state");

    // Machine: web-01
    let web01 = state_dir.join("web-01");
    std::fs::create_dir_all(&web01).unwrap();
    std::fs::write(
        web01.join("state.lock.yaml"),
        r#"hostname: web-01.prod
generated_at: "2026-03-08T12:00:00Z"
resources:
  nginx-pkg:
    type: package
    status: converged
    applied_at: "2026-03-08T12:00:00Z"
    hash: blake3:aabbcc
    duration_seconds: 2.5
    details:
      content_hash: blake3:aabbcc
      live_hash: blake3:aabbcc
  app-config:
    type: file
    status: drifted
    applied_at: "2026-03-08T12:01:00Z"
    hash: blake3:ddeeff
    duration_seconds: 0.3
    details:
      path: /etc/app/config.yaml
      content_hash: blake3:ddeeff
      live_hash: blake3:112233
"#,
    )
    .unwrap();

    std::fs::write(
        web01.join("events.jsonl"),
        r#"{"run_id":"run-001","event":"resource_converged","resource":"nginx-pkg","ts":"2026-03-08T12:00:00Z","duration_seconds":2.5}
{"run_id":"run-001","event":"resource_converged","resource":"app-config","ts":"2026-03-08T12:01:00Z","duration_seconds":0.3}
{"run_id":"run-002","event":"resource_failed","resource":"ssl-cert","ts":"2026-03-08T13:00:00Z","duration_seconds":0.0}
"#,
    )
    .unwrap();

    // ── Ingest ──
    println!("\n[Ingest]");
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    let result = ingest_state_dir(&conn, &state_dir).unwrap();
    println!("  {result}");

    // ── Health ──
    println!("\n[Health]");
    let health = query_health(&conn).unwrap();
    for m in &health.machines {
        println!(
            "  {}: {} resources ({} converged, {} drifted, {} failed)",
            m.name, m.resources, m.converged, m.drifted, m.failed
        );
    }
    println!(
        "  Overall: {:.1}% ({}/{})",
        health.health_pct(),
        health.total_converged,
        health.total_resources
    );

    // ── Drift ──
    println!("\n[Drift]");
    let drift = query_drift(&conn).unwrap();
    if drift.is_empty() {
        println!("  No drift detected.");
    } else {
        for d in &drift {
            println!(
                "  {} on {}: {} → {}",
                d.resource_id, d.machine, d.content_hash, d.live_hash
            );
        }
    }

    // ── Events ──
    println!("\n[Events]");
    let events = query_events(&conn, None, None, 10).unwrap();
    for ev in &events {
        println!(
            "  [{}] {} ({}ms)",
            ev.event_type,
            ev.run_id,
            ev.duration_ms.unwrap_or(0)
        );
    }

    // ── Failures ──
    println!("\n[Failures]");
    let failures = query_failures(&conn, None, 10).unwrap();
    if failures.is_empty() {
        println!("  No failures.");
    } else {
        for f in &failures {
            println!(
                "  {} on {} (run {}): exit {}",
                f.resource_id,
                f.machine,
                f.run_id,
                f.exit_code.unwrap_or(-1)
            );
        }
    }

    // ── Churn ──
    println!("\n[Churn]");
    let churn = query_churn(&conn).unwrap();
    for c in &churn {
        println!(
            "  {}: {} events across {} runs",
            c.resource_id, c.event_count, c.distinct_runs
        );
    }

    println!("\n{}", "=".repeat(45));
    println!("All ingest/query criteria survived.");
}
