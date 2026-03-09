//! FJ-2402/2403: WASM types, CI pipeline types — size budgets, drift, MSRV, features.
//! Usage: cargo test --test falsification_wasm_ci_types

use forjar::core::types::*;

// ============================================================================
// FJ-2402: WasmOptLevel
// ============================================================================

#[test]
fn wasm_opt_flags() {
    assert_eq!(WasmOptLevel::Fast.flag(), "-O1");
    assert_eq!(WasmOptLevel::Balanced.flag(), "-O2");
    assert_eq!(WasmOptLevel::MaxSpeed.flag(), "-O3");
    assert_eq!(WasmOptLevel::MinSize.flag(), "-Oz");
}

#[test]
fn wasm_opt_default() {
    assert_eq!(WasmOptLevel::default(), WasmOptLevel::MinSize);
}

#[test]
fn wasm_opt_display() {
    assert_eq!(WasmOptLevel::Fast.to_string(), "-O1");
    assert_eq!(WasmOptLevel::MinSize.to_string(), "-Oz");
}

#[test]
fn wasm_opt_serde_roundtrip() {
    for level in [
        WasmOptLevel::Fast,
        WasmOptLevel::Balanced,
        WasmOptLevel::MaxSpeed,
        WasmOptLevel::MinSize,
    ] {
        let json = serde_json::to_string(&level).unwrap();
        let parsed: WasmOptLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, level);
    }
}

// ============================================================================
// FJ-2402: WasmSizeBudget
// ============================================================================

#[test]
fn size_budget_defaults() {
    let b = WasmSizeBudget::default();
    assert_eq!(b.core_kb, 100);
    assert_eq!(b.widgets_kb, 150);
    assert_eq!(b.full_app_kb, 500);
}

#[test]
fn size_budget_check_core_within() {
    let b = WasmSizeBudget::default();
    assert!(b.check_core(100 * 1024)); // exactly at limit
    assert!(b.check_core(50 * 1024));
}

#[test]
fn size_budget_check_core_over() {
    let b = WasmSizeBudget::default();
    assert!(!b.check_core(101 * 1024));
}

#[test]
fn size_budget_check_full_app() {
    let b = WasmSizeBudget::default();
    assert!(b.check_full_app(500 * 1024));
    assert!(!b.check_full_app(501 * 1024));
}

#[test]
fn size_budget_custom() {
    let b = WasmSizeBudget {
        core_kb: 50,
        widgets_kb: 75,
        full_app_kb: 200,
    };
    assert!(b.check_core(50 * 1024));
    assert!(!b.check_core(51 * 1024));
}

// ============================================================================
// FJ-2402: BundleSizeDrift
// ============================================================================

#[test]
fn drift_within_budget_no_previous() {
    let budget = WasmSizeBudget::default();
    let d = BundleSizeDrift::check(&budget, 90 * 1024, None);
    assert!(d.is_ok());
    assert!(!d.exceeds_budget);
    assert!(!d.exceeds_growth_limit);
}

#[test]
fn drift_exceeds_budget() {
    let budget = WasmSizeBudget::default();
    let d = BundleSizeDrift::check(&budget, 110 * 1024, None);
    assert!(!d.is_ok());
    assert!(d.exceeds_budget);
}

#[test]
fn drift_exceeds_growth_limit() {
    let budget = WasmSizeBudget {
        core_kb: 200,
        ..Default::default()
    };
    let d = BundleSizeDrift::check(&budget, 130 * 1024, Some(100 * 1024));
    assert!(d.exceeds_growth_limit); // 30% > 20%
    assert!(!d.exceeds_budget);
    assert!(!d.is_ok());
}

#[test]
fn drift_within_growth_limit() {
    let budget = WasmSizeBudget::default();
    let d = BundleSizeDrift::check(&budget, 95 * 1024, Some(90 * 1024));
    assert!(d.is_ok()); // ~5.5% growth, within 20%
}

#[test]
fn drift_current_kb() {
    let budget = WasmSizeBudget::default();
    let d = BundleSizeDrift::check(&budget, 50 * 1024, None);
    assert_eq!(d.current_kb(), 50);
}

#[test]
fn drift_display_ok() {
    let budget = WasmSizeBudget::default();
    let d = BundleSizeDrift::check(&budget, 90 * 1024, Some(85 * 1024));
    let s = d.to_string();
    assert!(s.contains("OK:"));
    assert!(s.contains("prev:"));
}

