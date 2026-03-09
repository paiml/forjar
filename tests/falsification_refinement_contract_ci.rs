//! FJ-043/2203/2400: Refinement types, contract tiers, CI pipeline types.
//! Usage: cargo test --test falsification_refinement_contract_ci

use forjar::core::types::refinement::*;
use forjar::core::types::*;

// ── helpers ──

fn ce(func: &str, tier: VerificationTier) -> ContractEntry {
    ContractEntry {
        function: func.into(),
        module: "m".into(),
        contract_id: None,
        tier,
        verified_by: vec![],
    }
}

fn hi(rtype: &str, tier: VerificationTier, exempt: bool) -> HandlerInvariantStatus {
    HandlerInvariantStatus {
        resource_type: rtype.into(),
        tier,
        exempt,
        exemption_reason: if exempt { Some("reason".into()) } else { None },
    }
}

// ── FJ-043: Port ──

#[test]
fn port_valid() {
    assert_eq!(Port::new(80).unwrap().value(), 80);
    assert_eq!(Port::new(443).unwrap().value(), 443);
    assert_eq!(Port::new(65535).unwrap().value(), 65535);
    assert!(Port::new(1).is_ok());
}

#[test]
fn port_invalid() {
    assert!(Port::new(0).is_err());
}

#[test]
fn port_serde() {
    let p = Port::new(8080).unwrap();
    let json = serde_json::to_string(&p).unwrap();
    let parsed: Port = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.value(), 8080);
}

// ── FJ-043: FileMode ──

#[test]
fn filemode_valid() {
    assert_eq!(FileMode::new(0o644).unwrap().value(), 0o644);
    assert_eq!(FileMode::new(0o755).unwrap().as_octal_string(), "0755");
    assert_eq!(FileMode::new(0).unwrap().as_octal_string(), "0000");
    assert_eq!(FileMode::new(0o777).unwrap().value(), 0o777);
}

#[test]
fn filemode_invalid() {
    assert!(FileMode::new(0o1000).is_err());
}

#[test]
fn filemode_from_str() {
    assert_eq!(FileMode::from_str("644").unwrap().value(), 0o644);
    assert_eq!(FileMode::from_str("755").unwrap().as_octal_string(), "0755");
    assert!(FileMode::from_str("999").is_err());
    assert!(FileMode::from_str("abc").is_err());
}

// ── FJ-043: SemVer ──

#[test]
fn semver_valid() {
    let v = SemVer::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
    assert_eq!(v.to_string(), "1.2.3");
}

#[test]
fn semver_zero() {
    let v = SemVer::parse("0.0.0").unwrap();
    assert_eq!(v.to_string(), "0.0.0");
}

#[test]
fn semver_invalid() {
    assert!(SemVer::parse("1.2").is_err());
    assert!(SemVer::parse("abc").is_err());
    assert!(SemVer::parse("1.2.x").is_err());
}

#[test]
fn semver_serde() {
    let v = SemVer::parse("3.14.159").unwrap();
    let json = serde_json::to_string(&v).unwrap();
    let parsed: SemVer = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.major, 3);
    assert_eq!(parsed.patch, 159);
}

// ── FJ-043: Hostname ──

#[test]
fn hostname_valid() {
    assert_eq!(
        Hostname::new("example.com").unwrap().as_str(),
        "example.com"
    );
    assert!(Hostname::new("a-b.example.com").is_ok());
    assert!(Hostname::new("localhost").is_ok());
    assert!(Hostname::new("a").is_ok());
}

#[test]
fn hostname_invalid() {
    assert!(Hostname::new("").is_err());
    assert!(Hostname::new("-bad.com").is_err());
    assert!(Hostname::new("bad-.com").is_err());
    assert!(Hostname::new("inv@lid.com").is_err());
}

// ── FJ-043: AbsPath ──

#[test]
fn abspath_valid() {
    assert_eq!(
        AbsPath::new("/etc/nginx.conf").unwrap().as_str(),
        "/etc/nginx.conf"
    );
    assert!(AbsPath::new("/").is_ok());
}

#[test]
fn abspath_invalid() {
    assert!(AbsPath::new("relative/path").is_err());
    assert!(AbsPath::new("/has\0null").is_err());
}

// ── FJ-043: ResourceName ──

#[test]
fn resource_name_valid() {
    assert_eq!(
        ResourceName::new("pkg-nginx").unwrap().as_str(),
        "pkg-nginx"
    );
    assert!(ResourceName::new("my_resource").is_ok());
    assert!(ResourceName::new("a").is_ok());
}

