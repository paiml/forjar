//! Tests for MCP registry, dispatch, and schema export.

use super::registry::*;
use super::types::*;

#[test]
fn test_fj063_build_registry_has_all_tools() {
    let registry = build_registry();
    assert_eq!(registry.len(), 9);
    assert!(registry.has_handler("forjar_validate"));
    assert!(registry.has_handler("forjar_plan"));
    assert!(registry.has_handler("forjar_drift"));
    assert!(registry.has_handler("forjar_lint"));
    assert!(registry.has_handler("forjar_graph"));
    assert!(registry.has_handler("forjar_show"));
    assert!(registry.has_handler("forjar_status"));
    assert!(registry.has_handler("forjar_trace"));
    assert!(registry.has_handler("forjar_anomaly"));
}

#[test]
fn test_fj063_build_registry_no_unknown_tools() {
    let registry = build_registry();
    assert!(!registry.has_handler("forjar_apply"));
    assert!(!registry.has_handler("nonexistent"));
}

#[test]
fn test_fj063_forge_config_metadata() {
    let config = super::registry::build_forge_config_for_test();
    assert_eq!(config.forge.name, "forjar-mcp");
    assert_eq!(config.tools.len(), 9);
}

#[test]
fn test_fj063_forge_config_tool_names() {
    let config = super::registry::build_forge_config_for_test();
    let names: Vec<&str> = config.tools.iter().map(|t| t.name()).collect();
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

#[tokio::test]
async fn test_fj063_registry_dispatch_validate() {
    let registry = build_registry();
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test-pkg:\n    type: package\n    provider: apt\n    packages: [git]\n",
    )
    .unwrap();

    let input = serde_json::json!({
        "path": config_path.to_str().unwrap()
    });
    let result = registry
        .dispatch("forjar_validate", &serde_json::to_vec(&input).unwrap())
        .await;
    assert!(result.is_ok());
    let output: ValidateOutput = serde_json::from_slice(&result.unwrap()).unwrap();
    assert!(output.valid);
}

#[tokio::test]
async fn test_fj063_registry_dispatch_unknown_tool() {
    let registry = build_registry();
    let result = registry.dispatch("nonexistent", b"{}").await;
    assert!(result.is_err());
}

// ── FJ-142: Schema export tests ──────────────────────────────────

#[test]
fn test_fj142_export_schema_has_all_tools() {
    let schema = export_schema();
    assert_eq!(schema["tool_count"], 9);
    let tools = schema["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 9);
}

#[test]
fn test_fj142_export_schema_tool_names() {
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
fn test_fj142_export_schema_has_input_output() {
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
    }
}

#[test]
fn test_fj142_export_schema_metadata() {
    let schema = export_schema();
    assert_eq!(schema["schema_version"], "1.0");
    assert_eq!(schema["server"], "forjar-mcp");
    assert_eq!(schema["version"], env!("CARGO_PKG_VERSION"));
}