#[test]
fn drift_display_exceeded() {
    let budget = WasmSizeBudget::default();
    let d = BundleSizeDrift::check(&budget, 110 * 1024, None);
    let s = d.to_string();
    assert!(s.contains("BUDGET EXCEEDED"));
}

// ============================================================================
// FJ-2402: CdnTarget
// ============================================================================

#[test]
fn cdn_target_s3() {
    let t = CdnTarget::S3 {
        bucket: "my-bucket".into(),
        region: Some("us-east-1".into()),
        distribution: Some("E123".into()),
    };
    assert_eq!(t.name(), "s3");
    assert_eq!(t.to_string(), "s3://my-bucket");
}

#[test]
fn cdn_target_cloudflare() {
    let t = CdnTarget::Cloudflare {
        project: "app".into(),
    };
    assert_eq!(t.name(), "cloudflare");
    assert_eq!(t.to_string(), "cloudflare:app");
}

#[test]
fn cdn_target_local() {
    let t = CdnTarget::Local {
        path: "/var/www".into(),
    };
    assert_eq!(t.name(), "local");
    assert_eq!(t.to_string(), "local:/var/www");
}

#[test]
fn cdn_target_serde() {
    let t = CdnTarget::S3 {
        bucket: "b".into(),
        region: None,
        distribution: None,
    };
    let json = serde_json::to_string(&t).unwrap();
    let parsed: CdnTarget = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name(), "s3");
}

// ============================================================================
// FJ-2402: CachePolicy
// ============================================================================

#[test]
fn cache_policy_defaults() {
    let policies = CachePolicy::defaults();
    assert_eq!(policies.len(), 4);
    assert_eq!(policies[0].extension, ".wasm");
    assert!(policies[0].cache_control.contains("immutable"));
    assert_eq!(policies[3].extension, ".html");
    assert!(policies[3].cache_control.contains("no-cache"));
}

// ============================================================================
// FJ-2402: WasmBuildResult
// ============================================================================

#[test]
fn build_result_kb() {
    let r = WasmBuildResult {
        wasm_path: "dist/app.wasm".into(),
        wasm_size: 512 * 1024,
        js_size: 20 * 1024,
        total_size: 532 * 1024,
        duration_secs: 10.0,
        opt_level: WasmOptLevel::MinSize,
    };
    assert_eq!(r.wasm_kb(), 512);
    assert_eq!(r.total_kb(), 532);
}

#[test]
fn build_result_display() {
    let r = WasmBuildResult {
        wasm_path: "x.wasm".into(),
        wasm_size: 100 * 1024,
        js_size: 10 * 1024,
        total_size: 110 * 1024,
        duration_secs: 5.5,
        opt_level: WasmOptLevel::Balanced,
    };
    let s = r.to_string();
    assert!(s.contains("100 KB"));
    assert!(s.contains("-O2"));
}

// ============================================================================
// FJ-2403: ReproBuildConfig
// ============================================================================

#[test]
fn repro_default_reproducible() {
    let c = ReproBuildConfig::default();
    assert!(c.is_reproducible());
    assert!(c.locked);
    assert!(c.lto);
    assert_eq!(c.codegen_units, 1);
}

#[test]
fn repro_not_reproducible_unlocked() {
    let c = ReproBuildConfig {
        locked: false,
        ..Default::default()
    };
    assert!(!c.is_reproducible());
}

#[test]
fn repro_not_reproducible_multi_codegen() {
    let c = ReproBuildConfig {
        codegen_units: 4,
        ..Default::default()
    };
    assert!(!c.is_reproducible());
}

#[test]
fn repro_cargo_args() {
    let c = ReproBuildConfig::default();
    let args = c.cargo_args();
    assert!(args.contains(&"build".to_string()));
    assert!(args.contains(&"--release".to_string()));
    assert!(args.contains(&"--locked".to_string()));
}

#[test]
fn repro_cargo_args_unlocked() {
    let c = ReproBuildConfig {
        locked: false,
        ..Default::default()
    };
    let args = c.cargo_args();
    assert!(!args.contains(&"--locked".to_string()));
}

#[test]
fn repro_env_vars() {
    let c = ReproBuildConfig {
        source_date_epoch: Some(1234),
        ..Default::default()
    };
    let vars = c.env_vars();
    assert!(vars.iter().any(|(k, _)| k == "CARGO_INCREMENTAL"));
    assert!(vars
        .iter()
        .any(|(k, v)| k == "SOURCE_DATE_EPOCH" && v == "1234"));
}

