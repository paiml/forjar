//! Spec falsification tests: Phase D (Sandboxing & Scoring)
//!
//! Split from tests_falsify_spec_a.rs to stay under 500-line limit.
#![allow(unused_imports)]

use super::purity::PurityLevel;
use super::repro_score::{compute_score, grade, ReproInput};
use super::sandbox::{
    blocks_network, cgroup_path, enforces_fs_isolation, preset_profile, validate_config,
    SandboxConfig, SandboxLevel,
};

/// D-01: SandboxLevel has Full, NetworkOnly, Minimal, None.
#[test]
fn falsify_d01_sandbox_levels_exist() {
    let _full = SandboxLevel::Full;
    let _net = SandboxLevel::NetworkOnly;
    let _min = SandboxLevel::Minimal;
    let _none = SandboxLevel::None;
}

/// D-02: "full" preset — 2048 MB, 4 CPUs, 600s.
#[test]
fn falsify_d02_preset_full() {
    let cfg = preset_profile("full").expect("'full' preset must exist");
    assert_eq!(cfg.level, SandboxLevel::Full);
    assert_eq!(cfg.memory_mb, 2048, "full: 2048 MB");
    assert!((cfg.cpus - 4.0).abs() < f64::EPSILON, "full: 4 CPUs");
    assert_eq!(cfg.timeout, 600, "full: 600s");
}

/// D-03: "network-only" preset — 4096 MB, 8 CPUs, 1200s.
#[test]
fn falsify_d03_preset_network_only() {
    let cfg = preset_profile("network-only").expect("'network-only' preset must exist");
    assert_eq!(cfg.level, SandboxLevel::NetworkOnly);
    assert_eq!(cfg.memory_mb, 4096);
    assert!((cfg.cpus - 8.0).abs() < f64::EPSILON);
    assert_eq!(cfg.timeout, 1200);
}

/// D-04: "minimal" preset — 1024 MB, 2 CPUs, 300s.
#[test]
fn falsify_d04_preset_minimal() {
    let cfg = preset_profile("minimal").expect("'minimal' preset must exist");
    assert_eq!(cfg.level, SandboxLevel::Minimal);
    assert_eq!(cfg.memory_mb, 1024);
    assert!((cfg.cpus - 2.0).abs() < f64::EPSILON);
    assert_eq!(cfg.timeout, 300);
}

/// D-05: "gpu" preset — 16384 MB, 8 CPUs, 3600s.
#[test]
fn falsify_d05_preset_gpu() {
    let cfg = preset_profile("gpu").expect("'gpu' preset must exist");
    assert_eq!(cfg.memory_mb, 16384);
    assert!((cfg.cpus - 8.0).abs() < f64::EPSILON);
    assert_eq!(cfg.timeout, 3600);
}

/// D-06: blocks_network() is true only for Full.
#[test]
fn falsify_d06_network_blocking() {
    assert!(blocks_network(SandboxLevel::Full));
    assert!(!blocks_network(SandboxLevel::NetworkOnly));
    assert!(!blocks_network(SandboxLevel::Minimal));
    assert!(!blocks_network(SandboxLevel::None));
}

/// D-07: enforces_fs_isolation() for Full, NetworkOnly, Minimal but not None.
#[test]
fn falsify_d07_fs_isolation() {
    assert!(enforces_fs_isolation(SandboxLevel::Full));
    assert!(enforces_fs_isolation(SandboxLevel::NetworkOnly));
    assert!(enforces_fs_isolation(SandboxLevel::Minimal));
    assert!(!enforces_fs_isolation(SandboxLevel::None));
}

/// D-08: validate_config() catches memory_mb=0.
#[test]
fn falsify_d08_validate_zero_memory() {
    let cfg = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 0,
        cpus: 4.0,
        timeout: 600,
        bind_mounts: vec![],
        env: vec![],
    };
    let errors = validate_config(&cfg);
    assert!(!errors.is_empty(), "memory_mb=0 must be invalid");
}

/// D-09: Repro score weights: purity 50%, store 30%, lock 20%.
#[test]
fn falsify_d09_repro_score_weights() {
    let inputs = vec![ReproInput {
        name: "x".to_string(),
        purity: PurityLevel::Pure,
        has_store: true,
        has_lock_pin: true,
    }];
    let score = compute_score(&inputs);
    assert!(
        (score.composite - 100.0).abs() < 0.01,
        "all-pure+store+lock must score 100: got {}",
        score.composite
    );
}

/// D-10: Purity score mapping — Pure=100, Pinned=75, Constrained=25, Impure=0.
#[test]
fn falsify_d10_purity_score_mapping() {
    for (purity, expected_composite) in [
        (PurityLevel::Pure, 100.0 * 0.5),
        (PurityLevel::Pinned, 75.0 * 0.5),
        (PurityLevel::Constrained, 25.0 * 0.5),
        (PurityLevel::Impure, 0.0),
    ] {
        let inputs = vec![ReproInput {
            name: "x".to_string(),
            purity,
            has_store: false,
            has_lock_pin: false,
        }];
        let score = compute_score(&inputs);
        assert!(
            (score.composite - expected_composite).abs() < 0.01,
            "{purity:?}: expected composite {expected_composite}, got {}",
            score.composite
        );
    }
}

/// D-11: Grade thresholds — A≥90, B≥75, C≥50, D≥25, F<25.
#[test]
fn falsify_d11_grade_thresholds() {
    assert_eq!(grade(100.0), "A");
    assert_eq!(grade(90.0), "A");
    assert_eq!(grade(89.9), "B");
    assert_eq!(grade(75.0), "B");
    assert_eq!(grade(74.9), "C");
    assert_eq!(grade(50.0), "C");
    assert_eq!(grade(49.9), "D");
    assert_eq!(grade(25.0), "D");
    assert_eq!(grade(24.9), "F");
    assert_eq!(grade(0.0), "F");
}

/// D-12: Empty recipe scores 100 (vacuously pure).
#[test]
fn falsify_d12_empty_recipe_score() {
    let score = compute_score(&[]);
    assert!(
        (score.composite - 100.0).abs() < 0.01,
        "empty recipe must score 100"
    );
}

/// D-13: cgroup_path() generates valid path from store hash.
#[test]
fn falsify_d13_cgroup_path_format() {
    let path = cgroup_path("blake3:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4");
    assert!(path.starts_with("/sys/fs/cgroup/forjar-build-"));
    assert!(path.contains("a1b2c3d4e5f6a1b2"));
}
