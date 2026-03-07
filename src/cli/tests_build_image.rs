//! Tests for build_image.rs — FJ-2104 CLI wiring.

use super::build_image::*;
use crate::core::types::{ForjarConfig, ImageBuildPlan, LayerStrategy};
use std::io::Write;

fn make_config_with_image() -> ForjarConfig {
    serde_yaml_ng::from_str(r#"
version: "1.0"
name: test-stack
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  my-image:
    type: image
    machine: m
    name: myapp
    version: "1.0.0"
    image: "ubuntu:22.04"
    command: "/usr/local/bin/myapp"
    path: /etc/app/config.yaml
"#).unwrap()
}

fn minimal_config() -> ForjarConfig {
    serde_yaml_ng::from_str(
        "version: '1.0'\nname: test\nmachines: {}\nresources: {}\n"
    ).unwrap()
}

#[test]
fn build_plan_sets_tag_and_base() {
    let config = make_config_with_image();
    let res = config.resources.get("my-image").unwrap();
    let plan = test_build_plan_from_resource("my-image", res, &config).unwrap();
    assert_eq!(plan.tag, "myapp:1.0.0");
    assert_eq!(plan.base_image.as_deref(), Some("ubuntu:22.04"));
}

#[test]
fn build_plan_default_tag() {
    let config: ForjarConfig = serde_yaml_ng::from_str(r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  img:
    type: image
    machine: m
"#).unwrap();
    let res = config.resources.get("img").unwrap();
    let plan = test_build_plan_from_resource("img", res, &config).unwrap();
    assert_eq!(plan.tag, "img:latest");
}

#[test]
fn build_plan_entrypoint() {
    let config = make_config_with_image();
    let res = config.resources.get("my-image").unwrap();
    let plan = test_build_plan_from_resource("my-image", res, &config).unwrap();
    assert_eq!(plan.entrypoint, Some(vec!["/usr/local/bin/myapp".into()]));
}

#[test]
fn build_plan_no_entrypoint() {
    let config: ForjarConfig = serde_yaml_ng::from_str(r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  img:
    type: image
    machine: m
    name: myapp
"#).unwrap();
    let res = config.resources.get("img").unwrap();
    let plan = test_build_plan_from_resource("img", res, &config).unwrap();
    assert!(plan.entrypoint.is_none());
}

#[test]
fn collect_entries_for_files_strategy() {
    let config = make_config_with_image();
    let res = config.resources.get("my-image").unwrap();
    let plan = test_build_plan_from_resource("my-image", res, &config).unwrap();
    let entries = test_collect_layer_entries(&plan, &config).unwrap();
    assert_eq!(entries.len(), 1);
    // The file layer should have one entry for the path
    assert!(!entries[0].is_empty());
}

#[test]
fn collect_entries_packages_strategy() {
    let plan = ImageBuildPlan {
        tag: "test:latest".into(),
        base_image: None,
        layers: vec![LayerStrategy::Packages { names: vec!["curl".into(), "jq".into()] }],
        labels: vec![],
        entrypoint: None,
    };
    let config = minimal_config();
    let entries = test_collect_layer_entries(&plan, &config).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].len(), 1); // one marker file
}

#[test]
fn collect_entries_empty_plan() {
    let plan = ImageBuildPlan {
        tag: "test:latest".into(),
        base_image: None,
        layers: vec![],
        labels: vec![],
        entrypoint: None,
    };
    let config = minimal_config();
    let entries = test_collect_layer_entries(&plan, &config).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn cmd_build_resource_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.yaml");
    std::fs::write(&path, "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m\n    provider: apt\n    packages: [curl]\n").unwrap();
    let r = cmd_build(&path, "nonexistent", false, false, false, false, false);
    assert!(r.is_err(), "expected error, got: {:?}", r);
    assert!(r.as_ref().unwrap_err().contains("not found"), "got: {:?}", r);
}

#[test]
fn cmd_build_not_image_type() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.yaml");
    std::fs::write(&path, "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m\n    provider: apt\n    packages: [curl]\n").unwrap();
    let r = cmd_build(&path, "pkg", false, false, false, false, false);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("not type: image"));
}

#[test]
fn cmd_build_image_resource() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.yaml");
    std::fs::write(&path, "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  my-img:\n    type: image\n    machine: m\n    name: myapp\n    version: \"1.0\"\n    path: /usr/local/bin/app\n").unwrap();
    let r = cmd_build(&path, "my-img", false, false, false, false, false);
    assert!(r.is_ok(), "got error: {:?}", r);
}

#[test]
fn cmd_build_with_far_flag() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.yaml");
    std::fs::write(&path, "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  img:\n    type: image\n    machine: m\n    name: myapp\n    version: \"2.0\"\n    path: /app/bin\n").unwrap();
    let r = cmd_build(&path, "img", false, false, true, false, false);
    assert!(r.is_ok(), "far flag should succeed: {:?}", r);
}

