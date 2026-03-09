//! FJ-2803: Popperian falsification of ForjarScore v2 dimensions.
//!
//! Each scoring dimension must state conditions under which it would be
//! rejected as invalid. This example demonstrates the falsification criteria
//! from the spec — if any assertion fails, the dimension is measuring
//! the wrong thing.
//!
//! Usage: cargo run --example scoring_falsification

use forjar::core::scoring::{compute, score_bar, RuntimeData, ScoringInput, SCORE_VERSION};
use forjar::core::types::{ForjarConfig, OutputValue, Resource, ResourceType};

fn base_config() -> ForjarConfig {
    let mut c = ForjarConfig::default();
    c.name = "falsification-test".into();
    c.version = "1.0".into();
    c
}

fn base_input() -> ScoringInput {
    ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 60_000,
        runtime: None,
        raw_yaml: None,
    }
}

fn full_runtime() -> RuntimeData {
    RuntimeData {
        validate_pass: true,
        plan_pass: true,
        first_apply_pass: true,
        second_apply_pass: true,
        zero_changes_on_reapply: true,
        hash_stable: true,
        all_resources_converged: true,
        state_lock_written: true,
        warning_count: 0,
        changed_on_reapply: 0,
        first_apply_ms: 20_000,
        second_apply_ms: 500,
    }
}

fn dim_score(config: &ForjarConfig, input: &ScoringInput, code: &str) -> u32 {
    compute(config, input)
        .dimensions
        .iter()
        .find(|d| d.code == code)
        .unwrap()
        .score
}

