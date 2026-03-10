//! FJ-095/113/115: Reproducible builds, Ferrocene certification, flight-grade execution.
//!
//! Popperian rejection criteria for:
//! - FJ-095: Repro build environment checks, Cargo profile checks, source hashing,
//!   binary hashing, report generation, CI snippet
//! - FJ-113: Safety standards, ASIL/DAL levels, toolchain detection, source
//!   compliance, certification evidence, Ferrocene CI config
//! - FJ-115: Flight-grade constants, FgStatus/FgResource/FgPlan, compliance
//!   check, topological sort with bounded iteration
//!
//! Usage: cargo test --test falsification_repro_ferrocene_flight

use forjar::core::ferrocene::{
    check_source_compliance, detect_toolchain, ferrocene_ci_config, generate_evidence, AsilLevel,
    CertificationEvidence, DalLevel, SafetyStandard, ViolationSeverity,
};
use forjar::core::flight_grade::{
    check_compliance, fg_topo_sort, FgPlan, FgResource, FgStatus, MAX_DEPTH, MAX_HASH_LEN,
    MAX_RESOURCES,
};
use forjar::core::repro_build::{
    check_cargo_profile, check_environment, generate_report, hash_binary, hash_source_dir,
    repro_ci_snippet,
};

// ============================================================================
// FJ-095: Reproducible Build — Environment Checks
// ============================================================================

#[test]
fn repro_env_checks_exist() {
    let checks = check_environment();
    assert!(checks.len() >= 3);
}

#[test]
fn repro_env_has_source_date_epoch_check() {
    let checks = check_environment();
    assert!(checks.iter().any(|c| c.name == "SOURCE_DATE_EPOCH"));
}

#[test]
fn repro_env_has_cargo_incremental_check() {
    let checks = check_environment();
    assert!(checks.iter().any(|c| c.name == "CARGO_INCREMENTAL"));
}

#[test]
fn repro_env_has_strip_check() {
    let checks = check_environment();
    assert!(checks.iter().any(|c| c.name == "STRIP_SETTING"));
}

// ============================================================================
// FJ-095: Reproducible Build — Cargo Profile Checks
// ============================================================================

#[test]
fn repro_cargo_profile_all_good() {
    let toml = "[profile.release]\npanic = \"abort\"\nlto = true\ncodegen-units = 1\n";
    let checks = check_cargo_profile(toml);
    assert!(checks.iter().all(|c| c.passed));
}

#[test]
fn repro_cargo_profile_missing_cgu() {
    let toml = "[profile.release]\nlto = true\n";
    let checks = check_cargo_profile(toml);
    let cgu = checks.iter().find(|c| c.name == "codegen-units").unwrap();
    assert!(!cgu.passed);
}

#[test]
fn repro_cargo_profile_missing_lto() {
    let toml = "[profile.release]\ncodegen-units = 1\n";
    let checks = check_cargo_profile(toml);
    let lto = checks.iter().find(|c| c.name == "lto").unwrap();
    assert!(!lto.passed);
}

#[test]
fn repro_cargo_profile_missing_panic_abort() {
    let toml = "[profile.release]\nlto = true\ncodegen-units = 1\n";
    let checks = check_cargo_profile(toml);
    let pa = checks.iter().find(|c| c.name == "panic_abort").unwrap();
    assert!(!pa.passed);
}

#[test]
fn repro_cargo_profile_empty_toml() {
    let checks = check_cargo_profile("[package]\nname = \"test\"\n");
    assert!(checks.iter().any(|c| !c.passed));
}

// ============================================================================
// FJ-095: Source & Binary Hashing
// ============================================================================

#[test]
fn repro_hash_source_dir_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"x\"").unwrap();

    let h1 = hash_source_dir(dir.path()).unwrap();
    let h2 = hash_source_dir(dir.path()).unwrap();
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64); // BLAKE3 hex
}

