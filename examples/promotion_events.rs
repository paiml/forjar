//! Example: Promotion event logging (FJ-3509)
//!
//! Demonstrates structured JSONL event logging for environment
//! promotions and rollbacks.
//!
//! ```bash
//! cargo run --example promotion_events
//! ```

use forjar::core::promotion_events::{
    log_promotion, log_promotion_failure, log_rollback, PromotionParams,
};

fn main() {
    println!("=== Promotion Event Logging (FJ-3509) ===\n");

    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    // 1. Successful promotion
    println!("1. Log successful promotion (dev → staging):");
    let params = PromotionParams {
        state_dir: &state_dir,
        target_env: "staging",
        source: "dev",
        target: "staging",
        gates_passed: 3,
        gates_total: 3,
        rollout_strategy: Some("canary"),
    };
    log_promotion(&params).unwrap();
    println!("  Logged: source=dev target=staging gates=3/3 strategy=canary");

    // 2. Failed promotion
    println!("\n2. Log failed promotion (staging → prod):");
    let params = PromotionParams {
        state_dir: &state_dir,
        target_env: "prod",
        source: "staging",
        target: "prod",
        gates_passed: 1,
        gates_total: 4,
        rollout_strategy: None,
    };
    log_promotion_failure(&params).unwrap();
    println!("  Logged: source=staging target=prod gates=1/4 success=false");

    // 3. Rollback
    println!("\n3. Log rollback event:");
    log_rollback(&state_dir, "prod", 2, "canary health check failed: 503").unwrap();
    println!("  Logged: env=prod step=2 reason='canary health check failed: 503'");

    // 4. Read the event logs
    println!("\n4. Event log contents:");
    for env in &["staging", "prod"] {
        let log_path = state_dir.join(env).join("events.jsonl");
        if let Ok(content) = std::fs::read_to_string(&log_path) {
            println!("\n  [{env}] events.jsonl:");
            for line in content.lines() {
                // Pretty-print each event (truncated)
                if line.len() > 120 {
                    println!("    {}...", &line[..120]);
                } else {
                    println!("    {line}");
                }
            }
        }
    }

    println!("\nDone.");
}
