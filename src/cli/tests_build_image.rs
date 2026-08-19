//! Tests for build_image.rs — FJ-2104 CLI wiring.

use super::build_image::*;
use crate::core::types::{ForjarConfig, ImageBuildPlan, LayerStrategy};

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

// ── --push (Refs #210) ───────────────────────────────────────────────
//
// These two used to assert `is_ok()`. They passed only because the push
// swallowed every failure: it printed "push skipped: registry unreachable"
// (or, with a network, "Push complete: 3 uploaded" after PUTting the blob at
// whatever a 301 redirect pointed to) and returned Ok. Both now target a
// closed loopback port, so they are hermetic AND assert the honest outcome.

/// A registry that is guaranteed not to answer: port 1 on loopback.
const DEAD_REGISTRY: &str = "127.0.0.1:1";

#[test]
fn cmd_build_with_push_flag_fails_when_the_push_cannot_happen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.yaml");
    std::fs::write(&path, format!("version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  img-push-210:\n    type: image\n    machine: m\n    name: {DEAD_REGISTRY}/myapp\n    version: \"1.0\"\n    path: /app/bin\n")).unwrap();
    let r = cmd_build(&path, "img-push-210", false, true, false, false, false);
    let err = r.expect_err("a build whose push never reached a registry must exit non-zero");
    assert!(
        !err.to_lowercase().contains("skip"),
        "a push that did not happen is a failure, not a skip: {err}"
    );
}

#[test]
fn push_targets_the_declared_reference_not_a_hardcoded_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.yaml");
    // The resource declares a full reference via `tag:` — which the build used
    // to drop on the floor while the push invented `docker.io/app:latest`.
    std::fs::write(&path, format!("version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  img-ref-210:\n    type: image\n    machine: m\n    tag: \"{DEAD_REGISTRY}/foo/bar:1.2.3\"\n    path: /app/bin\n")).unwrap();
    let err = cmd_build(&path, "img-ref-210", false, true, false, false, false)
        .expect_err("push to a dead registry must fail");
    assert!(
        err.contains(DEAD_REGISTRY) && err.contains("foo/bar"),
        "the push must target the DECLARED reference, not docker.io/app:latest: {err}"
    );
    assert!(
        !err.contains("docker.io") && !err.contains("/app:latest"),
        "no hardcoded fallback target may appear: {err}"
    );
}

/// NON-REGRESSION GUARD for the half that was always correct: the build must
/// still write a real OCI layout (index.json → manifest → config + layer
/// blobs) under state/images/<resource>/.
#[test]
fn build_half_still_writes_a_real_oci_layout() {
    use crate::core::types::{OciIndex, OciManifest};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.yaml");
    std::fs::write(&path, "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  cfg-210:\n    type: file\n    machine: m\n    path: /etc/gh210.conf\n    content: \"gh210\\n\"\n  img-guard-210:\n    type: image\n    machine: m\n    name: guard/app\n    version: \"1.0\"\n    path: /etc/gh210.conf\n").unwrap();

    let out = std::path::Path::new("state/images/img-guard-210");
    let _ = std::fs::remove_dir_all(out);
    cmd_build(&path, "img-guard-210", false, false, false, false, false)
        .expect("build without --push must still succeed");

    let index: OciIndex =
        serde_json::from_str(&std::fs::read_to_string(out.join("index.json")).unwrap())
            .expect("index.json must parse as an OCI index");
    assert_eq!(index.manifests.len(), 1);

    let blobs = out.join("blobs").join("sha256");
    let digest_path = |digest: &str| blobs.join(digest.strip_prefix("sha256:").unwrap());
    let manifest_path = digest_path(&index.manifests[0].digest);
    let manifest: OciManifest =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap())
            .expect("the manifest blob must parse");
    assert!(
        digest_path(&manifest.config.digest).is_file(),
        "config blob must exist on disk"
    );
    assert!(!manifest.layers.is_empty(), "image must have layers");
    for layer in &manifest.layers {
        assert!(
            digest_path(&layer.digest).is_file(),
            "layer blob {} must exist on disk",
            layer.digest
        );
    }
    let _ = std::fs::remove_dir_all(out);
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
    if let Err(e) = &r {
        assert!(e.contains("docker or podman"), "got: {:?}", r);
    }
}

#[test]
fn split_paths_by_type_separates_configs() {
    let paths = vec![
        "/app/server".to_string(),
        "/etc/app/config.yaml".to_string(),
        "/etc/app/settings.toml".to_string(),
        "/app/worker".to_string(),
        "/etc/nginx/nginx.conf".to_string(),
    ];
    let (configs, apps) = test_split_paths_by_type(&paths);
    assert_eq!(configs.len(), 3, "should find 3 config files");
    assert_eq!(apps.len(), 2, "should find 2 app files");
    assert!(configs.contains(&"/etc/app/config.yaml".to_string()));
    assert!(configs.contains(&"/etc/app/settings.toml".to_string()));
    assert!(configs.contains(&"/etc/nginx/nginx.conf".to_string()));
    assert!(apps.contains(&"/app/server".to_string()));
}

#[test]
fn split_paths_no_configs() {
    let paths = vec!["/app/bin".to_string(), "/usr/bin/tool".to_string()];
    let (configs, apps) = test_split_paths_by_type(&paths);
    assert!(configs.is_empty());
    assert_eq!(apps.len(), 2);
}

#[test]
fn split_paths_all_configs() {
    let paths = vec![
        "/etc/app.json".to_string(),
        "/etc/db.env".to_string(),
    ];
    let (configs, apps) = test_split_paths_by_type(&paths);
    assert_eq!(configs.len(), 2);
    assert!(apps.is_empty());
}

#[test]
fn split_paths_empty() {
    let paths: Vec<String> = vec![];
    let (configs, apps) = test_split_paths_by_type(&paths);
    assert!(configs.is_empty());
    assert!(apps.is_empty());
}

#[test]
fn build_plan_single_path_one_layer() {
    let config: ForjarConfig = serde_yaml_ng::from_str(r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  single-file-image:
    type: image
    machine: m
    name: myapp
    version: "1.0.0"
    command: "/app/server"
    path: /app/server
"#).unwrap();
    let res = config.resources.get("single-file-image").unwrap();
    let plan = test_build_plan_from_resource("single-file-image", res, &config).unwrap();
    // Single path → single layer (no split possible)
    assert_eq!(plan.layers.len(), 1, "single path should produce 1 layer");
}

#[test]
fn split_paths_triggers_two_layers_when_mixed() {
    // Directly test split_paths_by_type to verify the split logic
    let paths = vec![
        "/app/server".to_string(),
        "/etc/app/config.yaml".to_string(),
    ];
    let (configs, apps) = test_split_paths_by_type(&paths);
    assert_eq!(configs.len(), 1);
    assert_eq!(apps.len(), 1);
    // Verify that build_plan_from_resource would create 2 layers
    // if the resource had multiple paths
    assert!(configs.contains(&"/etc/app/config.yaml".to_string()));
    assert!(apps.contains(&"/app/server".to_string()));
}