#[test]
fn repro_hash_source_dir_changes_on_edit() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("lib.rs"), "pub fn a() {}").unwrap();
    let h1 = hash_source_dir(dir.path()).unwrap();

    std::fs::write(dir.path().join("lib.rs"), "pub fn b() {}").unwrap();
    let h2 = hash_source_dir(dir.path()).unwrap();
    assert_ne!(h1, h2);
}

#[test]
fn repro_hash_source_dir_skips_target() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    let h1 = hash_source_dir(dir.path()).unwrap();

    // Adding a file in target/ should NOT change hash
    std::fs::create_dir_all(dir.path().join("target")).unwrap();
    std::fs::write(dir.path().join("target/debug.rs"), "junk").unwrap();
    let h2 = hash_source_dir(dir.path()).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn repro_hash_binary_missing() {
    assert!(hash_binary(std::path::Path::new("/nonexistent/binary")).is_err());
}

#[test]
fn repro_hash_binary_real_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test-bin");
    std::fs::write(&path, b"binary content here").unwrap();
    let hash = hash_binary(&path).unwrap();
    assert_eq!(hash.len(), 64);
}

// ============================================================================
// FJ-095: Report Generation
// ============================================================================

#[test]
fn repro_report_fields() {
    let report = generate_report("src-hash", "bin-hash", "[profile.release]\nlto = true\n");
    assert_eq!(report.source_hash, "src-hash");
    assert_eq!(report.binary_hash, "bin-hash");
    assert_eq!(report.profile, "release");
    assert!(!report.rustflags.is_empty());
}

#[test]
fn repro_report_serde() {
    let report = generate_report("a", "b", "");
    let json = serde_json::to_string(&report).unwrap();
    let parsed: forjar::core::repro_build::ReproReport = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.source_hash, "a");
    assert_eq!(parsed.binary_hash, "b");
}

#[test]
fn repro_ci_snippet_content() {
    let ci = repro_ci_snippet();
    assert!(ci.contains("SOURCE_DATE_EPOCH"));
    assert!(ci.contains("CARGO_INCREMENTAL"));
    assert!(ci.contains("b3sum"));
    assert!(ci.contains("cargo build --release"));
}

// ============================================================================
// FJ-113: Safety Standards Enum
// ============================================================================

#[test]
fn ferrocene_safety_standards_serde() {
    for s in &[
        SafetyStandard::Iso26262,
        SafetyStandard::Do178c,
        SafetyStandard::Iec61508,
        SafetyStandard::En50128,
    ] {
        let json = serde_json::to_string(s).unwrap();
        let parsed: SafetyStandard = serde_json::from_str(&json).unwrap();
        assert_eq!(*s, parsed);
    }
}

#[test]
fn ferrocene_asil_levels() {
    assert_ne!(AsilLevel::QM, AsilLevel::D);
    assert_ne!(AsilLevel::A, AsilLevel::B);
    assert_ne!(AsilLevel::B, AsilLevel::C);
    assert_ne!(AsilLevel::C, AsilLevel::D);
    assert_eq!(AsilLevel::A, AsilLevel::A);
}

#[test]
fn ferrocene_dal_levels() {
    assert_ne!(DalLevel::E, DalLevel::A);
    assert_ne!(DalLevel::D, DalLevel::B);
    assert_eq!(DalLevel::C, DalLevel::C);
}

// ============================================================================
// FJ-113: Toolchain Detection
// ============================================================================

#[test]
fn ferrocene_detect_toolchain() {
    let info = detect_toolchain();
    // Standard rustc, not ferrocene
    assert!(!info.is_ferrocene);
    assert!(
        info.channel == "stable" || info.channel == "nightly",
        "channel: {}",
        info.channel
    );
}

#[test]
fn ferrocene_toolchain_version_not_empty() {
    let info = detect_toolchain();
    assert!(!info.version.is_empty());
}

