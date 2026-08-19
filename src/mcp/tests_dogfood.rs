//! Regression tests for the three MCP defects found by dogfooding forjar
//! 1.12.3 (GH #208 — #214 selector scope, #212 machine output malformed).
//!
//! Verified RED against the unfixed handlers:
//!   * `forjar_plan {resource: "hello-file"}` -> 1 change alongside to_create: 2
//!   * `forjar_plan {resource: "totally-bogus"}` -> changes: [], to_create: 2, no error
//!   * `forjar_show {}` -> `"{{params.sandbox}}/hello.txt"`
//!   * `forjar_graph {format: "BOGUS"}` -> Mermaid source labelled `"format":"BOGUS"`

use pforge_runtime::Handler;

use super::handlers::*;
use super::types::*;

/// Two resources, one templated path, so a selector that reaches only the body
/// is visible in the counters and an unexpanded template is visible in `show`.
fn write_sandbox(dir: &std::path::Path) -> String {
    let path = dir.join("forjar.yaml");
    let sandbox = dir.display();
    std::fs::write(
        &path,
        format!(
            "version: \"1.0\"\n\
             name: dogfood-sandbox\n\
             params:\n  sandbox: {sandbox}\n\
             machines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\n\
             resources:\n\
             \x20 hello-file:\n    type: file\n    machine: local\n    path: \"{{{{params.sandbox}}}}/hello.txt\"\n    content: \"hello\\n\"\n\
             \x20 marker-task:\n    type: task\n    machine: local\n    command: \"true\"\n    depends_on: [hello-file]\n"
        ),
    )
    .expect("write config");
    path.to_str().expect("utf-8 path").to_string()
}

fn plan_input(path: &str, resource: Option<&str>, state_dir: &std::path::Path) -> PlanInput {
    PlanInput {
        path: path.to_string(),
        state_dir: Some(state_dir.display().to_string()),
        resource: resource.map(str::to_string),
        tag: None,
    }
}

// ── GH-214: a selector must reach the counters too ───────────────────

#[tokio::test]
async fn plan_resource_selector_narrows_the_counts_not_just_the_body() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sandbox(dir.path());
    let state = dir.path().join("state");

    let all = PlanHandler
        .handle(plan_input(&path, None, &state))
        .await
        .expect("unfiltered plan");
    // Non-regression: without a selector the whole config is still planned.
    assert_eq!(all.changes.len(), 2, "{all:?}");
    assert_eq!(all.to_create, 2, "{all:?}");

    let one = PlanHandler
        .handle(plan_input(&path, Some("hello-file"), &state))
        .await
        .expect("filtered plan");
    assert_eq!(one.changes.len(), 1, "{one:?}");
    assert_eq!(one.changes[0].resource_id, "hello-file");
    // RED before the fix: to_create stayed 2 — the counters were computed from
    // the UNFILTERED plan and the selector was a post-hoc changes.retain().
    assert_eq!(
        one.to_create, 1,
        "counts must describe the array they ship with: {one:?}"
    );
    assert_eq!(one.to_update + one.to_destroy + one.unchanged, 0, "{one:?}");
}

#[tokio::test]
async fn plan_rejects_a_resource_that_is_not_in_the_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sandbox(dir.path());
    let state = dir.path().join("state");

    // RED before the fix: Ok(changes: [], to_create: 2) with isError: false,
    // while the sibling forjar_show errored on exactly this input.
    let err = PlanHandler
        .handle(plan_input(&path, Some("totally-bogus"), &state))
        .await
        .expect_err("an unknown resource id must be an error");
    let msg = err.to_string();
    assert!(
        msg.contains("totally-bogus") && msg.contains("not found"),
        "error must name the missing resource: {msg}"
    );
}

#[tokio::test]
async fn plan_tag_selector_still_agrees_with_its_own_counts() {
    // Non-regression: the tag selector was already correct; keep it that way.
    let dir = tempfile::tempdir().unwrap();
    let path = write_sandbox(dir.path());
    let state = dir.path().join("state");
    let out = PlanHandler
        .handle(PlanInput {
            path,
            state_dir: Some(state.display().to_string()),
            resource: None,
            tag: Some("nosuchtag".to_string()),
        })
        .await
        .expect("tag plan");
    assert!(out.changes.is_empty(), "{out:?}");
    assert_eq!(out.to_create, 0, "{out:?}");
}

// ── GH-212: show must expand templates in both branches ──────────────

#[tokio::test]
async fn show_expands_templates_for_the_whole_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sandbox(dir.path());

    let out = ShowHandler
        .handle(ShowInput {
            path: path.clone(),
            resource: None,
        })
        .await
        .expect("show");
    let rendered = out.config["resources"]["hello-file"]["path"]
        .as_str()
        .expect("path field")
        .to_string();

    // RED before the fix: "{{params.sandbox}}/hello.txt" — a literal path that
    // exists nowhere on disk, from a tool that advertises "templates expanded".
    assert!(
        !rendered.contains("{{"),
        "whole-config show still returns an unexpanded template: {rendered}"
    );
    assert!(rendered.ends_with("/hello.txt"), "{rendered}");

    // And the resource branch — which was already correct — must agree.
    let one = ShowHandler
        .handle(ShowInput {
            path,
            resource: Some("hello-file".to_string()),
        })
        .await
        .expect("show resource");
    assert_eq!(one.config["path"].as_str().unwrap_or_default(), rendered);
}

// ── GH-212: graph must not label Mermaid as something else ───────────

#[tokio::test]
async fn graph_rejects_an_unknown_format_instead_of_mislabelling_mermaid() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sandbox(dir.path());

    for bad in ["BOGUS", "svg", "ascii"] {
        // RED before the fix: Ok(graph: "graph LR …", format: "BOGUS"/"svg"),
        // isError: false — Mermaid source under a label the caller would trust.
        let result = GraphHandler
            .handle(GraphInput {
                path: path.clone(),
                format: Some(bad.to_string()),
            })
            .await;
        match result {
            Ok(out) => panic!(
                "format '{bad}' was accepted and answered format={} over a {} payload",
                out.format,
                if out.graph.starts_with("graph") {
                    "mermaid"
                } else {
                    "dot"
                }
            ),
            Err(e) => {
                let msg = e.to_string();
                assert!(msg.contains(bad), "error must name the format: {msg}");
            }
        }
    }
}

#[tokio::test]
async fn graph_echoes_the_format_it_actually_rendered() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sandbox(dir.path());

    // Non-regression: the two supported formats still render, and the label
    // matches the payload.
    let mermaid = GraphHandler
        .handle(GraphInput {
            path: path.clone(),
            format: None,
        })
        .await
        .expect("default format");
    assert_eq!(mermaid.format, "mermaid");
    assert!(mermaid.graph.starts_with("graph LR"), "{mermaid:?}");

    let dot = GraphHandler
        .handle(GraphInput {
            path,
            format: Some("dot".to_string()),
        })
        .await
        .expect("dot");
    assert_eq!(dot.format, "dot");
    assert!(dot.graph.starts_with("digraph forjar"), "{dot:?}");
}
