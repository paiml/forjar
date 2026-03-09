//! Additional coverage for sbom.rs — parse_image_tag, truncate_str, model/file components.

use super::sbom::*;
use crate::core::{state, types};
use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

// ── cmd_sbom coverage for text output ───────────────────────────────

#[test]
fn sbom_text_with_packages() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test-app
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg-tools:
    type: package
    machine: m
    provider: apt
    packages: [curl, wget, vim]
    version: "1.0"
"#,
    );
    assert!(cmd_sbom(&p, dir.path(), false).is_ok());
}

#[test]
fn sbom_json_with_model() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: ml-pipeline
machines:
  m:
    hostname: m
    addr: localhost
resources:
  model-llm:
    type: model
    machine: m
    name: llama-3
    source: "huggingface.co/meta/llama-3"
    path: /models/llama-3
    version: "3.1"
    checksum: "blake3:abc123"
"#,
    );
    let r = cmd_sbom(&p, dir.path(), true);
    assert!(r.is_ok(), "sbom_json_with_model failed: {:?}", r.err());
}

#[test]
fn sbom_with_file_source() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  binary:
    type: file
    machine: m
    path: /usr/local/bin/app
    source: "https://example.com/app-v1"
"#,
    );
    assert!(cmd_sbom(&p, &state_dir, false).is_ok());
}

#[test]
fn sbom_docker_with_tag() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: containers
machines:
  m:
    hostname: m
    addr: localhost
resources:
  web:
    type: docker
    machine: m
    name: web-container
    image: "nginx:1.25-alpine"
"#,
    );
    assert!(cmd_sbom(&p, dir.path(), true).is_ok());
}

#[test]
fn sbom_docker_no_tag() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: containers
machines:
  m:
    hostname: m
    addr: localhost
resources:
  web:
    type: docker
    machine: m
    name: web-container
    image: "nginx"
"#,
    );
    assert!(cmd_sbom(&p, dir.path(), false).is_ok());
}

#[test]
fn sbom_docker_registry_port() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: containers
machines:
  m:
    hostname: m
    addr: localhost
resources:
  web:
    type: docker
    machine: m
    name: web-container
    image: "registry:5000/myapp/web"
"#,
    );
    assert!(cmd_sbom(&p, dir.path(), true).is_ok());
}

#[test]
fn sbom_with_state_hash_lookup() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");

    // Create a lock file with a resource hash
    let mut lock = state::new_lock("m", "m.local");
    lock.resources.insert(
        "cfg-app".to_string(),
        types::ResourceLock {
            resource_type: types::ResourceType::File,
            status: types::ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:deadbeef".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    state::save_lock(&state_dir, &lock).unwrap();

    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg-app:
    type: file
    machine: m
    path: /etc/app.conf
    source: "https://example.com/app.conf"
"#,
    );
    assert!(cmd_sbom(&p, &state_dir, true).is_ok());
}

#[test]
fn sbom_mixed_resource_types() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: mixed
machines:
  m:
    hostname: m
    addr: localhost
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [nginx]
  svc:
    type: service
    machine: m
    name: nginx
    service_name: nginx
  cfg:
    type: file
    machine: m
    path: /etc/nginx/nginx.conf
    content: "server {}"
"#,
    );
    // service and file-without-source types should be skipped
    let r = cmd_sbom(&p, dir.path(), false);
    assert!(r.is_ok(), "sbom_mixed failed: {:?}", r.err());
}

#[test]
fn sbom_pkg_no_version() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#,
    );
    assert!(cmd_sbom(&p, dir.path(), false).is_ok());
}

#[test]
fn sbom_model_no_source() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: localhost
resources:
  model:
    type: model
    machine: m
    name: test-model
    path: /models/test
    version: "1.0"
"#,
    );
    // Model without source should produce no component
    assert!(cmd_sbom(&p, dir.path(), true).is_ok());
}

// ── long name truncation in text output ─────────────────────────────

#[test]
fn sbom_text_long_names() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [this-is-a-very-long-package-name-that-exceeds-thirty-characters]
    version: "1.0.0-beta.12345+longbuildmeta"
"#,
    );
    assert!(cmd_sbom(&p, dir.path(), false).is_ok());
}
