//! Tests for MCP handler implementations (validate, plan, drift, lint).

use pforge_runtime::Handler;

use super::handlers::*;
use super::types::*;

#[tokio::test]
async fn test_fj063_validate_handler_valid_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test-pkg:\n    type: package\n    provider: apt\n    packages: [curl]\n",
    )
    .unwrap();

    let handler = ValidateHandler;
    let input = ValidateInput {
        path: config_path.to_str().unwrap().to_string(),
    };
    let output = handler.handle(input).await.unwrap();
    assert!(output.valid);
    assert_eq!(output.resource_count, 1);
    assert_eq!(output.machine_count, 1);
    assert!(output.errors.is_empty());
}

#[tokio::test]
async fn test_fj063_validate_handler_invalid_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(&config_path, "resources: []").unwrap();

    let handler = ValidateHandler;
    let input = ValidateInput {
        path: config_path.to_str().unwrap().to_string(),
    };
    let output = handler.handle(input).await.unwrap();
    assert!(!output.valid);
    assert!(!output.errors.is_empty());
}

#[tokio::test]
async fn test_fj063_validate_handler_missing_file() {
    let handler = ValidateHandler;
    let input = ValidateInput {
        path: "/nonexistent/forjar.yaml".to_string(),
    };
    let output = handler.handle(input).await.unwrap();
    assert!(!output.valid);
}

#[tokio::test]
async fn test_fj063_validate_handler_multiple_resources() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  web-pkg:\n    type: package\n    provider: apt\n    packages: [nginx]\n  web-conf:\n    type: file\n    path: /etc/nginx/nginx.conf\n    content: \"worker_processes 4;\"\n    depends_on: [web-pkg]\n  web-svc:\n    type: service\n    name: nginx\n    depends_on: [web-conf]\n",
    )
    .unwrap();

    let handler = ValidateHandler;
    let input = ValidateInput {
        path: config_path.to_str().unwrap().to_string(),
    };
    let output = handler.handle(input).await.unwrap();
    assert!(output.valid);
    assert_eq!(output.resource_count, 3);
    assert_eq!(output.machine_count, 1);
    assert!(output.errors.is_empty());
}

#[tokio::test]
async fn test_fj063_plan_handler_with_filter() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  pkg-a:\n    type: package\n    provider: apt\n    packages: [curl]\n  pkg-b:\n    type: package\n    provider: apt\n    packages: [wget]\n",
    )
    .unwrap();

    let handler = PlanHandler;
    let input = PlanInput {
        path: config_path.to_str().unwrap().to_string(),
        state_dir: Some(state_dir.to_str().unwrap().to_string()),
        resource: Some("pkg-a".to_string()),
        tag: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert_eq!(output.changes.len(), 1);
    assert_eq!(output.changes[0].resource_id, "pkg-a");
}

#[tokio::test]
async fn test_fj063_plan_handler_all_resources() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  pkg-a:\n    type: package\n    provider: apt\n    packages: [curl]\n  pkg-b:\n    type: package\n    provider: apt\n    packages: [wget]\n",
    )
    .unwrap();

    let handler = PlanHandler;
    let input = PlanInput {
        path: config_path.to_str().unwrap().to_string(),
        state_dir: Some(state_dir.to_str().unwrap().to_string()),
        resource: None,
        tag: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert!(output.to_create >= 2);
}

#[tokio::test]
async fn test_fj063_plan_handler_invalid_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(&config_path, "this is not valid yaml: [[[").unwrap();

    let handler = PlanHandler;
    let input = PlanInput {
        path: config_path.to_str().unwrap().to_string(),
        state_dir: None,
        resource: None,
        tag: None,
    };
    let result = handler.handle(input).await;
    assert!(result.is_err(), "expected error for invalid config");
}

#[tokio::test]
async fn test_fj063_drift_handler_no_state() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test-pkg:\n    type: package\n    provider: apt\n    packages: [curl]\n",
    )
    .unwrap();

    let handler = DriftHandler;
    let input = DriftInput {
        path: config_path.to_str().unwrap().to_string(),
        state_dir: Some(state_dir.to_str().unwrap().to_string()),
        machine: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert!(!output.drifted);
}

#[tokio::test]
async fn test_fj063_drift_handler_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  box-a:\n    hostname: a\n    addr: 10.0.0.1\n  box-b:\n    hostname: b\n    addr: 10.0.0.2\nresources:\n  test-pkg:\n    type: package\n    provider: apt\n    packages: [curl]\n",
    )
    .unwrap();

    let handler = DriftHandler;
    let input = DriftInput {
        path: config_path.to_str().unwrap().to_string(),
        state_dir: Some(state_dir.to_str().unwrap().to_string()),
        machine: Some("box-a".to_string()),
    };
    let output = handler.handle(input).await.unwrap();
    assert!(!output.drifted);
}

#[tokio::test]
async fn test_fj063_lint_handler_unused_machine() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\n  unused-box:\n    hostname: unused\n    addr: 10.0.0.99\nresources:\n  test-pkg:\n    type: package\n    provider: apt\n    packages: [curl]\n",
    )
    .unwrap();

    let handler = LintHandler;
    let input = LintInput {
        path: config_path.to_str().unwrap().to_string(),
    };
    let output = handler.handle(input).await.unwrap();
    assert!(output.warnings.iter().any(|w| w.contains("unused-box")));
}

#[tokio::test]
async fn test_fj063_lint_handler_clean_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  app-dir:\n    type: file\n    machine: local\n    path: /opt/app\n    state: directory\n",
    )
    .unwrap();

    let handler = LintHandler;
    let input = LintInput {
        path: config_path.to_str().unwrap().to_string(),
    };
    let output = handler.handle(input).await.unwrap();
    // No unused-machine warnings (structural lint is clean)
    let structural_warnings: Vec<_> = output
        .warnings
        .iter()
        .filter(|w| w.contains("Machine") || w.contains("[ERROR]"))
        .collect();
    assert!(
        structural_warnings.is_empty(),
        "expected no structural warnings, got: {:?}",
        structural_warnings
    );
    assert_eq!(output.error_count, 0);
}