fn main() {
    println!("ForjarScore v{SCORE_VERSION} — Popperian Falsification");
    println!("{}", "=".repeat(55));

    // ── SAF ──
    println!("\n[SAF] Safety dimension:");

    // Falsifier: mode:0777 must cap SAF at 40
    let mut cfg = base_config();
    let mut file = Resource::default();
    file.resource_type = ResourceType::File;
    file.mode = Some("0777".into());
    cfg.resources.insert("danger".into(), file);
    let saf = dim_score(&cfg, &base_input(), "SAF");
    println!(
        "  mode:0777       → SAF={saf:>3}  (must be ≤40) {}",
        if saf <= 40 { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(saf <= 40);

    // Boundary: no files → SAF=100
    let mut cfg2 = base_config();
    let mut pkg = Resource::default();
    pkg.resource_type = ResourceType::Package;
    pkg.version = Some("1.0".into());
    cfg2.resources.insert("pkg".into(), pkg);
    let saf2 = dim_score(&cfg2, &base_input(), "SAF");
    println!(
        "  no files        → SAF={saf2:>3}  (must be 100) {}",
        if saf2 == 100 { "✓" } else { "✗ FALSIFIED" }
    );
    assert_eq!(saf2, 100);

    // ── OBS ──
    println!("\n[OBS] Observability dimension:");

    let mut cfg = base_config();
    cfg.policy.tripwire = true;
    cfg.policy.lock_file = true;
    cfg.policy.notify.on_success = Some("echo ok".into());
    cfg.policy.notify.on_failure = Some("echo fail".into());
    cfg.policy.notify.on_drift = Some("echo drift".into());
    let out = OutputValue {
        value: "v".into(),
        description: Some("desc".into()),
    };
    cfg.outputs.insert("r".into(), out);
    let mut f = Resource::default();
    f.resource_type = ResourceType::File;
    f.mode = Some("0644".into());
    f.owner = Some("root".into());
    cfg.resources.insert("c".into(), f);
    let obs = dim_score(&cfg, &base_input(), "OBS");
    println!(
        "  full policy     → OBS={obs:>3}  (must be ≥90) {}",
        if obs >= 90 { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(obs >= 90);

    let mut cfg2 = base_config();
    cfg2.policy.tripwire = false;
    cfg2.policy.lock_file = false;
    let obs2 = dim_score(&cfg2, &base_input(), "OBS");
    println!(
        "  disabled policy → OBS={obs2:>3}  (must be ≤15) {}",
        if obs2 <= 15 { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(obs2 <= 15);

    // ── COR ──
    println!("\n[COR] Correctness dimension:");

    let mut input_rt = base_input();
    input_rt.runtime = Some(full_runtime());
    let cor = dim_score(&base_config(), &input_rt, "COR");
    println!(
        "  full convergence→ COR={cor:>3}  (must be ≥90) {}",
        if cor >= 90 { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(cor >= 90);

    let mut rt_fail = full_runtime();
    rt_fail.validate_pass = false;
    rt_fail.plan_pass = false;
    rt_fail.first_apply_pass = false;
    rt_fail.all_resources_converged = false;
    rt_fail.state_lock_written = false;
    let mut input_fail = base_input();
    input_fail.runtime = Some(rt_fail);
    let cor2 = dim_score(&base_config(), &input_fail, "COR");
    println!(
        "  nothing passes  → COR={cor2:>3}  (must be   0) {}",
        if cor2 == 0 { "✓" } else { "✗ FALSIFIED" }
    );
    assert_eq!(cor2, 0);

    // ── IDM ──
    println!("\n[IDM] Idempotency dimension:");

    let idm = dim_score(&base_config(), &input_rt, "IDM");
    println!(
        "  strong+stable   → IDM={idm:>3}  (must be ≥90) {}",
        if idm >= 90 { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(idm >= 90);

    let mut rt_bad = full_runtime();
    rt_bad.zero_changes_on_reapply = false;
    rt_bad.hash_stable = false;
    rt_bad.changed_on_reapply = 3;
    let mut input_bad = base_input();
    input_bad.idempotency = "eventual".into();
    input_bad.runtime = Some(rt_bad);
    let idm2 = dim_score(&base_config(), &input_bad, "IDM");
    println!(
        "  3 changes       → IDM={idm2:>3}  (must be <50) {}",
        if idm2 < 50 { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(idm2 < 50);

    // ── PRF ──
    println!("\n[PRF] Performance dimension:");

    let prf = dim_score(&base_config(), &input_rt, "PRF");
    println!(
        "  33% of budget   → PRF={prf:>3}  (must be ≥70) {}",
        if prf >= 70 { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(prf >= 70);

    let mut rt_slow = full_runtime();
    rt_slow.first_apply_ms = 120_000;
    rt_slow.second_apply_ms = 60_000;
    let mut input_slow = base_input();
    input_slow.runtime = Some(rt_slow);
    let prf2 = dim_score(&base_config(), &input_slow, "PRF");
    println!(
        "  200% of budget  → PRF={prf2:>3}  (must be ≤25) {}",
        if prf2 <= 25 { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(prf2 <= 25);

    // ── Monotonicity ──
    println!("\n[MONO] Monotonicity invariant:");

    let base = compute(&base_config(), &base_input()).static_composite;
    let mut cfg_deny = base_config();
    cfg_deny.policy.deny_paths = vec!["/etc/shadow".into()];
    let with_deny = compute(&cfg_deny, &base_input()).static_composite;
    let mono_ok = with_deny >= base;
    println!(
        "  +deny_paths     → {base} → {with_deny}  (must be ≥) {}",
        if mono_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(mono_ok);

    // ── Grade Format ──
    println!("\n[FORMAT] Two-tier grade display:");

    let mut input_rt2 = base_input();
    input_rt2.runtime = Some(full_runtime());
    let r1 = compute(&base_config(), &input_rt2);
    println!("  with runtime    → \"{}\"  (must contain '/') ✓", r1.grade);
    assert!(r1.grade.contains('/'));

    let r2 = compute(&base_config(), &base_input());
    println!(
        "  no runtime      → \"{}\"  (must contain '/pending') ✓",
        r2.grade
    );
    assert!(r2.grade.contains("/pending"));

    // ── Score bar ──
    println!("\n[BAR] Score bar rendering:");
    println!("  0:   {}", score_bar(0));
    println!("  50:  {}", score_bar(50));
    println!("  100: {}", score_bar(100));

    println!("\n{}", "=".repeat(55));
    println!("All falsification criteria survived. ForjarScore v{SCORE_VERSION} is valid.");
}
