//! Coverage tests for platform CLI dispatch routes (FJ-2001, FJ-2101, FJ-2104, FJ-2200, FJ-2300).

use super::commands::*;
use super::dispatch_misc::dispatch_misc_cmd;
use std::path::PathBuf;

fn contracts_config() -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;
    f.write_all(b"version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  p:\n    type: package\n    machine: m\n    provider: apt\n    packages: [curl]\n").unwrap();
    f.flush().unwrap();
    f
}

#[test]
fn dispatch_contracts_coverage_routes() {
    let f = contracts_config();
    let result = dispatch_misc_cmd(
        Commands::Contracts(ContractsArgs {
            coverage: true,
            file: f.path().to_path_buf(),
            json: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_contracts_json_routes() {
    let f = contracts_config();
    let result = dispatch_misc_cmd(
        Commands::Contracts(ContractsArgs {
            coverage: true,
            file: f.path().to_path_buf(),
            json: true,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_contracts_no_flag_still_works() {
    let f = contracts_config();
    let result = dispatch_misc_cmd(
        Commands::Contracts(ContractsArgs {
            coverage: false,
            file: f.path().to_path_buf(),
            json: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_oci_pack_missing_dir_errors() {
    let result = dispatch_misc_cmd(
        Commands::OciPack(OciPackArgs {
            dir: PathBuf::from("/nonexistent/dir"),
            tag: "test:latest".into(),
            output: PathBuf::from("/tmp/oci-out"),
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}

#[test]
fn dispatch_oci_pack_json_with_existing_dir() {
    let result = dispatch_misc_cmd(
        Commands::OciPack(OciPackArgs {
            dir: PathBuf::from("/tmp"),
            tag: "myapp:v1".into(),
            output: PathBuf::from("/tmp/oci-out"),
            json: true,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_oci_pack_text_with_existing_dir() {
    let result = dispatch_misc_cmd(
        Commands::OciPack(OciPackArgs {
            dir: PathBuf::from("/tmp"),
            tag: "myapp:v1".into(),
            output: PathBuf::from("/tmp/oci-out"),
            json: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_state_query_text() {
    let result = dispatch_misc_cmd(
        Commands::StateQuery(QueryArgs {
            query: Some("bash".into()),
            state_dir: PathBuf::from("/nonexistent"),
            resource_type: None,
            history: false,
            drift: false,
            health: false,
            timing: false,
            churn: false,
            reversibility: false,
            git_history: false,
            json: false,
            csv: false,
            sql: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_state_query_json_with_filters() {
    let result = dispatch_misc_cmd(
        Commands::StateQuery(QueryArgs {
            query: Some("nginx".into()),
            state_dir: PathBuf::from("/nonexistent"),
            resource_type: Some("package".into()),
            history: true,
            drift: true,
            health: false,
            timing: false,
            churn: false,
            reversibility: false,
            git_history: false,
            json: true,
            csv: false,
            sql: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_state_query_csv() {
    let result = dispatch_misc_cmd(
        Commands::StateQuery(QueryArgs {
            query: Some("curl".into()),
            state_dir: PathBuf::from("/nonexistent"),
            resource_type: None,
            history: false,
            drift: false,
            health: false,
            timing: false,
            churn: false,
            reversibility: false,
            git_history: false,
            json: false,
            csv: true,
            sql: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_state_query_health() {
    let result = dispatch_misc_cmd(
        Commands::StateQuery(QueryArgs {
            query: None,
            state_dir: PathBuf::from("/nonexistent"),
            resource_type: None,
            history: false,
            drift: false,
            health: true,
            timing: false,
            churn: false,
            reversibility: false,
            git_history: false,
            json: false,
            csv: false,
            sql: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_state_query_health_json() {
    let result = dispatch_misc_cmd(
        Commands::StateQuery(QueryArgs {
            query: None,
            state_dir: PathBuf::from("/nonexistent"),
            resource_type: None,
            history: false,
            drift: false,
            health: true,
            timing: false,
            churn: false,
            reversibility: false,
            git_history: false,
            json: true,
            csv: false,
            sql: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_state_query_drift() {
    let result = dispatch_misc_cmd(
        Commands::StateQuery(QueryArgs {
            query: None,
            state_dir: PathBuf::from("/nonexistent"),
            resource_type: None,
            history: false,
            drift: true,
            health: false,
            timing: false,
            churn: false,
            reversibility: false,
            git_history: false,
            json: false,
            csv: false,
            sql: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_state_query_churn() {
    let result = dispatch_misc_cmd(
        Commands::StateQuery(QueryArgs {
            query: None,
            state_dir: PathBuf::from("/nonexistent"),
            resource_type: None,
            history: false,
            drift: false,
            health: false,
            timing: false,
            churn: true,
            reversibility: false,
            git_history: false,
            json: false,
            csv: false,
            sql: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_state_query_sql() {
    let result = dispatch_misc_cmd(
        Commands::StateQuery(QueryArgs {
            query: Some("bash".into()),
            state_dir: PathBuf::from("/nonexistent"),
            resource_type: Some("file".into()),
            history: false,
            drift: false,
            health: false,
            timing: false,
            churn: false,
            reversibility: false,
            git_history: false,
            json: false,
            csv: false,
            sql: true,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_logs_gc() {
    let result = dispatch_misc_cmd(
        Commands::Logs(LogsArgs {
            state_dir: PathBuf::from("/nonexistent"),
            machine: None,
            run: None,
            resource: None,
            failures: false,
            script: false,
            all_machines: false,
            follow: false,
            gc: true,
            dry_run: false,
            keep_failed: false,
            json: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_logs_follow() {
    let result = dispatch_misc_cmd(
        Commands::Logs(LogsArgs {
            state_dir: PathBuf::from("/nonexistent"),
            machine: Some("intel".into()),
            run: None,
            resource: None,
            failures: true,
            script: false,
            all_machines: false,
            follow: true,
            gc: false,
            dry_run: false,
            keep_failed: false,
            json: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_logs_filter() {
    let result = dispatch_misc_cmd(
        Commands::Logs(LogsArgs {
            state_dir: PathBuf::from("/nonexistent"),
            machine: Some("jetson".into()),
            run: Some("r-abc123".into()),
            resource: None,
            failures: true,
            script: false,
            all_machines: false,
            follow: false,
            gc: false,
            dry_run: false,
            keep_failed: false,
            json: false,
        }),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn dispatch_build_missing_config_errors() {
    let result = dispatch_misc_cmd(
        Commands::Build(BuildArgs {
            file: PathBuf::from("/nonexistent/forjar.yaml"),
            resource: "nginx-image".into(),
            load: false,
            push: false,
            far: false,
            sandbox: false,
            json: false,
        }),
        false,
    );
    assert!(result.is_err());
}

// --- Test artifact collection (FJ-2606) ---

#[test]
fn collect_test_artifacts_writes_json() {
    use super::check_test::{collect_test_artifacts, TestRow};

    let dir = tempfile::tempdir().expect("tmpdir");
    let results = vec![
        TestRow {
            resource_id: "pkg-curl".into(),
            machine: "intel".into(),
            resource_type: "package".into(),
            status: "pass".into(),
            detail: String::new(),
            duration_secs: 0.12,
        },
        TestRow {
            resource_id: "svc-nginx".into(),
            machine: "intel".into(),
            resource_type: "service".into(),
            status: "FAIL".into(),
            detail: "exit 1".into(),
            duration_secs: 0.55,
        },
    ];
    let artifacts = collect_test_artifacts(&results, dir.path());
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].name, "test-results.json");
    assert!(artifacts[0].size_bytes.unwrap() > 0);
    let content = std::fs::read_to_string(dir.path().join("test-results.json")).unwrap();
    assert!(content.contains("pkg-curl"));
    assert!(content.contains("FAIL"));
}

#[test]
fn parallel_runner_returns_results() {
    use super::check_test::run_tests_parallel;
    // Empty input → empty output
    let results = run_tests_parallel(vec![]);
    assert!(results.is_empty());
}
