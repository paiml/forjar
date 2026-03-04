//! Tests for MCP handler implementations (graph, show, status, trace, anomaly).

use pforge_runtime::Handler;

use super::handlers::*;
use super::types::*;
use crate::tripwire::tracer;

#[tokio::test]
async fn test_fj063_graph_handler_mermaid() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  base-dir:\n    type: file\n    path: /opt/app\n    state: directory\n  app-config:\n    type: file\n    path: /opt/app/config.yml\n    content: \"key: value\"\n    depends_on: [base-dir]\n",
    )
    .unwrap();

    let handler = GraphHandler;
    let input = GraphInput {
        path: config_path.to_str().unwrap().to_string(),
        format: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert_eq!(output.format, "mermaid");
    assert!(output.graph.contains("graph LR"));
    assert!(output.graph.contains("base-dir"));
    assert!(output.graph.contains("app-config"));
}

#[tokio::test]
async fn test_fj063_graph_handler_dot() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    provider: apt\n    packages: [nginx]\n",
    )
    .unwrap();

    let handler = GraphHandler;
    let input = GraphInput {
        path: config_path.to_str().unwrap().to_string(),
        format: Some("dot".to_string()),
    };
    let output = handler.handle(input).await.unwrap();
    assert_eq!(output.format, "dot");
    assert!(output.graph.contains("digraph forjar"));
    assert!(output.graph.contains("pkg"));
}

#[tokio::test]
async fn test_fj063_graph_handler_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  step-a:\n    type: package\n    provider: apt\n    packages: [curl]\n  step-b:\n    type: file\n    path: /tmp/b.txt\n    content: hello\n    depends_on: [step-a]\n  step-c:\n    type: service\n    name: myapp\n    depends_on: [step-b]\n",
    )
    .unwrap();

    let handler = GraphHandler;
    let input = GraphInput {
        path: config_path.to_str().unwrap().to_string(),
        format: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert_eq!(output.format, "mermaid");
    // Verify all three resources appear
    assert!(output.graph.contains("step-a"));
    assert!(output.graph.contains("step-b"));
    assert!(output.graph.contains("step-c"));
    // Verify the dependency chain edges: a->b and b->c
    assert!(output.graph.contains("step-a --> step-b"));
    assert!(output.graph.contains("step-b --> step-c"));
}

#[tokio::test]
async fn test_fj063_show_handler_single_resource() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test-file:\n    type: file\n    path: /tmp/test.txt\n    content: hello\n",
    )
    .unwrap();

    let handler = ShowHandler;
    let input = ShowInput {
        path: config_path.to_str().unwrap().to_string(),
        resource: Some("test-file".to_string()),
    };
    let output = handler.handle(input).await.unwrap();
    assert!(output.config.is_object());
}

#[tokio::test]
async fn test_fj063_show_handler_missing_resource() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test-file:\n    type: file\n    path: /tmp/test.txt\n    content: hello\n",
    )
    .unwrap();

    let handler = ShowHandler;
    let input = ShowInput {
        path: config_path.to_str().unwrap().to_string(),
        resource: Some("nonexistent".to_string()),
    };
    let result = handler.handle(input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_fj063_show_handler_full_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &config_path,
        "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test-file:\n    type: file\n    path: /tmp/test.txt\n    content: hello\n",
    )
    .unwrap();

    let handler = ShowHandler;
    let input = ShowInput {
        path: config_path.to_str().unwrap().to_string(),
        resource: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert!(output.config.is_object());
}

