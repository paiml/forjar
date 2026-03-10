//! FJ-2402/2403: Popperian falsification for WASM deployment types
//! and CI build pipeline (reproducible builds, MSRV, feature matrix).
//!
//! Each test states conditions under which the build pipeline model
//! would be rejected as invalid.

use forjar::core::types::{
    BuildMetrics, BundleSizeDrift, CachePolicy, CdnTarget, FeatureMatrix, ImageBuildMetrics,
    LayerMetric, ModelIntegrityCheck, MsrvCheck, PurificationBenchmark, ReproBuildConfig,
    SizeThreshold, WasmBuildConfig, WasmBuildResult, WasmOptLevel, WasmSizeBudget,
};

// ── FJ-2402: WASM Optimization Levels ──────────────────────────────

#[test]
fn f_2402_1_wasm_opt_level_flags_are_correct() {
    let cases = [
        (WasmOptLevel::Fast, "-O1"),
        (WasmOptLevel::Balanced, "-O2"),
        (WasmOptLevel::MaxSpeed, "-O3"),
        (WasmOptLevel::MinSize, "-Oz"),
    ];
    for (level, expected) in &cases {
        assert_eq!(
            level.flag(),
            *expected,
            "WasmOptLevel::{level} must map to {expected}"
        );
    }
}

#[test]
fn f_2402_2_wasm_opt_level_default_is_min_size() {
    assert_eq!(
        WasmOptLevel::default(),
        WasmOptLevel::MinSize,
        "production default must be MinSize (-Oz)"
    );
}

#[test]
fn f_2402_3_wasm_opt_level_display_matches_flag() {
    for level in [
        WasmOptLevel::Fast,
        WasmOptLevel::Balanced,
        WasmOptLevel::MaxSpeed,
        WasmOptLevel::MinSize,
    ] {
        assert_eq!(level.to_string(), level.flag(), "Display must equal flag()");
    }
}

#[test]
fn f_2402_4_wasm_opt_level_serde_roundtrip() {
    for level in [
        WasmOptLevel::Fast,
        WasmOptLevel::Balanced,
        WasmOptLevel::MaxSpeed,
        WasmOptLevel::MinSize,
    ] {
        let json = serde_json::to_string(&level).unwrap();
        let parsed: WasmOptLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(level, parsed, "serde roundtrip must preserve variant");
    }
}

// ── FJ-2402: WASM Size Budget ──────────────────────────────────────

#[test]
fn f_2402_5_size_budget_defaults_reasonable() {
    let budget = WasmSizeBudget::default();
    assert_eq!(budget.core_kb, 100, "default core budget must be 100 KB");
    assert_eq!(
        budget.full_app_kb, 500,
        "default full app budget must be 500 KB"
    );
}

#[test]
fn f_2402_6_size_budget_check_boundary() {
    let budget = WasmSizeBudget {
        core_kb: 100,
        widgets_kb: 150,
        full_app_kb: 500,
    };
    // Exactly at budget = ok
    assert!(budget.check_core(100 * 1024), "exactly at budget must pass");
    // One byte over = fail
    assert!(
        !budget.check_core(100 * 1024 + 1),
        "one byte over budget must fail"
    );
    // Full app boundary
    assert!(budget.check_full_app(500 * 1024));
    assert!(!budget.check_full_app(500 * 1024 + 1));
}

// ── FJ-2402: Bundle Size Drift Detection ───────────────────────────

#[test]
fn f_2402_7_drift_within_budget_is_ok() {
    let budget = WasmSizeBudget::default();
    let drift = BundleSizeDrift::check(&budget, 90 * 1024, Some(85 * 1024));
    assert!(drift.is_ok(), "90 KB within 100 KB budget must be ok");
    assert!(!drift.exceeds_budget);
    assert!(!drift.exceeds_growth_limit);
}

#[test]
fn f_2402_8_drift_exceeds_budget() {
    let budget = WasmSizeBudget::default();
    let drift = BundleSizeDrift::check(&budget, 110 * 1024, Some(90 * 1024));
    assert!(!drift.is_ok());
    assert!(drift.exceeds_budget);
    assert!(
        drift.to_string().contains("BUDGET EXCEEDED"),
        "display must say BUDGET EXCEEDED"
    );
}

