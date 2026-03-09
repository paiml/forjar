//! FJ-043/2203/2400: Refinement types, contract tiers, CI pipeline types.
//!
//! Usage: cargo run --example refinement_contract_ci

use forjar::core::types::refinement::*;
use forjar::core::types::*;

fn main() {
    println!("Forjar: Refinement Types, Contracts & CI Pipeline");
    println!("{}", "=".repeat(55));

    // ── Refinement Types ──
    println!("\n[FJ-043] Refinement Types:");
    let port = Port::new(8080).expect("valid port");
    println!("  Port: {} (valid)", port.value());
    println!("  Port 0: {}", Port::new(0).unwrap_err());

    let mode = FileMode::new(0o644).unwrap();
    println!(
        "  FileMode: {} (octal: {})",
        mode.value(),
        mode.as_octal_string()
    );
    let from_str = FileMode::from_str("755").unwrap();
    println!("  FileMode from '755': {}", from_str.as_octal_string());

    let ver = SemVer::parse("1.88.0").unwrap();
    println!("  SemVer: {ver}");

    let host = Hostname::new("web-01.example.com").unwrap();
    println!("  Hostname: {}", host.as_str());

    let path = AbsPath::new("/etc/nginx/nginx.conf").unwrap();
    println!("  AbsPath: {}", path.as_str());

    let name = ResourceName::new("pkg-nginx").unwrap();
    println!("  ResourceName: {}", name.as_str());

    // ── Verification Tiers ──
    println!("\n[FJ-2203] Verification Tiers:");
    for tier in [
        VerificationTier::Unlabeled,
        VerificationTier::Labeled,
        VerificationTier::Runtime,
        VerificationTier::Bounded,
        VerificationTier::Proved,
        VerificationTier::Structural,
    ] {
        println!("  L{}: {} ({})", tier.level(), tier.label(), tier);
    }

    // ── Contract Coverage ──
    println!("\n[FJ-2203] Contract Coverage Report:");
    let report = ContractCoverageReport {
        total_functions: 24,
        entries: vec![
            ContractEntry {
                function: "hash_desired_state".into(),
                module: "core::planner".into(),
                contract_id: Some("blake3-state-v1".into()),
                tier: VerificationTier::Bounded,
                verified_by: vec!["kani::proof_hash_determinism".into()],
            },
            ContractEntry {
                function: "execute_plan".into(),
                module: "core::executor".into(),
                contract_id: None,
                tier: VerificationTier::Runtime,
                verified_by: vec![],
            },
        ],
        handler_invariants: vec![HandlerInvariantStatus {
            resource_type: "file".into(),
            tier: VerificationTier::Bounded,
            exempt: false,
            exemption_reason: None,
        }],
    };
    println!("{}", report.format_summary());

    // ── CI Pipeline Types ──
    println!("[FJ-2403] Reproducible Build Config:");
    let repro = ReproBuildConfig::default();
    println!(
        "  Reproducible: {} | Args: {:?}",
        repro.is_reproducible(),
        repro.cargo_args()
    );

    println!("\n[FJ-2403] MSRV Check:");
    let msrv = MsrvCheck::new("1.88.0");
    for v in ["1.87.0", "1.88.0", "1.89.0"] {
        println!("  {} satisfies 1.88.0: {}", v, msrv.satisfies(v));
    }

    println!("\n[FJ-2403] Feature Matrix:");
    let matrix = FeatureMatrix::new(vec!["encryption", "container-test"]);
    println!("  {} combinations:", matrix.combinations().len());
    for cmd in matrix.cargo_commands() {
        println!("    {cmd}");
    }

    println!("\n[FJ-2400] Purification Benchmark:");
    let bench = PurificationBenchmark {
        resource_type: "file".into(),
        validate_us: 50.0,
        purify_us: 150.0,
        sample_count: 100,
    };
    println!("  {bench}");

    println!("\n[FJ-2401] Model Integrity:");
    let check = ModelIntegrityCheck::check("llama-3.1-8b", "abc123ff", "abc123ff", 4_500_000_000);
    println!("  {check}");

    println!("\n{}", "=".repeat(55));
    println!("All refinement/contract/CI criteria survived.");
}
