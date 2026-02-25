//! Demonstrate the provenance event log system.
//!
//! Shows how forjar records apply events, resource outcomes, and
//! how to read back event history for auditing and anomaly detection.
//!
//! Usage: cargo run --example event_logging

use forjar::core::types::ProvenanceEvent;
use forjar::tripwire::eventlog;

fn main() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let state_dir = dir.path();
    let machine = "web-server";

    println!("=== Event Logging Example ===\n");
    println!("State dir: {}\n", state_dir.display());

    // Simulate an apply run
    let run_id = eventlog::generate_run_id();
    println!("Run ID: {}\n", run_id);

    // 1. Apply started
    eventlog::append_event(
        state_dir,
        machine,
        ProvenanceEvent::ApplyStarted {
            machine: machine.to_string(),
            run_id: run_id.clone(),
            forjar_version: env!("CARGO_PKG_VERSION").to_string(),
        },
    )
    .expect("write event");
    println!("Logged: apply_started");

    // 2. Resource started
    eventlog::append_event(
        state_dir,
        machine,
        ProvenanceEvent::ResourceStarted {
            machine: machine.to_string(),
            resource: "nginx-config".to_string(),
            action: "Create".to_string(),
        },
    )
    .expect("write event");
    println!("Logged: resource_started (nginx-config)");

    // 3. Resource converged
    eventlog::append_event(
        state_dir,
        machine,
        ProvenanceEvent::ResourceConverged {
            machine: machine.to_string(),
            resource: "nginx-config".to_string(),
            duration_seconds: 0.42,
            hash: "blake3:a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd"
                .to_string(),
        },
    )
    .expect("write event");
    println!("Logged: resource_converged (nginx-config, 0.42s)");

    // 4. Another resource that fails
    eventlog::append_event(
        state_dir,
        machine,
        ProvenanceEvent::ResourceStarted {
            machine: machine.to_string(),
            resource: "ssl-cert".to_string(),
            action: "Create".to_string(),
        },
    )
    .expect("write event");

    eventlog::append_event(
        state_dir,
        machine,
        ProvenanceEvent::ResourceFailed {
            machine: machine.to_string(),
            resource: "ssl-cert".to_string(),
            error: "exit code 1: certbot not found".to_string(),
        },
    )
    .expect("write event");
    println!("Logged: resource_failed (ssl-cert)");

    // 5. Apply completed
    eventlog::append_event(
        state_dir,
        machine,
        ProvenanceEvent::ApplyCompleted {
            machine: machine.to_string(),
            run_id: run_id.clone(),
            resources_converged: 1,
            resources_unchanged: 0,
            resources_failed: 1,
            total_seconds: 1.23,
        },
    )
    .expect("write event");
    println!("Logged: apply_completed (1 converged, 1 failed)\n");

    // Read back the event log
    println!("=== Reading Event Log ===\n");
    let events_path = state_dir.join(machine).join("events.jsonl");
    let content = std::fs::read_to_string(&events_path).expect("read events");

    for (i, line) in content.lines().enumerate() {
        let event: serde_json::Value = serde_json::from_str(line).expect("parse JSON");
        let ts = event["ts"].as_str().unwrap_or("?");
        let evt = event["event"].as_str().unwrap_or("?");
        println!("  Event {}: [{}] {}", i + 1, ts, evt);

        // Show details for key events
        match evt {
            "resource_converged" => {
                let res = event["resource"].as_str().unwrap_or("?");
                let dur = event["duration_seconds"].as_f64().unwrap_or(0.0);
                println!("    resource: {}, duration: {:.2}s", res, dur);
            }
            "resource_failed" => {
                let res = event["resource"].as_str().unwrap_or("?");
                let err = event["error"].as_str().unwrap_or("?");
                println!("    resource: {}, error: {}", res, err);
            }
            "apply_completed" => {
                let converged = event["resources_converged"].as_u64().unwrap_or(0);
                let failed = event["resources_failed"].as_u64().unwrap_or(0);
                let total = event["total_seconds"].as_f64().unwrap_or(0.0);
                println!(
                    "    converged: {}, failed: {}, total: {:.2}s",
                    converged, failed, total
                );
            }
            _ => {}
        }
    }

    println!("\n=== Event Logging Example Complete ===");
}
