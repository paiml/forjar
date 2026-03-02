//! Tests for FJ-1351: Contract coverage checker.

use super::contract_coverage::{
    coverage_report, read_binding_registry, scan_contracts_dir, BindingEntry, BindingRegistry,
    ContractStatus,
};
use super::hf_config::KernelRequirement;

fn sample_registry() -> BindingRegistry {
    BindingRegistry {
        version: "1.0".to_string(),
        target_crate: "trueno".to_string(),
        bindings: vec![
            BindingEntry {
                contract: "rmsnorm-kernel-v1".to_string(),
                equation: "EQ-RMSNORM-01".to_string(),
                status: "implemented".to_string(),
            },
            BindingEntry {
                contract: "silu-kernel-v1".to_string(),
                equation: "EQ-SILU-01".to_string(),
                status: "implemented".to_string(),
            },
            BindingEntry {
                contract: "rope-kernel-v1".to_string(),
                equation: "EQ-ROPE-01".to_string(),
                status: "partial".to_string(),
            },
            BindingEntry {
                contract: "softmax-kernel-v1".to_string(),
                equation: "EQ-SOFTMAX-01".to_string(),
                status: "implemented".to_string(),
            },
        ],
    }
}

fn sample_requirements() -> Vec<KernelRequirement> {
    vec![
        KernelRequirement {
            op: "rmsnorm".to_string(),
            contract: "rmsnorm-kernel-v1".to_string(),
        },
        KernelRequirement {
            op: "silu".to_string(),
            contract: "silu-kernel-v1".to_string(),
        },
        KernelRequirement {
            op: "rope".to_string(),
            contract: "rope-kernel-v1".to_string(),
        },
        KernelRequirement {
            op: "softmax".to_string(),
            contract: "softmax-kernel-v1".to_string(),
        },
        KernelRequirement {
            op: "matmul".to_string(),
            contract: "matmul-kernel-v1".to_string(),
        },
    ]
}

#[test]
fn test_fj1351_parse_binding_yaml() {
    let yaml = r#"
version: "1.0"
target_crate: trueno
bindings:
  - contract: rmsnorm-kernel-v1
    equation: EQ-RMSNORM-01
    status: implemented
  - contract: silu-kernel-v1
    equation: EQ-SILU-01
    status: partial
"#;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("binding.yaml");
    std::fs::write(&path, yaml).unwrap();

    let registry = read_binding_registry(&path).unwrap();
    assert_eq!(registry.version, "1.0");
    assert_eq!(registry.target_crate, "trueno");
    assert_eq!(registry.bindings.len(), 2);
    assert_eq!(registry.bindings[0].contract, "rmsnorm-kernel-v1");
    assert_eq!(registry.bindings[0].status, "implemented");
    assert_eq!(registry.bindings[1].status, "partial");
}

#[test]
fn test_fj1351_parse_binding_file_not_found() {
    let result = read_binding_registry(std::path::Path::new("/no/such/binding.yaml"));
    assert!(result.is_err());
}

#[test]
fn test_fj1351_scan_contracts_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("rmsnorm-kernel-v1.yaml"), "test").unwrap();
    std::fs::write(dir.path().join("silu-kernel-v1.yaml"), "test").unwrap();
    std::fs::write(dir.path().join("notes.txt"), "not yaml").unwrap();

    let names = scan_contracts_dir(dir.path()).unwrap();
    assert_eq!(names, vec!["rmsnorm-kernel-v1", "silu-kernel-v1"]);
}

#[test]
fn test_fj1351_scan_contracts_dir_empty() {
    let dir = tempfile::tempdir().unwrap();
    let names = scan_contracts_dir(dir.path()).unwrap();
    assert!(names.is_empty());
}

#[test]
fn test_fj1351_scan_contracts_dir_not_found() {
    let result = scan_contracts_dir(std::path::Path::new("/no/such/dir"));
    assert!(result.is_err());
}

#[test]
fn test_fj1351_coverage_report_mixed() {
    let registry = sample_registry();
    let required = sample_requirements();
    let available = vec![
        "rmsnorm-kernel-v1".to_string(),
        "silu-kernel-v1".to_string(),
        "softmax-kernel-v1".to_string(),
    ];

    let report = coverage_report("llama", &required, &registry, &available);
    assert_eq!(report.model_type, "llama");
    assert_eq!(report.total_required, 5);
    assert_eq!(report.covered, 3); // rmsnorm, silu, softmax
    assert_eq!(report.missing, 2); // rope (partial), matmul (not in registry)
    assert!((report.coverage_pct - 60.0).abs() < 0.01);

    assert_eq!(
        report.contracts["rmsnorm-kernel-v1"],
        ContractStatus::Implemented
    );
    assert_eq!(
        report.contracts["silu-kernel-v1"],
        ContractStatus::Implemented
    );
    assert_eq!(report.contracts["rope-kernel-v1"], ContractStatus::Partial);
    assert_eq!(
        report.contracts["softmax-kernel-v1"],
        ContractStatus::Implemented
    );
    assert_eq!(
        report.contracts["matmul-kernel-v1"],
        ContractStatus::Missing
    );
}

