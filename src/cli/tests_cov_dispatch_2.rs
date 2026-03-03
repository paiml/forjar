//! Tests: Coverage for dispatch_misc, dispatch_lock, check (part 2).

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
    fn test_cov_dispatch_misc_state_list_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = dispatch_misc_cmd(
            Commands::StateList(StateListArgs {
                state_dir: dir.path().to_path_buf(),
                machine: None,
                json: true,
            }),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_misc_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_misc_cmd(
            Commands::Rollback(RollbackArgs {
                file: config,
                revision: 1,
                generation: None,
                machine: None,
                dry_run: true,
                yes: false,
                state_dir: state,
            }),
            false,
        );
        // Fails due to no git history, but exercises the dispatch path
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_dispatch_misc_anomaly_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_misc_cmd(
            Commands::Anomaly(AnomalyArgs {
                state_dir: state,
                machine: None,
                min_events: 3,
                json: false,
            }),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_misc_trace_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_misc_cmd(
            Commands::Trace(TraceArgs {
                state_dir: state,
                machine: None,
                json: false,
            }),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_misc_bench_small() {
        let result = dispatch_misc_cmd(
            Commands::Bench(BenchArgs {
                iterations: 2,
                json: false,
            }),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_misc_bench_json() {
        let result = dispatch_misc_cmd(
            Commands::Bench(BenchArgs {
                iterations: 2,
                json: true,
            }),
            false,
        );
        assert!(result.is_ok());
    }

    // ========================================================================
    // 4. dispatch_lock.rs — dispatch_lock_cmd
    // ========================================================================

    #[test]
    fn test_cov_dispatch_lock_verify_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockVerify(LockVerifyArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_verify_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockVerify(LockVerifyArgs {
            state_dir: state,
            json: true,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_info_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockInfo(LockInfoArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_info_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockInfo(LockInfoArgs {
            state_dir: state,
            json: true,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_stats_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockStats(LockStatsArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_stats_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockStats(LockStatsArgs {
            state_dir: state,
            json: true,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_compact_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockCompact(LockCompactArgs {
            state_dir: state,
            yes: false,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_compact_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockCompact(LockCompactArgs {
            state_dir: state,
            yes: false,
            json: true,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_with_data() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::Lock(LockArgs {
            file: config,
            state_dir: state,
            env_file: None,
            workspace: None,
            verify: false,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_json() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::Lock(LockArgs {
            file: config,
            state_dir: state,
            env_file: None,
            workspace: None,
            verify: false,
            json: true,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_backup() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockBackup(LockBackupArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_audit() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockAudit(LockAuditArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_normalize() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockNormalize(LockNormalizeArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_validate() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockValidate(LockValidateArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_integrity() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockIntegrity(LockIntegrityArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_history_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockHistory(LockHistoryArgs {
            state_dir: state,
            json: false,
            limit: 10,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_defrag() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockDefrag(LockDefragArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    // ========================================================================
    // 5. check.rs — uncovered lines (verbose, resource_filter, tag_filter, skip paths)
    // ========================================================================

    #[test]
    fn test_cov_check_verbose_mode() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("check-verbose.txt");
        std::fs::write(&target, "hello").unwrap();
        let config_yaml = format!(
            r#"version: "1.0"
name: check-verbose
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
        let result = cmd_check(&config, None, None, None, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_resource_filter_match() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("check-rf.txt");
        std::fs::write(&target, "hello").unwrap();
        let config_yaml = format!(
            r#"version: "1.0"
name: check-rf
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
        let result = cmd_check(&config, None, Some("f"), None, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_resource_filter_no_match() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("check-rf-no.txt");
        std::fs::write(&target, "hello").unwrap();
        let config_yaml = format!(
            r#"version: "1.0"
name: check-rf-no
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
        let result = cmd_check(&config, None, Some("nonexistent"), None, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_tag_filter_skip() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("check-tag.txt");
        std::fs::write(&target, "hello").unwrap();
        let config_yaml = format!(
            r#"version: "1.0"
name: check-tag
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
        // Filter to a tag that doesn't match
        let result = cmd_check(&config, None, None, Some("db"), false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_audit_trail() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockAuditTrail(LockAuditTrailArgs {
            state_dir: state,
            machine: None,
            json: false,
        }));
        assert!(result.is_ok());
    }
}
