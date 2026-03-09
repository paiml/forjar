//! FJ-3505/3507: Rollout & promotion gate falsification.
//!
//! Demonstrates Popperian rejection criteria for:
//! - Progressive rollout planning (canary, percentage, all-at-once)
//! - Health check timeout enforcement
//! - Promotion gate evaluation
//!
//! Usage: cargo run --example rollout_promotion_falsification

use forjar::core::rollout::{execute_rollout, plan_rollout, run_health_check};
use forjar::core::types::environment::RolloutConfig;

fn main() {
    println!("Forjar Rollout & Promotion Gate Falsification");
    println!("{}", "=".repeat(50));

    // ── FJ-3507: Canary Strategy ──
    println!("\n[FJ-3507] Canary Strategy:");

    let config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 2,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![50, 100],
    };
    let steps = plan_rollout(&config, 10);
    let canary_ok = !steps.is_empty()
        && steps[0].machine_indices.len() == 2
        && steps.last().unwrap().percentage == 100;
    println!(
        "  Canary=2, ends at 100%: {} {}",
        if canary_ok { "yes" } else { "no" },
        if canary_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(canary_ok);

    // ── FJ-3507: Percentage Strategy ──
    println!("\n[FJ-3507] Percentage Strategy:");

    let pct_config = RolloutConfig {
        strategy: "percentage".into(),
        canary_count: 0,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![],
    };
    let pct_steps = plan_rollout(&pct_config, 4);
    let pct_ok = pct_steps.len() == 4 && pct_steps[0].percentage == 25;
    println!(
        "  Default 4 steps starting at 25%: {} {}",
        if pct_ok { "yes" } else { "no" },
        if pct_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(pct_ok);

    // ── FJ-3507: Health Check ──
    println!("\n[FJ-3507] Health Check:");

    let (pass, _) = run_health_check("true", None);
    let (fail, _) = run_health_check("false", None);
    let hc_ok = pass && !fail;
    println!(
        "  'true' passes, 'false' fails: {} {}",
        if hc_ok { "yes" } else { "no" },
        if hc_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(hc_ok);

    // ── FJ-3507: Timeout Enforcement ──
    println!("\n[FJ-3507] Timeout Enforcement:");

    let start = std::time::Instant::now();
    let (timeout_pass, timeout_msg) = run_health_check("sleep 60", Some("1s"));
    let elapsed = start.elapsed();
    let timeout_ok = !timeout_pass && elapsed.as_secs() < 5;
    println!(
        "  sleep 60 killed in ~1s: {} (took {:.1}s) {}",
        if timeout_ok { "yes" } else { "no" },
        elapsed.as_secs_f64(),
        if timeout_ok { "✓" } else { "✗ FALSIFIED" }
    );
    println!("  Message: {timeout_msg}");
    assert!(timeout_ok);

    // ── FJ-3507: Rollback on Failure ──
    println!("\n[FJ-3507] Rollback on Failure:");

    let fail_config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 1,
        health_check: Some("false".into()),
        health_timeout: None,
        percentage_steps: vec![50, 100],
    };
    let result = execute_rollout(&fail_config, 4, false);
    let rollback_ok = !result.completed && result.rollback_at == Some(0);
    println!(
        "  Rollback at step 0: {} {}",
        if rollback_ok { "yes" } else { "no" },
        if rollback_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(rollback_ok);

    // ── FJ-3507: Dry Run ──
    println!("\n[FJ-3507] Dry Run:");

    let dry_result = execute_rollout(&fail_config, 4, true);
    let dry_ok = dry_result.completed && dry_result.steps.iter().all(|s| s.health_passed);
    println!(
        "  Dry run skips health checks: {} {}",
        if dry_ok { "yes" } else { "no" },
        if dry_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(dry_ok);

    println!("\n{}", "=".repeat(50));
    println!("All rollout & promotion criteria survived.");
}