#[test]
fn ferrocene_toolchain_serde() {
    let info = detect_toolchain();
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("is_ferrocene"));
    assert!(json.contains("channel"));
}

// ============================================================================
// FJ-113: Source Compliance
// ============================================================================

#[test]
fn ferrocene_source_clean() {
    let source = "fn main() {\n    println!(\"hello\");\n}\n";
    let violations = check_source_compliance(source);
    assert!(violations.is_empty());
}

#[test]
fn ferrocene_source_unsafe_block() {
    let source = concat!(
        "fn main() {\n    ",
        "unsa",
        "fe { std::ptr::null::<u8>().read() };\n}\n"
    );
    let violations = check_source_compliance(source);
    assert!(!violations.is_empty());
    assert_eq!(violations[0].severity, ViolationSeverity::Error);
}

#[test]
fn ferrocene_source_forbidden_attribute() {
    let attr = concat!("#![allow(", "unsafe_code", ")]");
    let source = format!("{attr}\nfn main() {{}}\n");
    let violations = check_source_compliance(&source);
    assert!(!violations.is_empty());
    assert!(violations[0].message.contains("Forbidden attribute"));
}

#[test]
fn ferrocene_source_feature_gate() {
    let source = "#![feature(some_thing)]\nfn main() {}\n";
    let violations = check_source_compliance(source);
    assert!(!violations.is_empty());
}

#[test]
fn ferrocene_violation_line_numbers() {
    let source = "fn ok() {}\n// line 2\n#![feature(x)]\n";
    let violations = check_source_compliance(source);
    assert!(!violations.is_empty());
    assert_eq!(violations[0].line, 3);
}

// ============================================================================
// FJ-113: Certification Evidence
// ============================================================================

#[test]
fn ferrocene_generate_evidence_iso26262() {
    let ev = generate_evidence(SafetyStandard::Iso26262, "binhash", "srchash");
    assert_eq!(ev.standard, SafetyStandard::Iso26262);
    assert_eq!(ev.binary_hash, "binhash");
    assert_eq!(ev.source_hash, "srchash");
    assert!(!ev.build_flags.is_empty());
    assert!(!ev.forbidden_features.is_empty());
}

#[test]
fn ferrocene_evidence_do178c() {
    let ev = generate_evidence(SafetyStandard::Do178c, "h1", "h2");
    assert_eq!(ev.standard, SafetyStandard::Do178c);
}

#[test]
fn ferrocene_evidence_serde() {
    let ev = generate_evidence(SafetyStandard::Iso26262, "a", "b");
    let json = serde_json::to_string(&ev).unwrap();
    let parsed: CertificationEvidence = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.binary_hash, "a");
    assert_eq!(parsed.standard, SafetyStandard::Iso26262);
}

#[test]
fn ferrocene_evidence_compliance_checks() {
    let ev = generate_evidence(SafetyStandard::Iso26262, "a", "b");
    assert!(ev.compliance_checks.contains_key("no_unsafe_code"));
    assert!(ev.compliance_checks.contains_key("panic_abort"));
    assert!(ev.compliance_checks.contains_key("overflow_checks"));
    assert!(ev.compliance_checks.contains_key("deterministic_build"));
}

#[test]
fn ferrocene_ci_config_content() {
    let cfg = ferrocene_ci_config();
    assert!(cfg.contains("ferrocene"));
    assert!(cfg.contains("panic=abort"));
    assert!(cfg.contains("overflow-checks=on"));
    assert!(cfg.contains("certified-build"));
}

// ============================================================================
// FJ-115: Flight-Grade Constants
// ============================================================================

#[test]
fn flight_max_resources() {
    assert_eq!(MAX_RESOURCES, 256);
}

#[test]
fn flight_max_depth() {
    assert_eq!(MAX_DEPTH, 32);
}

#[test]
fn flight_max_hash_len() {
    assert_eq!(MAX_HASH_LEN, 64);
}