#[test]
fn resource_name_invalid() {
    assert!(ResourceName::new("").is_err());
    assert!(ResourceName::new("has space").is_err());
    assert!(ResourceName::new("inv@lid").is_err());
}

// ── FJ-2203: VerificationTier ──

#[test]
fn verification_tier_ordering() {
    assert!(VerificationTier::Unlabeled < VerificationTier::Labeled);
    assert!(VerificationTier::Labeled < VerificationTier::Runtime);
    assert!(VerificationTier::Runtime < VerificationTier::Bounded);
    assert!(VerificationTier::Bounded < VerificationTier::Proved);
    assert!(VerificationTier::Proved < VerificationTier::Structural);
}

#[test]
fn verification_tier_level_and_label() {
    assert_eq!(VerificationTier::Unlabeled.level(), 0);
    assert_eq!(VerificationTier::Structural.level(), 5);
    assert_eq!(VerificationTier::Bounded.label(), "bounded");
    assert_eq!(VerificationTier::Proved.label(), "proved");
}

#[test]
fn verification_tier_display_and_default() {
    assert_eq!(VerificationTier::Bounded.to_string(), "L3 (bounded)");
    assert_eq!(VerificationTier::Structural.to_string(), "L5 (structural)");
    assert_eq!(VerificationTier::default(), VerificationTier::Unlabeled);
}

#[test]
fn verification_tier_serde() {
    for tier in [
        VerificationTier::Unlabeled,
        VerificationTier::Labeled,
        VerificationTier::Runtime,
        VerificationTier::Bounded,
        VerificationTier::Proved,
        VerificationTier::Structural,
    ] {
        let json = serde_json::to_string(&tier).unwrap();
        let parsed: VerificationTier = serde_json::from_str(&json).unwrap();
        assert_eq!(tier, parsed);
    }
}

// ── FJ-2203: ContractEntry ──

#[test]
fn contract_entry_display() {
    let e = ce("hash_desired_state", VerificationTier::Bounded);
    assert_eq!(e.to_string(), "m::hash_desired_state: L3 (bounded)");
}

// ── FJ-2203: HandlerInvariantStatus ──

#[test]
fn handler_invariant_display() {
    assert_eq!(
        hi("file", VerificationTier::Bounded, false).to_string(),
        "file: L3 (bounded)"
    );
}

#[test]
fn handler_invariant_exempt() {
    let s = hi("task", VerificationTier::Unlabeled, true).to_string();
    assert!(s.contains("EXEMPT"));
    assert!(s.contains("reason"));
}

// ── FJ-2203: ContractCoverageReport ──

#[test]
fn coverage_report_histogram() {
    let report = ContractCoverageReport {
        total_functions: 10,
        entries: vec![
            ce("a", VerificationTier::Runtime),
            ce("b", VerificationTier::Runtime),
            ce("c", VerificationTier::Bounded),
        ],
        handler_invariants: vec![],
    };
    let hist = report.histogram();
    assert_eq!(hist[2], 2); // Runtime
    assert_eq!(hist[3], 1); // Bounded
    assert_eq!(hist[0], 0); // Unlabeled
}

#[test]
fn coverage_report_at_or_above() {
    let report = ContractCoverageReport {
        total_functions: 5,
        entries: vec![
            ce("a", VerificationTier::Unlabeled),
            ce("b", VerificationTier::Runtime),
            ce("c", VerificationTier::Proved),
        ],
        handler_invariants: vec![],
    };
    assert_eq!(report.at_or_above(VerificationTier::Runtime), 2);
    assert_eq!(report.at_or_above(VerificationTier::Proved), 1);
    assert_eq!(report.at_or_above(VerificationTier::Structural), 0);
}

#[test]
fn coverage_report_format_summary() {
    let report = ContractCoverageReport {
        total_functions: 3,
        entries: vec![ce("f", VerificationTier::Structural)],
        handler_invariants: vec![hi("file", VerificationTier::Bounded, false)],
    };
    let text = report.format_summary();
    assert!(text.contains("Total functions on critical path: 3"));
    assert!(text.contains("Level 5 (structural):   1"));
    assert!(text.contains("file: L3 (bounded)"));
}

#[test]
fn coverage_report_empty() {
    let report = ContractCoverageReport::default();
    assert_eq!(report.histogram(), [0; 6]);
    assert_eq!(report.at_or_above(VerificationTier::Unlabeled), 0);
}

// ── FJ-2403: ReproBuildConfig ──

