//! FJ-2005: Destroy-log replay — inspect and classify destroy entries.
//!
//! Demonstrates parsing destroy-log.jsonl entries and classifying them
//! by reliability for undo-destroy recovery.
//!
//! ```bash
//! cargo run --example destroy_replay
//! ```

use forjar::core::types::DestroyLogEntry;

fn main() {
    println!("=== FJ-2005: Undo-Destroy Replay ===\n");

    // Simulate destroy-log.jsonl entries
    let entries = vec![
        make_entry("nginx-config", "file", "web-01", true, Some(NGINX_FRAGMENT)),
        make_entry("monitoring-pkg", "package", "web-01", true, Some("type: package\nprovider: apt\npackages:\n  - prometheus-node-exporter\n")),
        make_entry("redis-svc", "service", "cache-01", false, None),
        make_entry("legacy-cron", "cron", "web-01", true, Some("type: cron\nschedule: '0 * * * *'\ncommand: /usr/local/bin/cleanup.sh\n")),
        make_entry("custom-task", "task", "db-01", false, None),
    ];

    // Classify entries
    let reliable: Vec<_> = entries.iter().filter(|e| e.reliable_recreate).collect();
    let unreliable: Vec<_> = entries.iter().filter(|e| !e.reliable_recreate).collect();
    let with_fragment: Vec<_> = entries.iter().filter(|e| e.config_fragment.is_some()).collect();
    let no_fragment: Vec<_> = entries.iter().filter(|e| e.config_fragment.is_none()).collect();

    println!("Destroy log: {} entries total", entries.len());
    println!("  Reliable:       {} (can be safely recreated)", reliable.len());
    println!("  Unreliable:     {} (best-effort only)", unreliable.len());
    println!("  With fragment:  {} (config preserved)", with_fragment.len());
    println!("  No fragment:    {} (cannot reconstruct)\n", no_fragment.len());

    // Show replay plan
    println!("=== Replay Plan (--dry-run) ===\n");
    for e in &reliable {
        let marker = if e.config_fragment.is_some() { "+" } else { "?" };
        println!("  {marker} {} ({}, {}) — reliable", e.resource_id, e.resource_type, e.machine);
    }
    for e in &unreliable {
        let marker = if e.config_fragment.is_some() { "~" } else { "x" };
        println!("  {marker} {} ({}, {}) — unreliable, skipped without --force",
            e.resource_id, e.resource_type, e.machine);
    }

    // Show what --force would add
    println!("\n=== With --force ===\n");
    let force_count = entries.iter().filter(|e| e.config_fragment.is_some()).count();
    let skip_count = entries.iter().filter(|e| e.config_fragment.is_none()).count();
    println!("  Would replay: {} entries", force_count);
    println!("  Would skip:   {} entries (no config_fragment)", skip_count);

    // Machine grouping
    println!("\n=== By Machine ===\n");
    let mut machines: std::collections::BTreeMap<&str, Vec<&DestroyLogEntry>> = std::collections::BTreeMap::new();
    for e in &entries {
        machines.entry(&e.machine).or_default().push(e);
    }
    for (machine, entries) in &machines {
        let r = entries.iter().filter(|e| e.reliable_recreate).count();
        println!("  {machine}: {} entries ({r} reliable)", entries.len());
    }

    // JSONL roundtrip
    println!("\n=== JSONL Serialization ===\n");
    let jsonl = serde_json::to_string(&entries[0]).unwrap();
    println!("  {jsonl}");
    let parsed: DestroyLogEntry = serde_json::from_str(&jsonl).unwrap();
    println!("  Roundtrip: {} (gen {})", parsed.resource_id, parsed.generation);
}

const NGINX_FRAGMENT: &str = "\
type: file
path: /etc/nginx/nginx.conf
content: |
  worker_processes auto;
  events { worker_connections 1024; }
";

fn make_entry(
    resource_id: &str,
    resource_type: &str,
    machine: &str,
    reliable: bool,
    fragment: Option<&str>,
) -> DestroyLogEntry {
    DestroyLogEntry {
        timestamp: "2026-03-05T14:30:00Z".into(),
        machine: machine.into(),
        resource_id: resource_id.into(),
        resource_type: resource_type.into(),
        pre_hash: "blake3:abc123def456".into(),
        generation: 5,
        config_fragment: fragment.map(String::from),
        reliable_recreate: reliable,
    }
}
