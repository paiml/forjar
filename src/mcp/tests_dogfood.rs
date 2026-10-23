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
use super::handlers_drift::DriftHandler;
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

// ── forjar#660: the MCP readers resolve `data:` sources too ──────────

fn write_data_manifest(dir: &std::path::Path, command: &str) -> String {
    let path = dir.join("forjar.yaml");
    std::fs::write(
        &path,
        format!(
            "version: \"1.0\"\nname: data-660\n\
             machines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\n\
             resources:\n  cfg:\n    type: file\n    machine: m1\n    path: {}/out.txt\n    content: \"greeting={{{{data.greeting}}}}\"\n\
             outputs:\n  g:\n    value: \"g: {{{{data.greeting}}}}\"\n\
             data:\n  greeting:\n    type: command\n    value: \"{command}\"\n",
            dir.display()
        ),
    )
    .expect("write manifest");
    path.to_str().expect("utf-8 path").to_string()
}

#[tokio::test]
async fn mcp_readers_fail_on_a_failing_data_source_and_resolve_a_good_one() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state").display().to_string();

    let bad = write_data_manifest(dir.path(), "exit 3");
    let e = PlanHandler
        .handle(PlanInput {
            path: bad.clone(),
            state_dir: Some(state.clone()),
            resource: None,
            tag: None,
        })
        .await
        .expect_err("forjar_plan must fail");
    assert!(format!("{e:?}").contains("data source 'greeting'"), "{e:?}");
    let e = ShowHandler
        .handle(ShowInput {
            path: bad.clone(),
            resource: None,
        })
        .await
        .expect_err("forjar_show must fail");
    assert!(format!("{e:?}").contains("data source 'greeting'"), "{e:?}");
    let e = DriftHandler
        .handle(DriftInput {
            path: bad,
            state_dir: Some(state),
            machine: None,
        })
        .await
        .expect_err("forjar_drift must fail");
    assert!(format!("{e:?}").contains("data source 'greeting'"), "{e:?}");

    let good = write_data_manifest(dir.path(), "echo hello-from-data");
    let out = ShowHandler
        .handle(ShowInput {
            path: good,
            resource: None,
        })
        .await
        .expect("show resolves a good data source");
    assert!(
        out.config.to_string().contains("hello-from-data"),
        "{:?}",
        out.config
    );
}

/// `forjar_show` without `resource` prints the RESOLVED outputs, and no `__data__`.
#[tokio::test]
async fn show_prints_resolved_outputs_and_no_data_key() {
    let dir = tempfile::tempdir().unwrap();
    let good = write_data_manifest(dir.path(), "echo hello-from-data");
    let out = ShowHandler
        .handle(ShowInput {
            path: good,
            resource: None,
        })
        .await
        .expect("show");
    let body = out.config.to_string();
    assert!(body.contains("g: hello-from-data"), "{body}");
    assert!(!body.contains("__data__"), "{body}");
    assert!(!body.contains("{{"), "{body}");
}

/// EVERY handler that reads the manifest, on a failing source and a good one.
/// The roster is checked against the handler impls in the source so a new
/// config-reading handler cannot be added without a row here.
#[tokio::test]
async fn every_config_reading_handler_resolves_or_fails_on_data() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state").display().to_string();
    let bad = write_data_manifest(dir.path(), "exit 3");
    let want = "data source 'greeting'";

    let mut failures: Vec<(&str, String)> = Vec::new();
    macro_rules! expect_err {
        ($name:expr, $call:expr) => {
            match $call.await {
                Err(e) => {
                    let t = format!("{e:?}");
                    if !t.contains(want) {
                        failures.push(($name, format!("error did not name the source: {t}")));
                    }
                }
                Ok(_) => failures.push(($name, "returned Ok on a failing data: source".into())),
            }
        };
    }
    expect_err!(
        "ValidateHandler",
        ValidateHandler.handle(ValidateInput { path: bad.clone() })
    );
    expect_err!(
        "PlanHandler",
        PlanHandler.handle(PlanInput {
            path: bad.clone(),
            state_dir: Some(state.clone()),
            resource: None,
            tag: None,
        })
    );
    expect_err!(
        "LintHandler",
        LintHandler.handle(LintInput {
            path: bad.clone(),
            max_cyclomatic: None,
        })
    );
    expect_err!(
        "GraphHandler",
        GraphHandler.handle(GraphInput {
            path: bad.clone(),
            format: None,
        })
    );
    expect_err!(
        "ShowHandler",
        ShowHandler.handle(ShowInput {
            path: bad.clone(),
            resource: None,
        })
    );
    expect_err!(
        "DriftHandler",
        DriftHandler.handle(DriftInput {
            path: bad.clone(),
            state_dir: Some(state.clone()),
            machine: None,
        })
    );
    expect_err!(
        "RemediateHandler",
        super::handlers::RemediateHandler.handle(RemediateInput {
            path: bad,
            policy_ids: None,
        })
    );
    assert!(failures.is_empty(), "{failures:#?}");

    // A manifest that is merely INVALID is still a result, not an error.
    let invalid = dir.path().join("invalid.yaml");
    std::fs::write(&invalid, "version: \"1.0\"\nname: x\nmachines: [\n").unwrap();
    let out = ValidateHandler
        .handle(ValidateInput {
            path: invalid.display().to_string(),
        })
        .await
        .expect("an invalid manifest is a valid:false result");
    assert!(!out.valid && !out.errors.is_empty(), "{out:?}");

    // Good source: each succeeds.
    let good = write_data_manifest(dir.path(), "echo hello-from-data");
    let v = ValidateHandler
        .handle(ValidateInput { path: good.clone() })
        .await
        .expect("validate");
    assert!(v.valid, "{v:?}");
    PlanHandler
        .handle(PlanInput {
            path: good.clone(),
            state_dir: Some(state.clone()),
            resource: None,
            tag: None,
        })
        .await
        .expect("plan");
    GraphHandler
        .handle(GraphInput {
            path: good.clone(),
            format: None,
        })
        .await
        .expect("graph");
    super::handlers::RemediateHandler
        .handle(RemediateInput {
            path: good,
            policy_ids: None,
        })
        .await
        .expect("remediate");

    // The roster: every `impl Handler for X` is either tested above or is
    // state-only (never opens the manifest).
    const STATE_ONLY: &[&str] = &[
        "AuditHandler",
        "WorkspaceHandler",
        "StatusHandler",
        "TraceHandler",
        "AnomalyHandler",
    ];
    const TESTED: &[&str] = &[
        "ValidateHandler",
        "PlanHandler",
        "LintHandler",
        "GraphHandler",
        "ShowHandler",
        "DriftHandler",
        "RemediateHandler",
    ];
    let mut impls = Vec::new();
    for e in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src/mcp")).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if !name.starts_with("handlers") {
            continue;
        }
        let src = std::fs::read_to_string(&p).unwrap();
        let prod = src.split("#[cfg(test)]").next().unwrap();
        for l in prod.lines() {
            if let Some(rest) = l.strip_prefix("impl Handler for ") {
                impls.push((rest.trim_end_matches(" {").to_string(), prod.to_string()));
            }
        }
    }
    assert!(impls.len() >= 12, "only {} handlers found", impls.len());
    for (h, _) in &impls {
        assert!(
            TESTED.contains(&h.as_str()) || STATE_ONLY.contains(&h.as_str()),
            "{h} is neither tested for data: resolution nor listed as state-only"
        );
    }
    for h in STATE_ONLY {
        assert!(
            impls.iter().any(|(n, _)| n == h),
            "stale state-only row {h}"
        );
    }
}