#[test]
fn repro_build_config_default() {
    let c = ReproBuildConfig::default();
    assert!(c.locked);
    assert!(c.no_incremental);
    assert!(c.lto);
    assert_eq!(c.codegen_units, 1);
    assert!(c.is_reproducible());
}

#[test]
fn repro_build_config_not_reproducible() {
    let c = ReproBuildConfig {
        locked: false,
        ..Default::default()
    };
    assert!(!c.is_reproducible());
}

#[test]
fn repro_build_config_cargo_args() {
    let args = ReproBuildConfig::default().cargo_args();
    assert!(args.contains(&"--locked".to_string()));
    assert!(args.contains(&"--release".to_string()));
}

#[test]
fn repro_build_config_env_vars() {
    let c = ReproBuildConfig {
        source_date_epoch: Some(1234567890),
        ..Default::default()
    };
    let vars = c.env_vars();
    assert!(vars.iter().any(|(k, _)| k == "CARGO_INCREMENTAL"));
    assert!(vars
        .iter()
        .any(|(k, v)| k == "SOURCE_DATE_EPOCH" && v == "1234567890"));
}

#[test]
fn repro_build_config_serde() {
    let json = serde_json::to_string(&ReproBuildConfig::default()).unwrap();
    let parsed: ReproBuildConfig = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_reproducible());
}

// ── FJ-2403: MsrvCheck ──

#[test]
fn msrv_satisfies() {
    let m = MsrvCheck::new("1.88.0");
    assert!(m.satisfies("1.88.0"));
    assert!(m.satisfies("1.89.0"));
    assert!(m.satisfies("2.0.0"));
    assert!(!m.satisfies("1.87.0"));
    assert!(!m.satisfies("1.87.9"));
}

#[test]
fn msrv_patch_version() {
    let m = MsrvCheck::new("1.88.1");
    assert!(!m.satisfies("1.88.0"));
    assert!(m.satisfies("1.88.1"));
    assert!(m.satisfies("1.88.2"));
}

// ── FJ-2403: FeatureMatrix ──

#[test]
fn feature_matrix_combinations() {
    let combos = FeatureMatrix::new(vec!["a", "b"]).combinations();
    assert_eq!(combos.len(), 4);
    assert!(combos.contains(&vec![]));
    assert!(combos.contains(&vec!["a".to_string()]));
    assert!(combos.contains(&vec!["b".to_string()]));
    assert!(combos.contains(&vec!["a".to_string(), "b".to_string()]));
}

#[test]
fn feature_matrix_single_and_empty() {
    assert_eq!(FeatureMatrix::new(vec!["x"]).combinations().len(), 2);
    assert_eq!(FeatureMatrix::new(vec![]).combinations().len(), 1);
}

#[test]
fn feature_matrix_cargo_commands() {
    let cmds = FeatureMatrix::new(vec!["encryption"]).cargo_commands();
    assert!(cmds
        .iter()
        .any(|c| c.contains("--no-default-features") && !c.contains("--features")));
    assert!(cmds.iter().any(|c| c.contains("--features encryption")));
}

// ── FJ-2400: PurificationBenchmark ──

#[test]
fn purification_benchmark() {
    let b = PurificationBenchmark {
        resource_type: "file".into(),
        validate_us: 50.0,
        purify_us: 150.0,
        sample_count: 100,
    };
    assert!((b.overhead_ratio() - 3.0).abs() < 0.01);
    let s = b.to_string();
    assert!(s.contains("file"));
    assert!(s.contains("3.0x"));
}

#[test]
fn purification_benchmark_zero_validate() {
    let b = PurificationBenchmark {
        resource_type: "x".into(),
        validate_us: 0.0,
        purify_us: 10.0,
        sample_count: 1,
    };
    assert_eq!(b.overhead_ratio(), 0.0);
}

// ── FJ-2401: ModelIntegrityCheck ──

#[test]
fn model_integrity_valid() {
    let c = ModelIntegrityCheck::check("llama3", "abc", "abc", 1024 * 1024);
    assert!(c.valid);
    assert!((c.size_mb() - 1.0).abs() < 0.01);
    assert!(c.to_string().contains("[OK]"));
}

#[test]
fn model_integrity_mismatch() {
    let c = ModelIntegrityCheck::check("gpt2", "abc", "def", 500_000_000);
    assert!(!c.valid);
    let s = c.to_string();
    assert!(s.contains("[MISMATCH]"));
    assert!(s.contains("gpt2"));
}