#[test]
fn f_2402_9_drift_exceeds_growth_limit() {
    let big_budget = WasmSizeBudget {
        core_kb: 500,
        ..Default::default()
    };
    // 130 KB from 100 KB = 30% growth > 20% limit
    let drift = BundleSizeDrift::check(&big_budget, 130 * 1024, Some(100 * 1024));
    assert!(!drift.is_ok());
    assert!(!drift.exceeds_budget, "under absolute budget");
    assert!(drift.exceeds_growth_limit, "30% growth > 20% limit");
}

#[test]
fn f_2402_10_drift_no_previous_is_ok() {
    let budget = WasmSizeBudget::default();
    let drift = BundleSizeDrift::check(&budget, 90 * 1024, None);
    assert!(drift.is_ok(), "no previous build = no growth violation");
    assert!(drift.previous_bytes.is_none());
}

// ── FJ-2402: CDN Target Types ──────────────────────────────────────

#[test]
fn f_2402_11_cdn_target_names() {
    let s3 = CdnTarget::S3 {
        bucket: "b".into(),
        region: None,
        distribution: None,
    };
    let cf = CdnTarget::Cloudflare {
        project: "p".into(),
    };
    let local = CdnTarget::Local {
        path: "/var/www".into(),
    };
    assert_eq!(s3.name(), "s3");
    assert_eq!(cf.name(), "cloudflare");
    assert_eq!(local.name(), "local");
}

#[test]
fn f_2402_12_cdn_target_display() {
    let s3 = CdnTarget::S3 {
        bucket: "my-bucket".into(),
        region: Some("us-east-1".into()),
        distribution: None,
    };
    assert_eq!(s3.to_string(), "s3://my-bucket");

    let cf = CdnTarget::Cloudflare {
        project: "app".into(),
    };
    assert_eq!(cf.to_string(), "cloudflare:app");

    let local = CdnTarget::Local {
        path: "/var/www".into(),
    };
    assert_eq!(local.to_string(), "local:/var/www");
}

#[test]
fn f_2402_13_cdn_target_serde_roundtrip() {
    let original = CdnTarget::S3 {
        bucket: "b".into(),
        region: Some("r".into()),
        distribution: Some("d".into()),
    };
    let json = serde_json::to_string(&original).unwrap();
    let parsed: CdnTarget = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name(), "s3");
}

// ── FJ-2402: Cache Policies ────────────────────────────────────────

#[test]
fn f_2402_14_cache_policy_defaults_cover_key_extensions() {
    let policies = CachePolicy::defaults();
    let extensions: Vec<&str> = policies.iter().map(|p| p.extension.as_str()).collect();
    assert!(extensions.contains(&".wasm"), "must cover .wasm");
    assert!(extensions.contains(&".js"), "must cover .js");
    assert!(extensions.contains(&".html"), "must cover .html");
}

#[test]
fn f_2402_15_wasm_cache_is_immutable() {
    let policies = CachePolicy::defaults();
    let wasm_policy = policies.iter().find(|p| p.extension == ".wasm").unwrap();
    assert!(
        wasm_policy.cache_control.contains("immutable"),
        "WASM files must use immutable caching"
    );
}

#[test]
fn f_2402_16_html_cache_is_no_cache() {
    let policies = CachePolicy::defaults();
    let html_policy = policies.iter().find(|p| p.extension == ".html").unwrap();
    assert!(
        html_policy.cache_control.contains("no-cache"),
        "HTML files must use no-cache for freshness"
    );
}

// ── FJ-2402: WASM Build Result ─────────────────────────────────────

#[test]
fn f_2402_17_build_result_size_conversion() {
    let result = WasmBuildResult {
        wasm_path: "dist/app.wasm".into(),
        wasm_size: 512 * 1024,
        js_size: 20 * 1024,
        total_size: 532 * 1024,
        duration_secs: 5.0,
        opt_level: WasmOptLevel::MinSize,
    };
    assert_eq!(result.wasm_kb(), 512);
    assert_eq!(result.total_kb(), 532);
    let display = result.to_string();
    assert!(display.contains("512 KB"));
    assert!(display.contains("-Oz"));
}