#[tokio::test]
async fn test_fj063_status_handler_empty() {
    let dir = tempfile::tempdir().unwrap();
    let handler = StatusHandler;
    let input = StatusInput {
        state_dir: Some(dir.path().to_str().unwrap().to_string()),
        machine: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert!(output.machines.is_empty());
}

#[tokio::test]
async fn test_fj063_status_handler_nonexistent_dir() {
    let handler = StatusHandler;
    let input = StatusInput {
        state_dir: Some("/nonexistent/state/dir".to_string()),
        machine: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert!(output.machines.is_empty());
}

#[tokio::test]
async fn test_fj063_status_handler_with_state() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    // StatusHandler reads .json files from state_dir
    let state_json = serde_json::json!({
        "version": "1.0",
        "machine": "local",
        "resources": {
            "test-pkg": {
                "resource_type": "Package",
                "status": "Converged",
                "hash": "abc123",
                "details": {}
            }
        }
    });
    std::fs::write(
        state_dir.join("local.json"),
        serde_json::to_string_pretty(&state_json).unwrap(),
    )
    .unwrap();

    let handler = StatusHandler;
    let input = StatusInput {
        state_dir: Some(state_dir.to_str().unwrap().to_string()),
        machine: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert_eq!(output.machines.len(), 1);
    assert_eq!(output.machines[0].name, "local");
}

#[tokio::test]
async fn test_fj063_trace_handler_empty() {
    let dir = tempfile::tempdir().unwrap();
    let handler = TraceHandler;
    let input = TraceInput {
        state_dir: Some(dir.path().to_str().unwrap().to_string()),
        machine: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert_eq!(output.trace_count, 0);
    assert!(output.spans.is_empty());
}

#[tokio::test]
async fn test_fj063_trace_handler_with_data() {
    let dir = tempfile::tempdir().unwrap();
    let machine_dir = dir.path().join("web1");
    std::fs::create_dir_all(&machine_dir).unwrap();

    let mut session = tracer::TraceSession::start("r-test");
    session.record_span(
        "nginx-config",
        "file",
        "web1",
        "create",
        std::time::Duration::from_millis(100),
        0,
        Some("blake3:abc"),
    );
    session.record_noop("app-config", "file", "web1");
    tracer::write_trace(dir.path(), "web1", &session).unwrap();

    let handler = TraceHandler;
    let input = TraceInput {
        state_dir: Some(dir.path().to_str().unwrap().to_string()),
        machine: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert_eq!(output.trace_count, 1);
    assert_eq!(output.spans.len(), 2);
    assert_eq!(output.spans[0].name, "apply:nginx-config");
    assert_eq!(output.spans[1].name, "apply:app-config");
}

#[tokio::test]
async fn test_fj063_trace_handler_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    for m in &["web1", "web2"] {
        let machine_dir = dir.path().join(m);
        std::fs::create_dir_all(&machine_dir).unwrap();
        let mut session = tracer::TraceSession::start(&format!("r-{m}"));
        session.record_noop("pkg", "package", m);
        tracer::write_trace(dir.path(), m, &session).unwrap();
    }

    let handler = TraceHandler;
    let input = TraceInput {
        state_dir: Some(dir.path().to_str().unwrap().to_string()),
        machine: Some("web1".to_string()),
    };
    let output = handler.handle(input).await.unwrap();
    assert_eq!(output.spans.len(), 1);
    assert_eq!(output.spans[0].machine, "web1");
}

#[tokio::test]
async fn test_fj063_trace_handler_nonexistent_dir() {
    let handler = TraceHandler;
    let input = TraceInput {
        state_dir: Some("/nonexistent/trace/dir".to_string()),
        machine: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert_eq!(output.trace_count, 0);
    assert!(output.spans.is_empty());
}

#[tokio::test]
async fn test_fj063_anomaly_handler_empty() {
    let dir = tempfile::tempdir().unwrap();
    let handler = AnomalyHandler;
    let input = AnomalyInput {
        state_dir: Some(dir.path().to_str().unwrap().to_string()),
        machine: None,
        min_events: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert_eq!(output.anomaly_count, 0);
    assert!(output.findings.is_empty());
}

#[tokio::test]
async fn test_fj063_anomaly_handler_nonexistent_dir() {
    let handler = AnomalyHandler;
    let input = AnomalyInput {
        state_dir: Some("/nonexistent/anomaly/dir".to_string()),
        machine: None,
        min_events: None,
    };
    let output = handler.handle(input).await.unwrap();
    assert_eq!(output.anomaly_count, 0);
}

#[tokio::test]
async fn test_fj063_anomaly_handler_with_events() {
    let dir = tempfile::tempdir().unwrap();
    let machine_dir = dir.path().join("local");
    std::fs::create_dir_all(&machine_dir).unwrap();

    // Write event log with anomalous data
    let mut events = String::new();
    // Normal resources: 5 converges each
    for resource in &["pkg-a", "pkg-b", "pkg-c", "pkg-d", "pkg-e"] {
        for _ in 0..5 {
            let event = serde_json::json!({
                "ts": "2026-02-25T12:00:00Z",
                "event": "resource_converged",
                "machine": "local",
                "resource": resource,
                "duration_seconds": 0.1,
                "hash": "abc123"
            });
            events.push_str(&serde_json::to_string(&event).unwrap());
            events.push('\n');
        }
    }
    // Anomalous: 500 converges (high churn)
    for _ in 0..500 {
        let event = serde_json::json!({
            "ts": "2026-02-25T12:00:00Z",
            "event": "resource_converged",
            "machine": "local",
            "resource": "churn-pkg",
            "duration_seconds": 0.1,
            "hash": "abc123"
        });
        events.push_str(&serde_json::to_string(&event).unwrap());
        events.push('\n');
    }
    std::fs::write(machine_dir.join("events.jsonl"), events).unwrap();

    let handler = AnomalyHandler;
    let input = AnomalyInput {
        state_dir: Some(dir.path().to_str().unwrap().to_string()),
        machine: None,
        min_events: Some(3),
    };
    let output = handler.handle(input).await.unwrap();
    assert!(output.anomaly_count > 0, "should detect high churn anomaly");
    assert!(
        output
            .findings
            .iter()
            .any(|f| f.resource.contains("churn-pkg")),
        "churn-pkg should be flagged"
    );
}