// ============================================================================
// FJ-115: FgStatus
// ============================================================================

#[test]
fn flight_status_equality() {
    assert_eq!(FgStatus::Pending, FgStatus::Pending);
    assert_eq!(FgStatus::Converged, FgStatus::Converged);
    assert_eq!(FgStatus::Failed, FgStatus::Failed);
    assert_eq!(FgStatus::Skipped, FgStatus::Skipped);
}

#[test]
fn flight_status_inequality() {
    assert_ne!(FgStatus::Pending, FgStatus::Converged);
    assert_ne!(FgStatus::Converged, FgStatus::Failed);
    assert_ne!(FgStatus::Failed, FgStatus::Skipped);
}

// ============================================================================
// FJ-115: FgResource
// ============================================================================

#[test]
fn flight_resource_empty() {
    let r = FgResource::empty();
    assert_eq!(r.id, 0);
    assert_eq!(r.status, FgStatus::Pending);
    assert_eq!(r.hash, [0u8; 32]);
    assert_eq!(r.dep_count, 0);
}

#[test]
fn flight_resource_clone() {
    let mut r = FgResource::empty();
    r.id = 42;
    r.status = FgStatus::Converged;
    let c = r;
    assert_eq!(c.id, 42);
    assert_eq!(c.status, FgStatus::Converged);
}

// ============================================================================
// FJ-115: FgPlan
// ============================================================================

#[test]
fn flight_plan_empty() {
    let p = FgPlan::empty();
    assert_eq!(p.count, 0);
    assert_eq!(p.order_len, 0);
}

// ============================================================================
// FJ-115: Compliance Check
// ============================================================================

#[test]
fn flight_compliance_within_limits() {
    let report = check_compliance(100, 10);
    assert!(report.compliant);
    assert!(report.no_dynamic_alloc);
    assert!(report.bounded_loops);
    assert!(report.no_panic_paths);
    assert!(report.deterministic_memory);
}

#[test]
fn flight_compliance_at_limits() {
    let report = check_compliance(MAX_RESOURCES, MAX_DEPTH);
    assert!(report.compliant);
}

#[test]
fn flight_compliance_exceeds_resources() {
    let report = check_compliance(MAX_RESOURCES + 1, MAX_DEPTH);
    assert!(!report.compliant);
    assert!(!report.deterministic_memory);
}

#[test]
fn flight_compliance_exceeds_depth() {
    let report = check_compliance(MAX_RESOURCES, MAX_DEPTH + 1);
    assert!(!report.compliant);
}

#[test]
fn flight_compliance_serde() {
    let r = check_compliance(10, 5);
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"compliant\":true"));
}

// ============================================================================
// FJ-115: Topological Sort
// ============================================================================

#[test]
fn flight_topo_sort_empty() {
    let mut plan = FgPlan::empty();
    assert!(fg_topo_sort(&mut plan).is_ok());
    assert_eq!(plan.order_len, 0);
}

#[test]
fn flight_topo_sort_single() {
    let mut plan = FgPlan::empty();
    plan.resources[0] = FgResource::empty();
    plan.count = 1;
    assert!(fg_topo_sort(&mut plan).is_ok());
    assert_eq!(plan.order_len, 1);
    assert_eq!(plan.order[0], 0);
}

#[test]
fn flight_topo_sort_chain_a_then_b() {
    let mut plan = FgPlan::empty();
    // A (no deps)
    plan.resources[0] = FgResource::empty();
    plan.resources[0].id = 0;
    // B depends on A
    plan.resources[1] = FgResource::empty();
    plan.resources[1].id = 1;
    plan.resources[1].deps[0] = 0;
    plan.resources[1].dep_count = 1;
    plan.count = 2;

    assert!(fg_topo_sort(&mut plan).is_ok());
    assert_eq!(plan.order_len, 2);
    assert_eq!(plan.order[0], 0); // A first
    assert_eq!(plan.order[1], 1); // B second
}