// ── FJ-2403: Reproducible Build Config ─────────────────────────────

#[test]
fn f_2403_1_default_config_is_reproducible() {
    let config = ReproBuildConfig::default();
    assert!(
        config.is_reproducible(),
        "default build config must be reproducible"
    );
    assert!(config.locked);
    assert!(config.no_incremental);
    assert!(config.lto);
    assert_eq!(config.codegen_units, 1);
}

#[test]
fn f_2403_2_unlocked_build_not_reproducible() {
    let config = ReproBuildConfig {
        locked: false,
        ..Default::default()
    };
    assert!(
        !config.is_reproducible(),
        "unlocked build must not be reproducible"
    );
}

#[test]
fn f_2403_3_incremental_build_not_reproducible() {
    let config = ReproBuildConfig {
        no_incremental: false,
        ..Default::default()
    };
    assert!(!config.is_reproducible());
}

#[test]
fn f_2403_4_multi_codegen_units_not_reproducible() {
    let config = ReproBuildConfig {
        codegen_units: 16,
        ..Default::default()
    };
    assert!(!config.is_reproducible());
}

#[test]
fn f_2403_5_cargo_args_include_locked_and_release() {
    let config = ReproBuildConfig::default();
    let args = config.cargo_args();
    assert!(args.contains(&"--locked".to_string()));
    assert!(args.contains(&"--release".to_string()));
}

#[test]
fn f_2403_6_env_vars_include_incremental_and_epoch() {
    let config = ReproBuildConfig {
        source_date_epoch: Some(1700000000),
        ..Default::default()
    };
    let vars = config.env_vars();
    assert!(vars
        .iter()
        .any(|(k, v)| k == "CARGO_INCREMENTAL" && v == "0"));
    assert!(vars
        .iter()
        .any(|(k, v)| k == "SOURCE_DATE_EPOCH" && v == "1700000000"));
}

// ── FJ-2403: MSRV Check ───────────────────────────────────────────

#[test]
fn f_2403_7_msrv_exact_match_satisfies() {
    let check = MsrvCheck::new("1.88.0");
    assert!(check.satisfies("1.88.0"));
}

#[test]
fn f_2403_8_msrv_newer_version_satisfies() {
    let check = MsrvCheck::new("1.88.0");
    assert!(check.satisfies("1.89.0"));
    assert!(check.satisfies("2.0.0"));
}

#[test]
fn f_2403_9_msrv_older_version_does_not_satisfy() {
    let check = MsrvCheck::new("1.88.0");
    assert!(!check.satisfies("1.87.0"));
    assert!(!check.satisfies("1.87.9"));
}

#[test]
fn f_2403_10_msrv_patch_level_matters() {
    let check = MsrvCheck::new("1.88.1");
    assert!(!check.satisfies("1.88.0"), "patch 0 < required patch 1");
    assert!(check.satisfies("1.88.1"));
    assert!(check.satisfies("1.88.2"));
}

// ── FJ-2403: Feature Matrix ────────────────────────────────────────

#[test]
fn f_2403_11_feature_matrix_2n_combinations() {
    let matrix = FeatureMatrix::new(vec!["encryption", "container-test"]);
    let combos = matrix.combinations();
    assert_eq!(combos.len(), 4, "2 features → 2^2 = 4 combinations");
    // Must include empty set and full set
    assert!(combos.contains(&vec![]));
    assert!(combos.contains(&vec![
        "encryption".to_string(),
        "container-test".to_string()
    ]));
}

#[test]
fn f_2403_12_feature_matrix_single_feature() {
    let matrix = FeatureMatrix::new(vec!["tls"]);
    assert_eq!(matrix.combinations().len(), 2);
}

#[test]
fn f_2403_13_feature_matrix_empty() {
    let matrix = FeatureMatrix::new(vec![]);
    let combos = matrix.combinations();
    assert_eq!(combos.len(), 1, "0 features → 1 combination (empty set)");
}

