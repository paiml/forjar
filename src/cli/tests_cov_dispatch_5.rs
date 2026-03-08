//! Coverage tests for dispatch_misc routing (FJ-1372).
//! Each test exercises a dispatch route with minimal valid input
//! to cover the routing logic in dispatch_misc_cmd().

use super::commands::*;
use super::dispatch_misc::dispatch_misc_cmd;
use std::path::PathBuf;

#[test]
fn dispatch_history_routes_correctly() {
    let result = dispatch_misc_cmd(
        Commands::History(HistoryArgs {
            state_dir: PathBuf::from("/nonexistent/state"),
            machine: None,
            limit: 10,
            json: false,
            since: None,
            resource: None,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_history_resource_routes() {
    let result = dispatch_misc_cmd(
        Commands::History(HistoryArgs {
            state_dir: PathBuf::from("/nonexistent/state"),
            machine: None,
            limit: 10,
            json: false,
            since: None,
            resource: Some("my-resource".to_string()),
        }),
        false,
    );
    // Routing works — may succeed with "No event logs" for nonexistent dir
    let _ = result;
}

#[test]
fn dispatch_show_routes() {
    let result = dispatch_misc_cmd(
        Commands::Show(ShowArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            resource: None,
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_import_routes() {
    let result = dispatch_misc_cmd(
        Commands::Import(ImportArgs {
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            name: None,
            output: PathBuf::from("/tmp/import-test.yaml"),
            scan: vec!["file".to_string()],
            smart: false,
        }),
        false,
    );
    // Import tries to connect — will fail on non-existent host
    let _ = result;
}

#[test]
fn dispatch_diff_routes() {
    let result = dispatch_misc_cmd(
        Commands::Diff(DiffArgs {
            from: PathBuf::from("/nonexistent/a.yaml"),
            to: PathBuf::from("/nonexistent/b.yaml"),
            machine: None,
            resource: None,
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_check_routes() {
    let result = dispatch_misc_cmd(
        Commands::Check(CheckArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            machine: None,
            resource: None,
            tag: None,
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_lint_routes() {
    let result = dispatch_misc_cmd(
        Commands::Lint(LintArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            json: false,
            strict: false,
            fix: false,
            rules: None,
            bashrs_version: false,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_rollback_routes() {
    let result = dispatch_misc_cmd(
        Commands::Rollback(RollbackArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            revision: 1,
            generation: None,
            machine: None,
            dry_run: false,
            yes: false,
            state_dir: PathBuf::from("/nonexistent/state"),
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_anomaly_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_misc_cmd(
        Commands::Anomaly(AnomalyArgs {
            state_dir: dir.path().to_path_buf(),
            machine: None,
            min_events: 5,
            json: false,
        }),
        false,
    );
    // Empty state dir → ok (no anomalies)
    assert!(result.is_ok());
}

#[test]
fn dispatch_trace_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_misc_cmd(
        Commands::Trace(TraceArgs {
            state_dir: dir.path().to_path_buf(),
            machine: None,
            json: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_state_list_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_misc_cmd(
        Commands::StateList(StateListArgs {
            state_dir: dir.path().to_path_buf(),
            machine: None,
            json: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_state_mv_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_misc_cmd(
        Commands::StateMv(StateMvArgs {
            old_id: "old".to_string(),
            new_id: "new".to_string(),
            state_dir: dir.path().to_path_buf(),
            machine: None,
        }),
        false,
    );
    // Fails because no state files exist
    let _ = result;
}

#[test]
fn dispatch_state_rm_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_misc_cmd(
        Commands::StateRm(StateRmArgs {
            resource_id: "test-resource".to_string(),
            state_dir: dir.path().to_path_buf(),
            machine: None,
            force: false,
        }),
        false,
    );
    let _ = result;
}

#[test]
fn dispatch_output_routes() {
    let result = dispatch_misc_cmd(
        Commands::Output(OutputArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            key: None,
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_policy_routes() {
    let result = dispatch_misc_cmd(
        Commands::Policy(PolicyArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_explain_routes() {
    let result = dispatch_misc_cmd(
        Commands::Explain(ExplainArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            resource: "my-resource".to_string(),
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_env_routes() {
    let result = dispatch_misc_cmd(
        Commands::Env(EnvArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            json: false,
        }),
        false,
    );
    // cmd_env prints env info even without valid config — routing works
    let _ = result;
}

#[test]
fn dispatch_test_routes() {
    let result = dispatch_misc_cmd(
        Commands::Test(TestArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            machine: None,
            resource: None,
            tag: None,
            group: None,
            json: false,
            sandbox: "pepita".to_string(),
            parallel: 4,
            pairs: false,
            mutations: 50,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_schema_routes() {
    let result = dispatch_misc_cmd(Commands::Schema, false);
    assert!(result.is_ok());
}

#[test]
fn dispatch_completion_routes() {
    // clap_complete::generate recursively traverses all 89 command variants;
    // needs larger stack in containers with limited default thread stack size.
    let result = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            dispatch_misc_cmd(
                Commands::Completion(CompletionArgs {
                    shell: CompletionShell::Bash,
                }),
                false,
            )
        })
        .expect("spawn thread")
        .join()
        .expect("join thread");
    assert!(result.is_ok());
}

#[test]
fn dispatch_plan_compact_routes() {
    let result = dispatch_misc_cmd(
        Commands::PlanCompact(PlanCompactArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            state_dir: PathBuf::from("/nonexistent/state"),
            machine: None,
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_compliance_routes() {
    let result = dispatch_misc_cmd(
        Commands::Compliance(ComplianceArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_suggest_routes() {
    let result = dispatch_misc_cmd(
        Commands::Suggest(SuggestArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_compare_routes() {
    let result = dispatch_misc_cmd(
        Commands::Compare(CompareArgs {
            file1: PathBuf::from("/nonexistent/a.yaml"),
            file2: PathBuf::from("/nonexistent/b.yaml"),
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_env_diff_routes() {
    let result = dispatch_misc_cmd(
        Commands::EnvDiff(EnvDiffArgs {
            env1: "prod".to_string(),
            env2: "staging".to_string(),
            state_dir: PathBuf::from("/nonexistent/state"),
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_score_routes() {
    let result = dispatch_misc_cmd(
        Commands::Score(ScoreArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            status: "converged".to_string(),
            idempotency: "full".to_string(),
            budget_ms: 0,
            json: false,
            state_dir: PathBuf::from("state"),
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_bench_routes() {
    let result = dispatch_misc_cmd(
        Commands::Bench(BenchArgs {
            iterations: 1,
            json: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_inventory_routes() {
    let result = dispatch_misc_cmd(
        Commands::Inventory(InventoryArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_retry_failed_routes() {
    let result = dispatch_misc_cmd(
        Commands::RetryFailed(RetryFailedArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            state_dir: PathBuf::from("/nonexistent/state"),
            params: vec![],
            timeout: None,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_rolling_routes() {
    let result = dispatch_misc_cmd(
        Commands::Rolling(RollingArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            state_dir: PathBuf::from("/nonexistent/state"),
            batch_size: 2,
            params: vec![],
            timeout: None,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_canary_routes() {
    let result = dispatch_misc_cmd(
        Commands::Canary(CanaryArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            state_dir: PathBuf::from("/nonexistent/state"),
            machine: "web-1".to_string(),
            auto_proceed: false,
            params: vec![],
            timeout: None,
        }),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn dispatch_audit_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_misc_cmd(
        Commands::Audit(AuditArgs {
            state_dir: dir.path().to_path_buf(),
            machine: None,
            limit: 10,
            json: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_mcp_schema_routes() {
    let result = dispatch_misc_cmd(Commands::Mcp(McpArgs { schema: true }), false);
    assert!(result.is_ok());
}

#[test]
fn dispatch_unknown_cmd_returns_error() {
    let result = dispatch_misc_cmd(
        Commands::Init(InitArgs {
            path: PathBuf::from("."),
        }),
        false,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown"));
}
