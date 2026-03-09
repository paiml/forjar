//! FJ-MCP/2104: MCP registry schema, image assembler.
//! Usage: cargo test --test falsification_mcp_registry_image

use forjar::core::store::image_assembler::assemble_image;
use forjar::core::store::layer_builder::LayerEntry;
use forjar::core::types::{ImageBuildPlan, LayerStrategy, OciLayerConfig};
use forjar::mcp::registry::{build_registry, export_schema};

// ── FJ-MCP: export_schema ──

#[test]
fn schema_has_expected_tools() {
    let schema = export_schema();
    assert_eq!(schema["schema_version"], "1.0");
    assert_eq!(schema["server"], "forjar-mcp");
    assert_eq!(schema["tool_count"], 9);
    let tools = schema["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 9);
}

#[test]
fn schema_tool_names() {
    let schema = export_schema();
    let tools = schema["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    for expected in [
        "forjar_validate",
        "forjar_plan",
        "forjar_drift",
        "forjar_lint",
        "forjar_graph",
        "forjar_show",
        "forjar_status",
        "forjar_trace",
        "forjar_anomaly",
    ] {
        assert!(names.contains(&expected), "missing tool: {expected}");
    }
}

#[test]
fn schema_tools_have_schemas() {
    let schema = export_schema();
    let tools = schema["tools"].as_array().unwrap();
    for tool in tools {
        assert!(
            tool["input_schema"].is_object(),
            "tool {} missing input_schema",
            tool["name"]
        );
        assert!(
            tool["output_schema"].is_object(),
            "tool {} missing output_schema",
            tool["name"]
        );
        assert!(
            tool["description"].is_string(),
            "tool {} missing description",
            tool["name"]
        );
    }
}

#[test]
fn schema_version_matches_cargo() {
    let schema = export_schema();
    let version = schema["version"].as_str().unwrap();
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
}

// ── FJ-MCP: build_registry ──

#[test]
fn registry_has_9_handlers() {
    let registry = build_registry();
    assert_eq!(registry.len(), 9);
}

#[test]
fn registry_handler_names() {
    let registry = build_registry();
    for name in [
        "forjar_validate",
        "forjar_plan",
        "forjar_drift",
        "forjar_lint",
        "forjar_graph",
        "forjar_show",
        "forjar_status",
        "forjar_trace",
        "forjar_anomaly",
    ] {
        assert!(registry.has_handler(name), "missing handler: {name}");
    }
}

// ── FJ-2104: assemble_image ──

fn default_layer_config() -> OciLayerConfig {
    OciLayerConfig::default()
}

fn plan(layers: Vec<LayerStrategy>) -> ImageBuildPlan {
    ImageBuildPlan {
        tag: "test:latest".into(),
        base_image: None,
        layers,
        labels: vec![("org.forjar".into(), "test".into())],
        entrypoint: Some(vec!["/bin/sh".into()]),
    }
}

fn entries(files: &[(&str, &[u8])]) -> Vec<LayerEntry> {
    files
        .iter()
        .map(|(path, content)| LayerEntry {
            path: path.to_string(),
            content: content.to_vec(),
            mode: 0o644,
            is_dir: false,
        })
        .collect()
}

#[test]
fn assemble_single_layer() {
    let tmp = tempfile::tempdir().unwrap();
    let p = plan(vec![LayerStrategy::Files {
        paths: vec!["app.txt".into()],
    }]);
    let layer_data = vec![entries(&[("app.txt", b"hello forjar")])];
    let result =
        assemble_image(&p, &layer_data, tmp.path(), &default_layer_config(), None).unwrap();
    assert_eq!(result.layers.len(), 1);
    assert!(result.total_size > 0);
    assert!(result.layout_dir.exists());
    assert!(tmp.path().join("oci-layout").exists());
    assert!(tmp.path().join("index.json").exists());
    assert!(tmp.path().join("manifest.json").exists());
    assert!(tmp.path().join("blobs/sha256").exists());
}

#[test]
fn assemble_multi_layer() {
    let tmp = tempfile::tempdir().unwrap();
    let p = plan(vec![
        LayerStrategy::Packages {
            names: vec!["curl".into()],
        },
        LayerStrategy::Files {
            paths: vec!["config.yaml".into()],
        },
    ]);
    let layer_data = vec![
        entries(&[("usr/bin/curl", b"fake-binary")]),
        entries(&[("etc/config.yaml", b"key: value")]),
    ];
    let result =
        assemble_image(&p, &layer_data, tmp.path(), &default_layer_config(), None).unwrap();
    assert_eq!(result.layers.len(), 2);
    assert_eq!(result.manifest.layers.len(), 2);
}

#[test]
fn assemble_with_arch() {
    let tmp = tempfile::tempdir().unwrap();
    let p = plan(vec![LayerStrategy::Files {
        paths: vec!["app".into()],
    }]);
    let layer_data = vec![entries(&[("app", b"binary")])];
    let result = assemble_image(
        &p,
        &layer_data,
        tmp.path(),
        &default_layer_config(),
        Some("arm64"),
    )
    .unwrap();
    assert_eq!(result.config.architecture, "arm64");
}

#[test]
fn assemble_layer_count_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let p = plan(vec![
        LayerStrategy::Files { paths: vec![] },
        LayerStrategy::Files { paths: vec![] },
    ]);
    let layer_data = vec![entries(&[("a", b"x")])]; // 1 entry set but plan has 2 layers
    let err =
        assemble_image(&p, &layer_data, tmp.path(), &default_layer_config(), None).unwrap_err();
    assert!(err.contains("layer count mismatch"));
}

