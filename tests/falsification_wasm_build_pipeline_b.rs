//! FJ-2402/2403: Popperian falsification for WASM deployment types
//! and CI build pipeline (reproducible builds, MSRV, feature matrix).
//!
//! Each test states conditions under which the build pipeline model
//! would be rejected as invalid.

use forjar::core::types::{
    BuildMetrics, ImageBuildMetrics, LayerMetric, ModelIntegrityCheck, ReproBuildConfig,
    WasmBuildConfig, WasmOptLevel,
};

// ── FJ-2402: WASM Optimization Levels ──────────────────────────────

#[test]
fn f_2401_2_model_hash_mismatch_invalid() {
    let check = ModelIntegrityCheck::check("llama3", "abc123", "def456", 5_000_000_000);
    assert!(!check.valid, "mismatched hashes must be invalid");
    let display = check.to_string();
    assert!(display.contains("[MISMATCH]"));
}

// ── FJ-2402: WASM Build Config Serde ───────────────────────────────

#[test]
fn f_2402_18_build_config_yaml_parse() {
    let yaml = r#"
crate_path: crates/app
target: web
opt_level: minsize
dist_dir: dist
optimize: true
"#;
    let config: WasmBuildConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.crate_path, "crates/app");
    assert_eq!(config.opt_level, WasmOptLevel::MinSize);
    assert!(config.optimize);
}

// ── FJ-2403: Image Build Metrics ───────────────────────────────────

#[test]
fn f_2403_22_image_build_metrics_serde_roundtrip() {
    let m = ImageBuildMetrics {
        tag: "app:v1".into(),
        layer_count: 2,
        total_size: 4096,
        layers: vec![
            LayerMetric {
                file_count: 10,
                uncompressed_size: 8000,
                compressed_size: 3000,
            },
            LayerMetric {
                file_count: 5,
                uncompressed_size: 2000,
                compressed_size: 1096,
            },
        ],
        duration_secs: 2.5,
        built_at: "2026-03-09T00:00:00Z".into(),
        forjar_version: "0.1.0".into(),
        target_arch: "x86_64".into(),
    };
    let json = serde_json::to_string(&m).unwrap();
    let parsed: ImageBuildMetrics = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.tag, "app:v1");
    assert_eq!(parsed.layer_count, 2);
    assert_eq!(parsed.layers.len(), 2);
    assert_eq!(parsed.layers[0].file_count, 10);
}

// ── FJ-2403: Serde Roundtrips ──────────────────────────────────────

#[test]
fn f_cross_1_repro_build_config_serde_roundtrip() {
    let config = ReproBuildConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let parsed: ReproBuildConfig = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_reproducible());
}

#[test]
fn f_cross_2_build_metrics_serde_roundtrip() {
    let mut m = BuildMetrics::current();
    m.binary_size = Some(5_000_000);
    m.locked = true;
    let json = serde_json::to_string(&m).unwrap();
    let parsed: BuildMetrics = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.binary_size, Some(5_000_000));
    assert!(parsed.locked);
}