#[test]
fn cmd_build_with_push_flag() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.yaml");
    std::fs::write(&path, "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  img:\n    type: image\n    machine: m\n    name: registry.io/myapp\n    version: \"1.0\"\n    path: /app/bin\n").unwrap();
    let r = cmd_build(&path, "img", false, true, false, false, false);
    assert!(r.is_ok(), "push flag should succeed: {:?}", r);
}

#[test]
fn cmd_build_push_with_local_name() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.yaml");
    std::fs::write(&path, "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  img:\n    type: image\n    machine: m\n    name: myapp\n    path: /app/bin\n").unwrap();
    // No version → defaults to "latest", no slash in name → docker.io default
    let r = cmd_build(&path, "img", false, true, false, false, false);
    assert!(r.is_ok(), "push with local name: {:?}", r);
}

#[test]
fn collect_entries_build_strategy_with_overlay() {
    let dir = tempfile::tempdir().unwrap();
    // Create overlay-like directory structure
    std::fs::create_dir_all(dir.path().join("etc")).unwrap();
    std::fs::write(dir.path().join("etc/app.conf"), "key=value\n").unwrap();

    let plan = ImageBuildPlan {
        tag: "test:latest".into(),
        base_image: None,
        layers: vec![LayerStrategy::Build {
            command: "make install".into(),
            workdir: Some(dir.path().to_string_lossy().to_string()),
        }],
        labels: vec![],
        entrypoint: None,
    };
    let config = minimal_config();
    let entries = test_collect_layer_entries(&plan, &config).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(!entries[0].is_empty(), "should find files in overlay dir");
}

#[test]
fn collect_entries_build_strategy_missing_dir() {
    let plan = ImageBuildPlan {
        tag: "test:latest".into(),
        base_image: None,
        layers: vec![LayerStrategy::Build {
            command: "make".into(),
            workdir: Some("/nonexistent/overlay/path/xyz".into()),
        }],
        labels: vec![],
        entrypoint: None,
    };
    let config = minimal_config();
    let entries = test_collect_layer_entries(&plan, &config).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].is_empty(), "missing dir should produce empty entries");
}

#[test]
fn collect_entries_derivation_strategy_with_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("bin")).unwrap();
    std::fs::write(dir.path().join("bin/app"), "#!/bin/sh\nexec main").unwrap();

    let plan = ImageBuildPlan {
        tag: "test:latest".into(),
        base_image: None,
        layers: vec![LayerStrategy::Derivation {
            store_path: dir.path().to_string_lossy().to_string(),
        }],
        labels: vec![],
        entrypoint: None,
    };
    let config = minimal_config();
    let entries = test_collect_layer_entries(&plan, &config).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(!entries[0].is_empty(), "should find files in store path");
}

#[test]
fn collect_entries_derivation_strategy_missing() {
    let plan = ImageBuildPlan {
        tag: "test:latest".into(),
        base_image: None,
        layers: vec![LayerStrategy::Derivation {
            store_path: "/nonexistent/store/path/abc".into(),
        }],
        labels: vec![],
        entrypoint: None,
    };
    let config = minimal_config();
    let entries = test_collect_layer_entries(&plan, &config).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].is_empty());
}

#[test]
fn cmd_build_far_produces_valid_archive() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.yaml");
    std::fs::write(&path, "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  img:\n    type: image\n    machine: m\n    name: myapp\n    version: \"3.0\"\n    path: /app/bin\n    content: \"#!/bin/sh\\nexec app\"\n").unwrap();
    let r = cmd_build(&path, "img", false, false, true, false, false);
    assert!(r.is_ok(), "far build should succeed: {:?}", r);
    // Verify the FAR file exists and can be decoded
    let far_path = std::path::Path::new("state/images/img.far");
    assert!(far_path.exists(), "FAR archive should be created");
    let file = std::fs::File::open(far_path).unwrap();
    let reader = std::io::BufReader::new(file);
    let (manifest, chunks) = crate::core::store::far::decode_far_manifest(reader).unwrap();
    assert_eq!(manifest.name, "img");
    assert!(manifest.file_count > 0);
    assert!(!chunks.is_empty());
    // Clean up
    let _ = std::fs::remove_file(far_path);
}

#[test]
fn cmd_build_with_load_flag_no_runtime() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.yaml");
    std::fs::write(&path, "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  img:\n    type: image\n    machine: m\n    name: myapp\n    path: /app/bin\n").unwrap();
    // --load requires docker or podman; may or may not be available in test env
    let r = cmd_build(&path, "img", true, false, false, false, false);
    // Either succeeds (docker/podman found) or errors with known message
    if r.is_err() {
        assert!(r.as_ref().unwrap_err().contains("docker or podman"), "got: {:?}", r);
    }
}