#[test]
fn repro_serde_roundtrip() {
    let c = ReproBuildConfig::default();
    let json = serde_json::to_string(&c).unwrap();
    let parsed: ReproBuildConfig = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_reproducible());
}

// ============================================================================
// FJ-2403: MsrvCheck
// ============================================================================

#[test]
fn msrv_satisfies_exact() {
    let m = MsrvCheck::new("1.88.0");
    assert!(m.satisfies("1.88.0"));
}

#[test]
fn msrv_satisfies_higher() {
    let m = MsrvCheck::new("1.88.0");
    assert!(m.satisfies("1.89.0"));
    assert!(m.satisfies("2.0.0"));
}

#[test]
fn msrv_rejects_lower() {
    let m = MsrvCheck::new("1.88.0");
    assert!(!m.satisfies("1.87.9"));
    assert!(!m.satisfies("1.87.0"));
}

#[test]
fn msrv_patch_level() {
    let m = MsrvCheck::new("1.88.1");
    assert!(!m.satisfies("1.88.0"));
    assert!(m.satisfies("1.88.1"));
    assert!(m.satisfies("1.88.2"));
}

// ============================================================================
// FJ-2403: FeatureMatrix
// ============================================================================

#[test]
fn feature_matrix_empty() {
    let m = FeatureMatrix::new(vec![]);
    assert_eq!(m.combinations().len(), 1);
}

#[test]
fn feature_matrix_single() {
    let m = FeatureMatrix::new(vec!["encryption"]);
    let combos = m.combinations();
    assert_eq!(combos.len(), 2);
}

#[test]
fn feature_matrix_two() {
    let m = FeatureMatrix::new(vec!["a", "b"]);
    let combos = m.combinations();
    assert_eq!(combos.len(), 4);
    assert!(combos.contains(&vec![]));
    assert!(combos.contains(&vec!["a".to_string(), "b".to_string()]));
}

#[test]
fn feature_matrix_three() {
    let m = FeatureMatrix::new(vec!["a", "b", "c"]);
    assert_eq!(m.combinations().len(), 8); // 2^3
}

#[test]
fn feature_matrix_cargo_commands() {
    let m = FeatureMatrix::new(vec!["encryption"]);
    let cmds = m.cargo_commands();
    assert_eq!(cmds.len(), 2);
    assert!(cmds
        .iter()
        .any(|c| c.contains("--no-default-features") && !c.contains("--features")));
    assert!(cmds.iter().any(|c| c.contains("--features encryption")));
}

// ============================================================================
// FJ-2400: PurificationBenchmark
// ============================================================================

#[test]
fn purification_ratio() {
    let b = PurificationBenchmark {
        resource_type: "file".into(),
        validate_us: 50.0,
        purify_us: 150.0,
        sample_count: 100,
    };
    assert!((b.overhead_ratio() - 3.0).abs() < 0.01);
}

#[test]
fn purification_zero_validate() {
    let b = PurificationBenchmark {
        resource_type: "file".into(),
        validate_us: 0.0,
        purify_us: 100.0,
        sample_count: 10,
    };
    assert!((b.overhead_ratio() - 0.0).abs() < 0.01);
}

#[test]
fn purification_display() {
    let b = PurificationBenchmark {
        resource_type: "service".into(),
        validate_us: 100.0,
        purify_us: 200.0,
        sample_count: 50,
    };
    let s = b.to_string();
    assert!(s.contains("service"));
    assert!(s.contains("2.0x"));
}

// ============================================================================
// FJ-2401: ModelIntegrityCheck
// ============================================================================

#[test]
fn model_integrity_valid() {
    let c = ModelIntegrityCheck::check("llama3", "abc", "abc", 1024 * 1024);
    assert!(c.valid);
    assert!((c.size_mb() - 1.0).abs() < 0.01);
}

#[test]
fn model_integrity_invalid() {
    let c = ModelIntegrityCheck::check("gpt2", "expected", "actual", 5_000_000_000);
    assert!(!c.valid);
    let s = c.to_string();
    assert!(s.contains("[MISMATCH]"));
}

#[test]
fn model_integrity_display_ok() {
    let c = ModelIntegrityCheck::check("model", "abcdef01", "abcdef01", 500_000_000);
    let s = c.to_string();
    assert!(s.contains("[OK]"));
    assert!(s.contains("model"));
}