#[test]
fn assemble_entrypoint_in_config() {
    let tmp = tempfile::tempdir().unwrap();
    let p = plan(vec![LayerStrategy::Files {
        paths: vec!["app".into()],
    }]);
    let layer_data = vec![entries(&[("app", b"binary")])];
    let result =
        assemble_image(&p, &layer_data, tmp.path(), &default_layer_config(), None).unwrap();
    assert_eq!(result.config.config.entrypoint, vec!["/bin/sh"]);
}

#[test]
fn assemble_labels_in_config() {
    let tmp = tempfile::tempdir().unwrap();
    let p = plan(vec![LayerStrategy::Files {
        paths: vec!["a".into()],
    }]);
    let layer_data = vec![entries(&[("a", b"x")])];
    let result =
        assemble_image(&p, &layer_data, tmp.path(), &default_layer_config(), None).unwrap();
    assert_eq!(result.config.config.labels["org.forjar"], "test");
}

#[test]
fn assemble_history_matches_layers() {
    let tmp = tempfile::tempdir().unwrap();
    let p = plan(vec![
        LayerStrategy::Packages {
            names: vec!["vim".into()],
        },
        LayerStrategy::Build {
            command: "make install".into(),
            workdir: None,
        },
    ]);
    let layer_data = vec![
        entries(&[("usr/bin/vim", b"vim")]),
        entries(&[("usr/local/bin/app", b"app")]),
    ];
    let result =
        assemble_image(&p, &layer_data, tmp.path(), &default_layer_config(), None).unwrap();
    assert_eq!(result.config.history.len(), 2);
    assert!(result.config.history[0]
        .created_by
        .as_ref()
        .unwrap()
        .contains("packages"));
    assert!(result.config.history[1]
        .created_by
        .as_ref()
        .unwrap()
        .contains("build"));
}

#[test]
fn assemble_diff_ids_match_layers() {
    let tmp = tempfile::tempdir().unwrap();
    let p = plan(vec![LayerStrategy::Files {
        paths: vec!["a".into()],
    }]);
    let layer_data = vec![entries(&[("a", b"content")])];
    let result =
        assemble_image(&p, &layer_data, tmp.path(), &default_layer_config(), None).unwrap();
    assert_eq!(result.config.rootfs.diff_ids.len(), 1);
    assert!(result.config.rootfs.diff_ids[0].starts_with("sha256:"));
}
