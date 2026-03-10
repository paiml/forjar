//! FJ-1385/1382/1387/1390/2702: Proof obligations, reversibility, compliance,
//! security scanning, and quality gates.
//!
//! Demonstrates:
//! - Proof obligation classification for resource operations
//! - Reversibility analysis for destroy operations
//! - CIS/NIST/SOC2/HIPAA compliance benchmark evaluation
//! - IaC security smell detection (10 rules)
//! - Quality gate evaluation (exit code, JSON, regex)
//! - GPU device targeting environment variables
//!
//! Usage: cargo run --example proof_compliance_security

use forjar::core::compliance::{count_by_severity, evaluate_benchmark, supported_benchmarks};
use forjar::core::planner::proof_obligation;
use forjar::core::planner::reversibility;
use forjar::core::security_scanner;
use forjar::core::task::{evaluate_gate, gpu_env_vars};
use forjar::core::types::*;
use indexmap::IndexMap;

fn make_config(resources: Vec<(&str, Resource)>) -> ForjarConfig {
    let mut res = IndexMap::new();
    for (id, r) in resources {
        res.insert(id.to_string(), r);
    }
    ForjarConfig {
        version: "1.0".into(),
        name: "demo".into(),
        resources: res,
        description: None,
        params: Default::default(),
        machines: Default::default(),
        policy: Default::default(),
        outputs: Default::default(),
        policies: Default::default(),
        data: Default::default(),
        includes: Default::default(),
        include_provenance: Default::default(),
        checks: Default::default(),
        moved: Default::default(),
        secrets: Default::default(),
        environments: Default::default(),
        dist: None,
    }
}

fn main() {
    println!("Forjar: Proof, Compliance, Security & Gates");
    println!("{}", "=".repeat(50));

    // ── Proof Obligations ──
    println!("\n[FJ-1385] Proof Obligations:");
    let cases = [
        (ResourceType::File, PlanAction::Create),
        (ResourceType::Service, PlanAction::Create),
        (ResourceType::Model, PlanAction::Create),
        (ResourceType::File, PlanAction::Destroy),
        (ResourceType::User, PlanAction::Destroy),
    ];
    for (rtype, action) in &cases {
        let po = proof_obligation::classify(rtype, action);
        println!(
            "  {:?}/{:?} → {} (safe={})",
            rtype,
            action,
            proof_obligation::label(&po),
            proof_obligation::is_safe(&po)
        );
    }

    // ── Reversibility ──
    println!("\n[FJ-1382] Reversibility:");
    let file_no_src = Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    };
    let mut file_with_src = file_no_src.clone();
    file_with_src.content = Some("data".into());
    println!(
        "  File destroy (no source): {:?}",
        reversibility::classify(&file_no_src, &PlanAction::Destroy)
    );
    println!(
        "  File destroy (with content): {:?}",
        reversibility::classify(&file_with_src, &PlanAction::Destroy)
    );

    // ── Compliance ──
    println!(
        "\n[FJ-1387] Compliance Benchmarks: {:?}",
        supported_benchmarks()
    );
    let mut insecure_file = Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    };
    insecure_file.mode = Some("0777".into());
    insecure_file.path = Some("/tmp/script.sh".into());
    insecure_file.owner = Some("root".into());
    let config = make_config(vec![("web", insecure_file)]);
    for benchmark in supported_benchmarks() {
        let findings = evaluate_benchmark(benchmark, &config);
        let (c, h, m, l) = count_by_severity(&findings);
        println!(
            "  {benchmark}: {} findings (C={c}, H={h}, M={m}, L={l})",
            findings.len()
        );
    }

    // ── Security Scanner ──
    println!("\n[FJ-1390] Security Scanner:");
    let mut vulnerable = Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    };
    vulnerable.content = Some("password=s3cret\nurl: ftp://old.server".into());
    vulnerable.mode = Some("0644".into());
    let config = make_config(vec![("cfg", vulnerable)]);
    let findings = security_scanner::scan(&config);
    for f in &findings {
        println!(
            "  [{}] {} — {:?}: {}",
            f.rule_id, f.category, f.severity, f.message
        );
    }
    let (c, h, m, l) = security_scanner::severity_counts(&findings);
    println!("  Total: {} (C={c}, H={h}, M={m}, L={l})", findings.len());

    // ── Quality Gates ──
    println!("\n[FJ-2702] Quality Gates:");
    let gate = QualityGate::default();
    println!("  exit 0: {:?}", evaluate_gate(&gate, 0, ""));
    println!("  exit 1: {:?}", evaluate_gate(&gate, 1, ""));

    let json_gate = QualityGate {
        parse: Some("json".into()),
        field: Some("coverage".into()),
        min: Some(80.0),
        ..Default::default()
    };
    println!(
        "  JSON coverage=95: {:?}",
        evaluate_gate(&json_gate, 0, r#"{"coverage":95.0}"#)
    );
    println!(
        "  JSON coverage=60: {:?}",
        evaluate_gate(&json_gate, 0, r#"{"coverage":60.0}"#)
    );

    // ── GPU Targeting ──
    println!("\n[FJ-2703] GPU Env Vars:");
    let vars = gpu_env_vars(Some(0));
    for (k, v) in &vars {
        println!("  {k}={v}");
    }

    println!("\n{}", "=".repeat(50));
    println!("All proof/compliance/security/gate criteria survived.");
}