#[test]
fn f_2403_14_feature_matrix_cargo_commands() {
    let matrix = FeatureMatrix::new(vec!["encryption"]);
    let cmds = matrix.cargo_commands();
    assert!(cmds
        .iter()
        .any(|c| c.contains("--no-default-features") && !c.contains("--features")));
    assert!(cmds.iter().any(|c| c.contains("--features encryption")));
}

// ── FJ-2403: Build Metrics ─────────────────────────────────────────

#[test]
fn f_2403_15_build_metrics_current() {
    let m = BuildMetrics::current();
    assert!(!m.version.is_empty());
    // Tests run in debug mode
    assert_eq!(m.profile, "debug");
}

#[test]
fn f_2403_16_build_metrics_size_check() {
    let mut m = BuildMetrics::current();
    m.binary_size = Some(15_000_000);
    assert!(m.exceeds_size(10_000_000), "15 MB > 10 MB threshold");
    assert!(!m.exceeds_size(20_000_000), "15 MB < 20 MB threshold");
}

#[test]
fn f_2403_17_build_metrics_no_size_never_exceeds() {
    let m = BuildMetrics::current();
    assert!(
        !m.exceeds_size(1),
        "no binary_size means it can never exceed"
    );
}

#[test]
fn f_2403_18_build_metrics_size_change_percentage() {
    let mut current = BuildMetrics::current();
    current.binary_size = Some(11_000_000);
    let mut prev = BuildMetrics::current();
    prev.binary_size = Some(10_000_000);
    let pct = current.size_change_pct(&prev).unwrap();
    assert!(
        (pct - 10.0).abs() < 0.1,
        "10M → 11M = 10% growth, got {pct}"
    );
}

// ── FJ-2403: Size Threshold ────────────────────────────────────────

#[test]
fn f_2403_19_size_threshold_pass() {
    let t = SizeThreshold::default();
    let mut m = BuildMetrics::current();
    m.binary_size = Some(8_000_000);
    let violations = t.check(&m, None);
    assert!(violations.is_empty(), "8 MB under 10 MB threshold");
}

#[test]
fn f_2403_20_size_threshold_absolute_violation() {
    let t = SizeThreshold::default();
    let mut m = BuildMetrics::current();
    m.binary_size = Some(15_000_000);
    let violations = t.check(&m, None);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].contains("exceeds threshold"));
}

#[test]
fn f_2403_21_size_threshold_growth_violation() {
    let t = SizeThreshold::default();
    let mut current = BuildMetrics::current();
    current.binary_size = Some(9_000_000);
    let mut prev = BuildMetrics::current();
    prev.binary_size = Some(7_000_000);
    let violations = t.check(&current, Some(&prev));
    assert_eq!(violations.len(), 1);
    assert!(violations[0].contains("grew"));
}

// ── FJ-2400: Purification Benchmark ────────────────────────────────

#[test]
fn f_2400_1_purification_overhead_ratio() {
    let b = PurificationBenchmark {
        resource_type: "file".into(),
        validate_us: 50.0,
        purify_us: 150.0,
        sample_count: 100,
    };
    assert!(
        (b.overhead_ratio() - 3.0).abs() < 0.01,
        "150/50 = 3.0x overhead"
    );
}

#[test]
fn f_2400_2_purification_zero_validate_no_panic() {
    let b = PurificationBenchmark {
        resource_type: "test".into(),
        validate_us: 0.0,
        purify_us: 100.0,
        sample_count: 1,
    };
    assert_eq!(b.overhead_ratio(), 0.0, "zero validate → 0.0 ratio");
}

// ── FJ-2401: Model Integrity Check ─────────────────────────────────

#[test]
fn f_2401_1_model_hash_match_valid() {
    let check = ModelIntegrityCheck::check("llama3", "abc123", "abc123", 1024 * 1024);
    assert!(check.valid, "matching hashes must be valid");
    assert!((check.size_mb() - 1.0).abs() < 0.01);
}
