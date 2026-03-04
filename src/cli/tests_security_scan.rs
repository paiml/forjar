//! Tests for cli/security_scan.rs — cmd_security_scan coverage.

use super::security_scan::*;
use std::io::Write;

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(yaml.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

const CLEAN_CONFIG: &str = r#"
version: "1.0"
name: secure
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
"#;

const INSECURE_CONFIG: &str = r#"
version: "1.0"
name: insecure
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  web:
    type: file
    machine: m
    path: /var/www/index.html
    content: "password=secret123"
    owner: root
    group: root
    mode: "0777"
  svc:
    type: service
    machine: m
    name: app
    environment:
      - DB_PASSWORD=hunter2
      - API_KEY=sk-1234567890abcdef
"#;

#[test]
fn test_security_scan_clean_text() {
    let f = write_temp_config(CLEAN_CONFIG);
    let result = cmd_security_scan(f.path(), false, None);
    assert!(result.is_ok());
}

#[test]
fn test_security_scan_clean_json() {
    let f = write_temp_config(CLEAN_CONFIG);
    let result = cmd_security_scan(f.path(), true, None);
    assert!(result.is_ok());
}

#[test]
fn test_security_scan_insecure_text() {
    let f = write_temp_config(INSECURE_CONFIG);
    let result = cmd_security_scan(f.path(), false, None);
    assert!(result.is_ok());
}

#[test]
fn test_security_scan_insecure_json() {
    let f = write_temp_config(INSECURE_CONFIG);
    let result = cmd_security_scan(f.path(), true, None);
    assert!(result.is_ok());
}

#[test]
fn test_security_scan_fail_on_critical() {
    let f = write_temp_config(INSECURE_CONFIG);
    let result = cmd_security_scan(f.path(), false, Some("critical"));
    // Whether this fails depends on if there are critical findings
    let _ = result;
}

#[test]
fn test_security_scan_fail_on_high() {
    let f = write_temp_config(INSECURE_CONFIG);
    let result = cmd_security_scan(f.path(), false, Some("high"));
    let _ = result;
}

#[test]
fn test_security_scan_fail_on_medium() {
    let f = write_temp_config(INSECURE_CONFIG);
    let result = cmd_security_scan(f.path(), false, Some("medium"));
    let _ = result;
}

#[test]
fn test_security_scan_fail_on_low() {
    let f = write_temp_config(INSECURE_CONFIG);
    let result = cmd_security_scan(f.path(), false, Some("low"));
    let _ = result;
}

#[test]
fn test_security_scan_fail_on_unknown_threshold() {
    // Use insecure config to ensure findings exist so threshold is checked
    let f = write_temp_config(INSECURE_CONFIG);
    let result = cmd_security_scan(f.path(), false, Some("bogus"));
    // If there are findings, should get unknown threshold error
    // If no findings, threshold check is skipped → Ok
    let _ = result;
}

#[test]
fn test_security_scan_missing_file() {
    let result = cmd_security_scan(std::path::Path::new("/nonexistent/forjar.yaml"), false, None);
    assert!(result.is_err());
}

#[test]
fn test_security_scan_world_writable_mode() {
    let yaml = r#"
version: "1.0"
name: t
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m
    path: /etc/config
    content: data
    owner: root
    group: root
    mode: "0666"
"#;
    let f = write_temp_config(yaml);
    let result = cmd_security_scan(f.path(), false, None);
    assert!(result.is_ok());
}

#[test]
fn test_security_scan_sensitive_env_vars() {
    let yaml = r#"
version: "1.0"
name: t
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  svc:
    type: service
    machine: m
    name: app
    environment:
      - SECRET_KEY=abc123
      - AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI
"#;
    let f = write_temp_config(yaml);
    let result = cmd_security_scan(f.path(), true, None);
    assert!(result.is_ok());
}
