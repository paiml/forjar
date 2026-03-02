//! Coverage tests: overflow from tests_cov_validate_ext (FJ-1372).

#![allow(unused_imports)]
use super::validate_governance::*;
use super::validate_ordering_ext::*;
use std::path::{Path, PathBuf};

fn write_cfg(dir: &Path, yaml: &str) -> PathBuf {
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

fn pkg_cfg() -> &'static str {
    r#"version: "1.0"
name: validate-pkg
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  web:
    type: package
    machine: m1
    packages: [nginx]
    tags: [web]
  db:
    type: package
    machine: m1
    packages: [postgresql]
    version: "15"
    tags: [db]
  cfg:
    type: file
    machine: m1
    path: /tmp/app.conf
    content: "key=value"
    tags: [app]
"#
}

#[test]
fn governance_provider_support_pkg() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), pkg_cfg());
    assert!(cmd_validate_check_resource_provider_support(&f, false).is_ok());
}
#[test]
fn governance_provider_version_pkg() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), pkg_cfg());
    assert!(cmd_validate_check_resource_provider_version(&f, false).is_ok());
}
#[test]
fn governance_drift_risk_pkg() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), pkg_cfg());
    assert!(cmd_validate_check_resource_drift_risk(&f, false).is_ok());
}
#[test]
fn ordering_ext_content_size_pkg() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), pkg_cfg());
    assert!(cmd_validate_check_resource_content_size_limit(&f, false).is_ok());
}
#[test]
fn ordering_ext_gpu_backend_pkg() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), pkg_cfg());
    assert!(cmd_validate_check_resource_gpu_backend_consistency(&f, false).is_ok());
}
