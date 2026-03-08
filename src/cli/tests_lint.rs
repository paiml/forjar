//! Tests: Linting.

#![allow(unused_imports)]
use super::commands::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::lint::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

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
        let cmd = Commands::Lint(LintArgs {
            file: PathBuf::from("f.yaml"),
            json: false,
            strict: false,
            fix: true,
            rules: None,
            bashrs_version: false,
        });
        match cmd {
            Commands::Lint(LintArgs { fix, .. }) => assert!(fix),
            _ => panic!("expected Lint"),
        }
    }

    // ── FJ-3000: Semicolon chain lint ────────────────────────

    #[test]
    fn test_fj3000_lint_semicolon_chain_warns() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: semi
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  deploy:
    type: task
    machine: m1
    command: "cd /app ; make build ; make install"
"#,
        )
        .unwrap();
        let result = cmd_lint(&config, true, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj3000_lint_semicolon_no_warn_on_ampersand() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: semi-ok
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  deploy:
    type: task
    machine: m1
    command: "cd /app && make build && make install"
"#,
        )
        .unwrap();
        let cfg = parse_and_validate(&config).unwrap();
        let warnings = lint_semicolon_chains(&cfg);
        assert!(warnings.is_empty(), "no warnings for && chains");
    }

    #[test]
    fn test_fj3000_lint_semicolon_quoted_ignored() {
        // Semicolons inside quotes should NOT trigger
        assert!(!has_bare_semicolon("echo 'hello; world'"));
        assert!(!has_bare_semicolon(r#"echo "hello; world""#));
        // But bare semicolons should
        assert!(has_bare_semicolon("cmd1 ; cmd2"));
        assert!(has_bare_semicolon("cmd1;cmd2"));
    }

    #[test]
    fn test_fj3000_lint_semicolon_skips_non_task() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: semi-file
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "a;b;c"
"#,
        )
        .unwrap();
        let cfg = parse_and_validate(&config).unwrap();
        let warnings = lint_semicolon_chains(&cfg);
        assert!(warnings.is_empty(), "file resources should not be linted");
    }

    // ── FJ-3030: nohup LD_LIBRARY_PATH lint ─────────────────

    #[test]
    fn test_fj3030_lint_nohup_ld_path_warns() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: nohup-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  serve:
    type: task
    machine: m1
    command: "nohup /opt/llama/llama-server --port 8080 &"
"#,
        )
        .unwrap();
        let cfg = parse_and_validate(&config).unwrap();
        let warnings = lint_nohup_ld_path(&cfg);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("LD_LIBRARY_PATH"));
    }

    #[test]
    fn test_fj3030_lint_nohup_with_ld_path_no_warn() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: nohup-ok
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  serve:
    type: task
    machine: m1
    command: "LD_LIBRARY_PATH=/opt/llama nohup /opt/llama/llama-server --port 8080 &"
"#,
        )
        .unwrap();
        let cfg = parse_and_validate(&config).unwrap();
        let warnings = lint_nohup_ld_path(&cfg);
        assert!(warnings.is_empty());
    }

    // ── FJ-3040: nohup sleep health check race lint ──────

    #[test]
    fn test_fj3040_lint_nohup_sleep_health_warns() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: race-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  serve:
    type: task
    machine: m1
    command: "nohup /opt/server --port 8080 & sleep 15; curl -sf http://localhost:8080/health"
"#,
        )
        .unwrap();
        let cfg = parse_and_validate(&config).unwrap();
        let warnings = lint_nohup_sleep_health(&cfg);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("health_check"));
    }

    #[test]
    fn test_fj3040_lint_nohup_no_sleep_no_warn() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: no-race
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  serve:
    type: task
    machine: m1
    task_mode: service
    command: "/opt/server --port 8080"
"#,
        )
        .unwrap();
        let cfg = parse_and_validate(&config).unwrap();
        let warnings = lint_nohup_sleep_health(&cfg);
        assert!(warnings.is_empty());
    }

    // ── FJ-2920: OutputWriter adoption ──────────────────

    #[test]
    fn test_fj2920_lint_with_test_writer_clean() {
        use crate::cli::output::TestWriter;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: clean
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
        )
        .unwrap();
        let mut w = TestWriter::new();
        // Use JSON mode to capture structured output via OutputWriter
        cmd_lint_with_writer(&file, true, false, false, &mut w).unwrap();
        let json_out = w.stdout_text();
        assert!(
            json_out.contains("\"findings\""),
            "JSON lint output should be captured by TestWriter: {json_out:?}"
        );
    }

    #[test]
    fn test_fj2920_lint_with_test_writer_warnings() {
        use crate::cli::output::TestWriter;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: warn
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  deploy:
    type: task
    machine: m
    command: "cd /app ; make build"
"#,
        )
        .unwrap();
        let mut w = TestWriter::new();
        cmd_lint_with_writer(&file, false, false, false, &mut w).unwrap();
        assert!(
            w.stderr_text().contains("command uses ';'"),
            "semicolon chain should be warned: {:?}",
            w.stderr_text()
        );
        assert!(
            w.stdout_text().contains("warning(s)"),
            "summary should go to result: {:?}",
            w.stdout_text()
        );
    }

    #[test]
    fn test_fj2920_lint_json_via_writer() {
        use crate::cli::output::TestWriter;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: json
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
        )
        .unwrap();
        let mut w = TestWriter::new();
        cmd_lint_with_writer(&file, true, false, false, &mut w).unwrap();
        let json_out = w.stdout_text();
        assert!(json_out.contains("\"warnings\""), "JSON output: {json_out:?}");
        assert!(json_out.contains("\"findings\""), "JSON output: {json_out:?}");
    }

    #[test]
    fn test_fj374_lint_rules_flag() {
        let cmd = Commands::Lint(LintArgs {
            file: PathBuf::from("f.yaml"),
            json: false,
            strict: false,
            fix: false,
            rules: Some(PathBuf::from("rules.yaml")),
            bashrs_version: false,
        });
        match cmd {
            Commands::Lint(LintArgs { rules, .. }) => {
                assert_eq!(rules, Some(PathBuf::from("rules.yaml")));
            }
            _ => panic!("expected Lint"),
        }
    }
}
