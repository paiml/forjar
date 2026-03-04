//! Tests: Coverage for check, observe, apply (part 3).

#![allow(unused_imports)]
use super::apply::*;
use super::check::*;
use super::commands::*;
use super::destroy::*;
use super::dispatch::*;
use super::dispatch_lock::*;
use super::dispatch_misc::*;
use super::helpers::*;
use super::helpers_state::*;
use super::infra::*;
use super::observe::*;
use super::test_fixtures::*;
use crate::core::{executor, parser, planner, resolver, state, types};
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    fn write_yaml(dir: &Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
        p
    }

    fn minimal_config_yaml() -> &'static str {
        r#"version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: local
    path: /tmp/forjar-cov-dispatch-test.txt
    content: "hello"
"#
    }

    #[test]
    fn test_cov_check_tag_filter_match() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("check-tag-match.txt");
        std::fs::write(&target, "hello").unwrap();
        let config_yaml = format!(
            r#"version: "1.0"
name: check-tag-match
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {}
    content: hello
    tags: [web]
"#,
            target.display()
        );
        let config = write_yaml(dir.path(), "forjar.yaml", &config_yaml);
        let result = cmd_check(&config, None, None, Some("web"), false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_machine_filter_skip() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("check-mf.txt");
        std::fs::write(&target, "hello").unwrap();
        let config_yaml = format!(
            r#"version: "1.0"
name: check-mf
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {}
    content: hello
"#,
            target.display()
        );
        let config = write_yaml(dir.path(), "forjar.yaml", &config_yaml);
        let result = cmd_check(
            &config,
            Some("nonexistent-machine"),
            None,
            None,
            false,
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_json_with_failures() {
        let dir = tempfile::tempdir().unwrap();
        let config_yaml = r#"version: "1.0"
name: check-json-fail
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: local
    provider: apt
    packages: [nonexistent-package-xyz-12345]
"#;
        let config = write_yaml(dir.path(), "forjar.yaml", config_yaml);
        let result = cmd_check(&config, None, None, None, true, false);
        // Package check may pass or fail depending on system; exercise the path
        let _ = result;
    }

    #[test]
    fn test_cov_check_multiple_resources() {
        let dir = tempfile::tempdir().unwrap();
        let t1 = dir.path().join("multi-check-1.txt");
        let t2 = dir.path().join("multi-check-2.txt");
        std::fs::write(&t1, "one").unwrap();
        std::fs::write(&t2, "two").unwrap();
        let config_yaml = format!(
            r#"version: "1.0"
name: multi-check
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: local
    path: {}
    content: one
  f2:
    type: file
    machine: local
    path: {}
    content: two
"#,
            t1.display(),
            t2.display()
        );
        let config = write_yaml(dir.path(), "forjar.yaml", &config_yaml);
        let result = cmd_check(&config, None, None, None, false, true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 6. observe.rs — run_watch_apply (line 332, 0%)
    // ========================================================================

    #[test]
    fn test_cov_watch_requires_yes() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_watch(&config, &state, 2, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--yes"));
    }

    #[test]
    fn test_cov_watch_dispatch_requires_yes() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_misc_cmd(
            Commands::Watch(WatchArgs {
                file: config,
                state_dir: state,
                interval: 2,
                apply: true,
                yes: false,
            }),
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_anomaly_json_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_anomaly(&state, None, 3, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_anomaly_with_events_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let machine_dir = state.join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();

        let mut events = String::new();
        for i in 0..5 {
            let event = serde_json::to_string(&types::TimestampedEvent {
                ts: format!("2026-02-25T{i:02}:00:00Z"),
                event: types::ProvenanceEvent::ResourceConverged {
                    machine: "web".to_string(),
                    resource: "pkg".to_string(),
                    duration_seconds: 1.0,
                    hash: "blake3:abc".to_string(),
                },
            })
            .unwrap();
            events.push_str(&event);
            events.push('\n');
        }
        // Add failures
        for i in 0..3 {
            let event = serde_json::to_string(&types::TimestampedEvent {
                ts: format!("2026-02-25T{:02}:00:00Z", i + 10),
                event: types::ProvenanceEvent::ResourceFailed {
                    machine: "web".to_string(),
                    resource: "pkg".to_string(),
                    error: "install failed".to_string(),
                },
            })
            .unwrap();
            events.push_str(&event);
            events.push('\n');
        }
        std::fs::write(machine_dir.join("events.jsonl"), &events).unwrap();

        let result = cmd_anomaly(&state, None, 3, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_trace_json_with_spans() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let mut session = crate::tripwire::tracer::TraceSession::start("r-cov-test");
        session.record_noop("r1", "file", "m1");
        session.record_span(
            "r2",
            "package",
            "m1",
            "create",
            std::time::Duration::from_millis(50),
            0,
            None,
        );
        session.record_span(
            "r3",
            "service",
            "m1",
            "update",
            std::time::Duration::from_millis(200),
            1,
            None,
        );
        crate::tripwire::tracer::write_trace(&state, "m1", &session).unwrap();

        let result = cmd_trace(&state, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_trace_text_with_multiple_spans() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let mut session = crate::tripwire::tracer::TraceSession::start("r-multi");
        session.record_noop("r1", "file", "m1");
        session.record_span(
            "r2",
            "file",
            "m1",
            "create",
            std::time::Duration::from_millis(100),
            0,
            None,
        );
        crate::tripwire::tracer::write_trace(&state, "m1", &session).unwrap();

        let result = cmd_trace(&state, None, false);
        assert!(result.is_ok());
    }
}