#[test]
fn test_fj1351_coverage_report_full() {
    let registry = BindingRegistry {
        version: "1.0".to_string(),
        target_crate: "trueno".to_string(),
        bindings: vec![
            BindingEntry {
                contract: "rmsnorm-kernel-v1".to_string(),
                equation: "EQ-01".to_string(),
                status: "implemented".to_string(),
            },
            BindingEntry {
                contract: "silu-kernel-v1".to_string(),
                equation: "EQ-02".to_string(),
                status: "implemented".to_string(),
            },
        ],
    };
    let required = vec![
        KernelRequirement {
            op: "rmsnorm".to_string(),
            contract: "rmsnorm-kernel-v1".to_string(),
        },
        KernelRequirement {
            op: "silu".to_string(),
            contract: "silu-kernel-v1".to_string(),
        },
    ];
    let available = vec![
        "rmsnorm-kernel-v1".to_string(),
        "silu-kernel-v1".to_string(),
    ];

    let report = coverage_report("test", &required, &registry, &available);
    assert_eq!(report.covered, 2);
    assert_eq!(report.missing, 0);
    assert!((report.coverage_pct - 100.0).abs() < 0.01);
}

#[test]
fn test_fj1351_coverage_report_zero() {
    let registry = BindingRegistry {
        version: "1.0".to_string(),
        target_crate: "trueno".to_string(),
        bindings: vec![],
    };
    let required = vec![
        KernelRequirement {
            op: "rmsnorm".to_string(),
            contract: "rmsnorm-kernel-v1".to_string(),
        },
        KernelRequirement {
            op: "silu".to_string(),
            contract: "silu-kernel-v1".to_string(),
        },
    ];
    let available: Vec<String> = vec![];

    let report = coverage_report("test", &required, &registry, &available);
    assert_eq!(report.covered, 0);
    assert_eq!(report.missing, 2);
    assert!((report.coverage_pct - 0.0).abs() < 0.01);
}

#[test]
fn test_fj1351_coverage_report_empty_requirements() {
    let registry = sample_registry();
    let report = coverage_report("test", &[], &registry, &[]);
    assert_eq!(report.total_required, 0);
    assert_eq!(report.covered, 0);
    assert_eq!(report.missing, 0);
    assert!((report.coverage_pct - 100.0).abs() < 0.01);
}

#[test]
fn test_fj1351_coverage_deterministic_ordering() {
    let registry = sample_registry();
    let required = sample_requirements();
    let available = vec![
        "rmsnorm-kernel-v1".to_string(),
        "silu-kernel-v1".to_string(),
        "softmax-kernel-v1".to_string(),
    ];

    let report1 = coverage_report("llama", &required, &registry, &available);
    let report2 = coverage_report("llama", &required, &registry, &available);

    let keys1: Vec<&String> = report1.contracts.keys().collect();
    let keys2: Vec<&String> = report2.contracts.keys().collect();
    assert_eq!(keys1, keys2);
}

#[test]
fn test_fj1351_implemented_without_file_is_missing() {
    // Contract listed as "implemented" in registry but no .yaml file → Missing
    let registry = BindingRegistry {
        version: "1.0".to_string(),
        target_crate: "trueno".to_string(),
        bindings: vec![BindingEntry {
            contract: "rmsnorm-kernel-v1".to_string(),
            equation: "EQ-01".to_string(),
            status: "implemented".to_string(),
        }],
    };
    let required = vec![KernelRequirement {
        op: "rmsnorm".to_string(),
        contract: "rmsnorm-kernel-v1".to_string(),
    }];
    // available is empty — no .yaml file found
    let report = coverage_report("test", &required, &registry, &[]);
    assert_eq!(
        report.contracts["rmsnorm-kernel-v1"],
        ContractStatus::Missing
    );
    assert_eq!(report.covered, 0);
}

#[test]
fn test_fj1351_scan_and_report_integration() {
    let dir = tempfile::tempdir().unwrap();
    let contracts_dir = dir.path().join("contracts");
    std::fs::create_dir(&contracts_dir).unwrap();
    std::fs::write(contracts_dir.join("rmsnorm-kernel-v1.yaml"), "stub").unwrap();
    std::fs::write(contracts_dir.join("silu-kernel-v1.yaml"), "stub").unwrap();

    let available = scan_contracts_dir(&contracts_dir).unwrap();
    let registry = sample_registry();
    let required = sample_requirements();

    let report = coverage_report("llama", &required, &registry, &available);
    assert_eq!(
        report.contracts["rmsnorm-kernel-v1"],
        ContractStatus::Implemented
    );
    assert_eq!(
        report.contracts["silu-kernel-v1"],
        ContractStatus::Implemented
    );
}
