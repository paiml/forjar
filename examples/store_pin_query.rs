//! FJ-1364/2001: Pin resolution parsing and query engine.
//!
//! Demonstrates:
//! - Pin resolution command generation for multiple providers
//! - Version output parsing (apt, cargo, pip, nix, docker)
//! - Deterministic pin hashing
//! - State database queries (health, drift, events)
//!
//! Usage: cargo run --example store_pin_query

use forjar::core::store::pin_resolve::{parse_resolved_version, pin_hash, resolution_command};
use forjar::core::store::query::{query_drift, query_events, query_health};

fn main() {
    println!("Forjar Store: Pin Resolution & Query Engine");
    println!("{}", "=".repeat(50));

    // ── FJ-1364: Pin Resolution ──
    println!("\n[FJ-1364] Pin Resolution Commands:");
    for (provider, pkg) in [
        ("apt", "nginx"),
        ("cargo", "serde"),
        ("nix", "hello"),
        ("pip", "requests"),
        ("docker", "nginx"),
        ("apr", "llama-3.1"),
    ] {
        if let Some(cmd) = resolution_command(provider, pkg) {
            println!("  {provider}/{pkg}: {cmd}");
        }
    }

    println!("\n[FJ-1364] Version Parsing:");
    let apt_out = "nginx:\n  Installed: (none)\n  Candidate: 1.24.0-2\n  Version table:";
    let cargo_out = r#"serde = "1.0.215"    # A serialization framework"#;
    let pip_out = "Available versions: 2.31.0, 2.30.0";

    for (provider, output, expected) in [
        ("apt", apt_out, "1.24.0-2"),
        ("cargo", cargo_out, "1.0.215"),
        ("pip", pip_out, "2.31.0"),
        ("nix", "23.11", "23.11"),
    ] {
        let version = parse_resolved_version(provider, output).unwrap();
        let ok = version == expected;
        println!(
            "  {provider}: {version} {}",
            if ok { "✓" } else { "✗ FALSIFIED" }
        );
        assert!(ok);
    }

    println!("\n[FJ-1364] Pin Hash Determinism:");
    let h1 = pin_hash("apt", "nginx", "1.24.0-2");
    let h2 = pin_hash("apt", "nginx", "1.24.0-2");
    let h3 = pin_hash("apt", "nginx", "1.25.0");
    let hash_ok = h1 == h2 && h1 != h3;
    println!(
        "  Same inputs → same hash: {}",
        if h1 == h2 { "✓" } else { "✗ FALSIFIED" }
    );
    println!(
        "  Different version → different hash: {}",
        if h1 != h3 { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(hash_ok);

    // ── FJ-2001: Query Engine ──
    println!("\n[FJ-2001] Query Engine (in-memory):");
    let conn = forjar::core::store::db::open_state_db(std::path::Path::new(":memory:")).unwrap();

    // Seed test data
    conn.execute(
        "INSERT INTO machines (name, first_seen, last_seen) VALUES ('web-01', 'now', 'now')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO generations (generation_num, run_id, config_hash, created_at) \
         VALUES (1, 'run-1', 'hash', 'now')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO resources (machine_id, generation_id, resource_id, resource_type, \
         status, content_hash, live_hash, applied_at) \
         VALUES (1, 1, 'nginx-pkg', 'package', 'converged', 'abc', 'abc', 'now')",
        rusqlite::params![],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO resources (machine_id, generation_id, resource_id, resource_type, \
         status, content_hash, live_hash, applied_at) \
         VALUES (1, 1, 'config-file', 'file', 'drifted', 'aaa', 'bbb', 'now')",
        rusqlite::params![],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO events (run_id, resource_id, machine, event_type, timestamp) \
         VALUES ('run-1', 'nginx-pkg', 'web-01', 'converged', 'now')",
        [],
    )
    .unwrap();

    let health = query_health(&conn).unwrap();
    println!(
        "  Health: {:.0}% ({}/{} converged)",
        health.health_pct(),
        health.total_converged,
        health.total_resources
    );

    let drift = query_drift(&conn).unwrap();
    println!("  Drifted: {} resources", drift.len());

    let events = query_events(&conn, None, None, 50).unwrap();
    println!("  Events: {} total", events.len());

    let query_ok = health.total_resources == 2
        && health.total_converged == 1
        && drift.len() == 1
        && events.len() == 1;
    println!(
        "  Query results: {}",
        if query_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(query_ok);

    println!("\n{}", "=".repeat(50));
    println!("All pin/query criteria survived.");
}
