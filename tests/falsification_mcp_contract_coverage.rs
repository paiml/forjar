//! FJ-063/1351: MCP schema export, registry, and kernel contract coverage.
//!
//! Popperian rejection criteria for:
//! - FJ-063: export_schema (tool count, schema structure, version)
//! - FJ-063: build_registry (handler registration)
//! - FJ-1351: coverage_report (implemented, partial, missing classification)
//! - FJ-1351: coverage_report (edge cases: empty required, zero coverage, full coverage)
//!
//! Usage: cargo test --test falsification_mcp_contract_coverage

use forjar::core::store::contract_coverage::{
    coverage_report, BindingEntry, BindingRegistry, ContractStatus,
};
use forjar::core::store::hf_config::KernelRequirement;
use forjar::mcp::{build_registry, export_schema};

// ============================================================================
// FJ-063: export_schema
// ============================================================================

#[test]
fn schema_has_correct_tool_count() {
    let schema = export_schema();
    let tools = schema["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 9);
    assert_eq!(schema["tool_count"], 9);
}

#[test]
fn schema_includes_all_tool_names() {
    let schema = export_schema();
    let tools = schema["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"forjar_validate"));
    assert!(names.contains(&"forjar_plan"));
    assert!(names.contains(&"forjar_drift"));
    assert!(names.contains(&"forjar_lint"));
    assert!(names.contains(&"forjar_graph"));
    assert!(names.contains(&"forjar_show"));
    assert!(names.contains(&"forjar_status"));
    assert!(names.contains(&"forjar_trace"));
    assert!(names.contains(&"forjar_anomaly"));
}

#[test]
fn schema_tools_have_schemas() {
    let schema = export_schema();
    let tools = schema["tools"].as_array().unwrap();
    for tool in tools {
        assert!(
            tool["input_schema"].is_object(),
            "{} missing input_schema",
            tool["name"]
        );
        assert!(
            tool["output_schema"].is_object(),
            "{} missing output_schema",
            tool["name"]
        );
        assert!(!tool["description"].as_str().unwrap().is_empty());
    }
}

#[test]
fn schema_version_and_server() {
    let schema = export_schema();
    assert_eq!(schema["schema_version"], "1.0");
    assert_eq!(schema["server"], "forjar-mcp");
    assert!(schema["version"].as_str().unwrap().contains('.'));
}

// ============================================================================
// FJ-063: build_registry
// ============================================================================

#[test]
fn registry_has_handlers() {
    let registry = build_registry();
    assert!(registry.has_handler("forjar_validate"));
    assert!(registry.has_handler("forjar_plan"));
    assert!(registry.has_handler("forjar_drift"));
    assert!(registry.has_handler("forjar_lint"));
    assert!(registry.has_handler("forjar_graph"));
    assert!(registry.has_handler("forjar_anomaly"));
}

#[test]
fn registry_missing_handler_returns_false() {
    let registry = build_registry();
    assert!(!registry.has_handler("nonexistent_tool"));
}

// ============================================================================
// FJ-1351: coverage_report — full coverage
// ============================================================================

fn test_registry() -> BindingRegistry {
    BindingRegistry {
        version: "1.0".into(),
        target_crate: "forjar-kernels".into(),
        bindings: vec![
            BindingEntry {
                contract: "softmax-kernel-v1".into(),
                equation: "EQ-SOFTMAX-01".into(),
                status: "implemented".into(),
            },
            BindingEntry {
                contract: "matmul-kernel-v1".into(),
                equation: "EQ-MATMUL-01".into(),
                status: "implemented".into(),
            },
            BindingEntry {
                contract: "rope-kernel-v1".into(),
                equation: "EQ-ROPE-01".into(),
                status: "partial".into(),
            },
        ],
    }
}

#[test]
fn coverage_all_implemented() {
    let registry = test_registry();
    let required = vec![
        KernelRequirement {
            op: "softmax".into(),
            contract: "softmax-kernel-v1".into(),
        },
        KernelRequirement {
            op: "matmul".into(),
            contract: "matmul-kernel-v1".into(),
        },
    ];
    let available = vec!["softmax-kernel-v1".into(), "matmul-kernel-v1".into()];
    let report = coverage_report("llama", &required, &registry, &available);
    assert_eq!(report.model_type, "llama");
    assert_eq!(report.total_required, 2);
    assert_eq!(report.covered, 2);
    assert_eq!(report.missing, 0);
    assert!((report.coverage_pct - 100.0).abs() < 0.01);
    assert_eq!(
        report.contracts["softmax-kernel-v1"],
        ContractStatus::Implemented
    );
}

#[test]
fn coverage_partial_counts_as_not_covered() {
    let registry = test_registry();
    let required = vec![KernelRequirement {
        op: "rope".into(),
        contract: "rope-kernel-v1".into(),
    }];
    let available = vec!["rope-kernel-v1".into()];
    let report = coverage_report("llama", &required, &registry, &available);
    assert_eq!(report.covered, 0);
    assert_eq!(report.missing, 1);
    assert_eq!(report.contracts["rope-kernel-v1"], ContractStatus::Partial);
}

#[test]
fn coverage_missing_not_in_registry() {
    let registry = test_registry();
    let required = vec![KernelRequirement {
        op: "gelu".into(),
        contract: "gelu-kernel-v1".into(),
    }];
    let report = coverage_report("gpt2", &required, &registry, &[]);
    assert_eq!(report.covered, 0);
    assert_eq!(report.missing, 1);
    assert_eq!(report.contracts["gelu-kernel-v1"], ContractStatus::Missing);
}

#[test]
fn coverage_implemented_but_no_file() {
    let registry = test_registry();
    let required = vec![KernelRequirement {
        op: "softmax".into(),
        contract: "softmax-kernel-v1".into(),
    }];
    // Registry says "implemented" but not in available list
    let report = coverage_report("llama", &required, &registry, &[]);
    assert_eq!(report.covered, 0);
    assert_eq!(
        report.contracts["softmax-kernel-v1"],
        ContractStatus::Missing
    );
}

#[test]
fn coverage_empty_required() {
    let registry = test_registry();
    let report = coverage_report("empty", &[], &registry, &[]);
    assert_eq!(report.total_required, 0);
    assert_eq!(report.covered, 0);
    assert_eq!(report.missing, 0);
    assert!((report.coverage_pct - 100.0).abs() < 0.01);
}

#[test]
fn coverage_mixed() {
    let registry = test_registry();
    let required = vec![
        KernelRequirement {
            op: "softmax".into(),
            contract: "softmax-kernel-v1".into(),
        },
        KernelRequirement {
            op: "rope".into(),
            contract: "rope-kernel-v1".into(),
        },
        KernelRequirement {
            op: "gelu".into(),
            contract: "gelu-kernel-v1".into(),
        },
    ];
    let available = vec!["softmax-kernel-v1".into()];
    let report = coverage_report("mixed", &required, &registry, &available);
    assert_eq!(report.total_required, 3);
    assert_eq!(report.covered, 1);
    assert_eq!(report.missing, 2);
    assert!((report.coverage_pct - 33.33).abs() < 0.5);
}
