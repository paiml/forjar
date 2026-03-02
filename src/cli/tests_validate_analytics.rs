//! Coverage tests for validate_analytics.rs (FJ-1038, FJ-1041, FJ-1044).

use super::validate_analytics::{
    cmd_validate_check_dependency_optimization,
    cmd_validate_check_resource_consolidation_opportunities,
    cmd_validate_check_resource_health_correlation, levenshtein,
};
use std::io::Write;

fn write_config(dir: &tempfile::TempDir, content: &str) -> std::path::PathBuf {
    let path = dir.path().join("forjar.yaml");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

fn config_with_hub() -> &'static str {
    r#"version: "1.0"
name: hub-test
machines:
  web1:
    hostname: web1
    addr: 127.0.0.1
    user: root
    arch: x86_64
resources:
  base-pkg:
    type: package
    machine: web1
    packages: [curl]
    depends_on: []
  app-a:
    type: file
    machine: web1
    path: /etc/a
    depends_on: [base-pkg]
  app-b:
    type: file
    machine: web1
    path: /etc/b
    depends_on: [base-pkg]
  app-c:
    type: file
    machine: web1
    path: /etc/c
    depends_on: [base-pkg]
  app-d:
    type: file
    machine: web1
    path: /etc/d
    depends_on: [base-pkg]
"#
}

fn config_no_hub() -> &'static str {
    r#"version: "1.0"
name: no-hub-test
machines:
  web1:
    hostname: web1
    addr: 127.0.0.1
    user: root
    arch: x86_64
resources:
  base:
    type: package
    machine: web1
    packages: [curl]
    depends_on: []
  app:
    type: file
    machine: web1
    path: /etc/app
    depends_on: [base]
"#
}

fn config_with_chain() -> &'static str {
    r#"version: "1.0"
name: chain-test
machines:
  web1:
    hostname: web1
    addr: 127.0.0.1
    user: root
    arch: x86_64
resources:
  a:
    type: package
    machine: web1
    packages: [a]
    depends_on: []
  b:
    type: file
    machine: web1
    path: /b
    depends_on: [a]
  c:
    type: file
    machine: web1
    path: /c
    depends_on: [a, b]
"#
}

fn config_consolidation() -> &'static str {
    r#"version: "1.0"
name: consolidation-test
machines:
  web1:
    hostname: web1
    addr: 127.0.0.1
    user: root
    arch: x86_64
  web2:
    hostname: web2
    addr: 10.0.0.2
    user: root
    arch: x86_64
resources:
  nginx-web1:
    type: package
    machine: web1
    packages: [nginx]
    content: "server { listen 80; }"
    depends_on: []
  nginx-web2:
    type: package
    machine: web2
    packages: [nginx]
    content: "server { listen 80; }"
    depends_on: []
"#
}

#[test]
fn health_correlation_with_hub_text() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, config_with_hub());
    let result = cmd_validate_check_resource_health_correlation(&path, false);
    assert!(result.is_ok(), "error: {:?}", result.err());
}

#[test]
fn health_correlation_with_hub_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, config_with_hub());
    let result = cmd_validate_check_resource_health_correlation(&path, true);
    assert!(result.is_ok(), "error: {:?}", result.err());
}

#[test]
fn health_correlation_no_hub() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, config_no_hub());
    let result = cmd_validate_check_resource_health_correlation(&path, false);
    assert!(result.is_ok(), "error: {:?}", result.err());
}

#[test]
fn health_correlation_missing_file() {
    let result = cmd_validate_check_resource_health_correlation(
        std::path::Path::new("/nonexistent/forjar.yaml"),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dependency_optimization_with_chain() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, config_with_chain());
    let result = cmd_validate_check_dependency_optimization(&path, false);
    assert!(result.is_ok(), "error: {:?}", result.err());
}

#[test]
fn dependency_optimization_with_chain_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, config_with_chain());
    let result = cmd_validate_check_dependency_optimization(&path, true);
    assert!(result.is_ok(), "error: {:?}", result.err());
}

#[test]
fn dependency_optimization_no_redundant() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, config_no_hub());
    let result = cmd_validate_check_dependency_optimization(&path, false);
    assert!(result.is_ok(), "error: {:?}", result.err());
}

#[test]
fn consolidation_opportunities_with_dupes() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, config_consolidation());
    let result = cmd_validate_check_resource_consolidation_opportunities(&path, false);
    assert!(result.is_ok(), "error: {:?}", result.err());
}

#[test]
fn consolidation_opportunities_with_dupes_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, config_consolidation());
    let result = cmd_validate_check_resource_consolidation_opportunities(&path, true);
    assert!(result.is_ok(), "error: {:?}", result.err());
}

#[test]
fn consolidation_no_dupes() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, config_no_hub());
    let result = cmd_validate_check_resource_consolidation_opportunities(&path, false);
    assert!(result.is_ok(), "error: {:?}", result.err());
}

#[test]
fn levenshtein_identical() {
    assert_eq!(levenshtein("hello", "hello"), 0);
}

#[test]
fn levenshtein_one_edit() {
    assert_eq!(levenshtein("cat", "bat"), 1);
}

#[test]
fn levenshtein_two_edits() {
    assert_eq!(levenshtein("kitten", "sitten"), 1);
    assert_eq!(levenshtein("cat", "car"), 1);
}

#[test]
fn levenshtein_empty() {
    assert_eq!(levenshtein("", "abc"), 3);
    assert_eq!(levenshtein("abc", ""), 3);
    assert_eq!(levenshtein("", ""), 0);
}

#[test]
fn levenshtein_different_lengths() {
    assert_eq!(levenshtein("abc", "abcd"), 1);
    assert_eq!(levenshtein("nginx-web1", "nginx-web2"), 1);
}

#[test]
fn levenshtein_swapped_args() {
    assert_eq!(levenshtein("abc", "xyz"), levenshtein("xyz", "abc"));
    assert_eq!(
        levenshtein("longer", "short"),
        levenshtein("short", "longer")
    );
}
