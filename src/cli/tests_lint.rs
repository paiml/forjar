//! Tests: Linting.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::lint::*;
use super::commands::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj017_lint_duplicate_content() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: lint-dup
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  file-a:
    type: file
    machine: m1
    path: /etc/a.conf
    content: "same content"
  file-b:
    type: file
    machine: m1
    path: /etc/b.conf
    content: "same content"
  file-c:
    type: file
    machine: m1
    path: /etc/c.conf
    content: "same content"
"#,
        )
        .unwrap();
        // Lint should detect duplicate content
        cmd_lint(&config, false, false, false).unwrap();
    }

    // ── Init edge case ────────────────────────────────────────


    #[test]
    fn test_fj132_cmd_lint_valid() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
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
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_lint(&file, false, false, false).unwrap();
    }


    #[test]
    fn test_fj132_cmd_lint_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
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
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_lint(&file, true, false, false).unwrap();
    }


    #[test]
    fn test_fj036_cmd_lint_bashrs_reports() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        // Config with a package resource — codegen will produce scripts
        // that bashrs can lint for shell safety diagnostics
        let yaml = r#"
version: "1.0"
name: lint-bashrs
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl, wget]
  conf:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "key=value"
"#;
        std::fs::write(&file, yaml).unwrap();
        // cmd_lint should succeed and produce bashrs diagnostics summary
        let result = cmd_lint(&file, true, false, false);
        assert!(
            result.is_ok(),
            "cmd_lint should succeed: {:?}",
            result.err()
        );
    }


    #[test]
    fn test_fj017_cmd_lint_clean_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  my-config:
    type: file
    machine: local
    path: /etc/app.conf
    content: "key=value"
"#,
        )
        .unwrap();
        let result = cmd_lint(&config, false, false, false);
        assert!(
            result.is_ok(),
            "cmd_lint should succeed on a valid config with file resource"
        );
    }


    #[test]
    fn test_fj332_lint_fix_flag() {
        let cmd = Commands::Lint {
            file: PathBuf::from("f.yaml"),
            json: false,
            strict: false,
            fix: true,
            rules: None,
        };
        match cmd {
            Commands::Lint { fix, .. } => assert!(fix),
            _ => panic!("expected Lint"),
        }
    }


    #[test]
    fn test_fj374_lint_rules_flag() {
        let cmd = Commands::Lint {
            file: PathBuf::from("f.yaml"),
            json: false,
            strict: false,
            fix: false,
            rules: Some(PathBuf::from("rules.yaml")),
        };
        match cmd {
            Commands::Lint { rules, .. } => {
                assert_eq!(rules, Some(PathBuf::from("rules.yaml")));
            }
            _ => panic!("expected Lint"),
        }
    }

}
