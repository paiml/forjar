//! `forjar test` — formatted test runner (FJ-273).

use super::check::{check_resource_filters, localhost_machine, skip_machine};
use super::helpers::*;
use crate::core::{codegen, resolver, types};
use crate::transport;
use std::path::Path;

/// Test result row for the formatted test summary.
pub(crate) struct TestRow {
    pub(crate) resource_id: String,
    pub(crate) machine: String,
    pub(crate) resource_type: String,
    pub(crate) status: String,
    pub(crate) detail: String,
    pub(crate) duration_secs: f64,
}

/// Execute a single test check and return the result row.
fn run_test_check(
    machine: &types::Machine,
    check_script: &str,
    resource_id: &str,
    machine_name: &str,
    resource_type: &str,
) -> (TestRow, bool) {
    use std::time::Instant;

    if machine.is_container_transport() {
        if let Err(e) = transport::container::ensure_container(machine) {
            return (
                TestRow {
                    resource_id: resource_id.to_string(),
                    machine: machine_name.to_string(),
                    resource_type: resource_type.to_string(),
                    status: "FAIL".to_string(),
                    detail: e,
                    duration_secs: 0.0,
                },
                false,
            );
        }
    }

    let t = Instant::now();
    let output = transport::exec_script(machine, check_script);
    let dur = t.elapsed().as_secs_f64();

    let (status, detail, passed) = match output {
        Ok(out) if out.success() => ("pass", String::new(), true),
        Ok(out) => ("FAIL", format!("exit {}", out.exit_code), false),
        Err(e) => ("FAIL", e, false),
    };

    (
        TestRow {
            resource_id: resource_id.to_string(),
            machine: machine_name.to_string(),
            resource_type: resource_type.to_string(),
            status: status.to_string(),
            detail,
            duration_secs: dur,
        },
        passed,
    )
}

/// Print test results as a formatted table.
pub(crate) fn print_test_table(
    results: &[TestRow],
    total_pass: usize,
    total_fail: usize,
    total_skip: usize,
    elapsed: &std::time::Duration,
) {
    println!(
        "{:<30} {:<10} {:<12} {:<8} {:>10}",
        bold("RESOURCE"),
        bold("TYPE"),
        bold("MACHINE"),
        bold("STATUS"),
        bold("DURATION"),
    );
    println!("{}", dim(&"-".repeat(74)));
    for r in results {
        let status_str = match r.status.as_str() {
            "pass" => green("pass"),
            "FAIL" => red("FAIL"),
            _ => yellow(&r.status),
        };
        println!(
            "{:<30} {:<10} {:<12} {:<8} {:>9.3}s",
            r.resource_id, r.resource_type, r.machine, status_str, r.duration_secs
        );
        if !r.detail.is_empty() && r.status == "FAIL" {
            println!("  {}", dim(&r.detail));
        }
    }
    println!("{}", dim(&"-".repeat(74)));
    println!(
        "{} pass, {} fail, {} skip ({:.3}s)",
        green(&total_pass.to_string()),
        if total_fail > 0 {
            red(&total_fail.to_string())
        } else {
            total_fail.to_string()
        },
        total_skip,
        elapsed.as_secs_f64()
    );
}

/// Print test results as JSON.
pub(crate) fn print_test_json(
    results: &[TestRow],
    total_pass: usize,
    total_fail: usize,
    total_skip: usize,
    elapsed: &std::time::Duration,
) -> Result<(), String> {
    let json_results: Vec<_> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "resource": r.resource_id,
                "machine": r.machine,
                "type": r.resource_type,
                "status": r.status,
                "detail": r.detail,
                "duration_seconds": r.duration_secs,
            })
        })
        .collect();
    let report = serde_json::json!({
        "pass": total_pass,
        "fail": total_fail,
        "skip": total_skip,
        "duration_seconds": elapsed.as_secs_f64(),
        "results": json_results,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?
    );
    Ok(())
}

/// FJ-2606: Collect test artifacts to a directory.
pub(crate) fn collect_test_artifacts(
    results: &[TestRow],
    artifact_dir: &Path,
) -> Vec<types::TestArtifact> {
    let _ = std::fs::create_dir_all(artifact_dir);
    let mut artifacts = Vec::new();
    // Write summary JSON
    let summary_path = artifact_dir.join("test-results.json");
    let rows: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "resource": r.resource_id, "machine": r.machine,
                "type": r.resource_type, "status": r.status,
                "detail": r.detail, "duration_seconds": r.duration_secs,
            })
        })
        .collect();
    if let Ok(json_str) = serde_json::to_string_pretty(&rows) {
        let size = json_str.len() as u64;
        let _ = std::fs::write(&summary_path, &json_str);
        artifacts.push(types::TestArtifact {
            name: "test-results.json".into(),
            path: summary_path.display().to_string(),
            content_type: Some("application/json".into()),
            size_bytes: Some(size),
        });
    }
    artifacts
}

/// FJ-273: Dedicated `forjar test` — runs check scripts with a formatted summary table.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_test(
    file: &Path,
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
    group_filter: Option<&str>,
    json: bool,
    verbose: bool,
    runner_opts: &super::check_test_runners::RunnerOpts,
) -> Result<(), String> {
    // FJ-2602/2604: Dispatch to specialized test modes via --group
    match group_filter {
        Some("behavior") => return cmd_test_behavior(file),
        Some("mutation") => return cmd_test_mutation(file, runner_opts),
        Some("convergence") => return cmd_test_convergence(file, runner_opts),
        Some("coverage") => return super::check_test_runners::cmd_test_coverage(file),
        _ => {}
    }

    use std::time::Instant;
    let t0 = Instant::now();

    let config = parse_and_validate(file)?;

    if verbose {
        eprintln!(
            "Testing {} ({} machines, {} resources)",
            config.name,
            config.machines.len(),
            config.resources.len()
        );
    }

    let execution_order = resolver::build_execution_order(&config)?;
    let localhost = localhost_machine();

    let mut results: Vec<TestRow> = Vec::new();
    let mut total_pass = 0usize;
    let mut total_fail = 0usize;
    let mut total_skip = 0usize;

    for resource_id in &execution_order {
        let resource = match config.resources.get(resource_id) {
            Some(r) => r,
            None => continue,
        };

        let (skip, count) =
            check_resource_filters(resource_id, resource, resource_filter, tag_filter);
        if skip {
            if count {
                total_skip += 1;
            }
            continue;
        }

        if let Some(group) = group_filter {
            if resource.resource_group.as_deref() != Some(group) {
                total_skip += 1;
                continue;
            }
        }

        let resolved =
            resolver::resolve_resource_templates(resource, &config.params, &config.machines)?;

        let rtype = format!("{:?}", resource.resource_type).to_lowercase();
        let check_script = match codegen::check_script(&resolved) {
            Ok(s) => s,
            Err(_) => {
                total_skip += 1;
                results.push(TestRow {
                    resource_id: resource_id.clone(),
                    machine: "-".to_string(),
                    resource_type: rtype,
                    status: "skip".to_string(),
                    detail: "no check script".to_string(),
                    duration_secs: 0.0,
                });
                continue;
            }
        };

        for machine_name in resource.machine.to_vec() {
            let machine = config.machines.get(&machine_name).unwrap_or(&localhost);
            if skip_machine(&machine_name, machine_filter, resource, machine) {
                total_skip += 1;
                continue;
            }

            let (row, passed) =
                run_test_check(machine, &check_script, resource_id, &machine_name, &rtype);
            if passed {
                total_pass += 1;
            } else {
                total_fail += 1;
            }
            results.push(row);
        }
    }

    let elapsed = t0.elapsed();

    // FJ-2606: Collect test artifacts when verbose
    if verbose {
        let artifact_dir = file
            .parent()
            .unwrap_or(Path::new("."))
            .join(".forjar-test-artifacts");
        let artifacts = collect_test_artifacts(&results, &artifact_dir);
        if !artifacts.is_empty() {
            eprintln!("Artifacts written to {}", artifact_dir.display());
        }
    }

    if json {
        print_test_json(&results, total_pass, total_fail, total_skip, &elapsed)?;
    } else {
        print_test_table(&results, total_pass, total_fail, total_skip, &elapsed);
    }

    if total_fail > 0 {
        Err(format!("{total_fail} test(s) failed"))
    } else {
        Ok(())
    }
}

/// FJ-2606: Run tests in parallel across machines using thread::scope.
#[allow(dead_code)]
pub(crate) fn run_tests_parallel(
    checks: Vec<(types::Machine, String, String, String, String)>,
) -> Vec<(TestRow, bool)> {
    std::thread::scope(|s| {
        let handles: Vec<_> = checks
            .into_iter()
            .map(|(machine, script, rid, mname, rtype)| {
                s.spawn(move || run_test_check(&machine, &script, &rid, &mname, &rtype))
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    })
}

/// Check file content against expected (exact or BLAKE3 hash).
fn check_file_content(path: &str, expected_content: &str) -> Option<String> {
    let actual = match std::fs::read_to_string(path) {
        Ok(a) => a,
        Err(e) => return Some(format!("cannot read {path}: {e}")),
    };
    if let Some(expected_hash) = expected_content.strip_prefix("blake3:") {
        let actual_hash = blake3::hash(actual.as_bytes()).to_hex().to_string();
        if actual_hash != expected_hash {
            return Some(format!(
                "file content hash mismatch: got blake3:{}, expected {expected_content}",
                &actual_hash[..16]
            ));
        }
    } else if actual.trim() != expected_content.trim() {
        return Some(format!("file content mismatch in {path}"));
    }
    None
}

/// Check that a TCP port is open on localhost.
fn check_port_open(port: u16) -> Option<String> {
    use std::net::TcpStream;
    let addr = format!("127.0.0.1:{port}");
    let timeout = std::time::Duration::from_secs(2);
    if TcpStream::connect_timeout(&addr.parse().unwrap(), timeout).is_err() {
        return Some(format!("port {port} not open on 127.0.0.1"));
    }
    None
}

fn check_exit_code(verify: &crate::core::types::VerifyCommand, code: i32) -> Option<String> {
    let expected = verify.exit_code.unwrap_or(0);
    if code != expected {
        return Some(format!("exit code {code}, expected {expected}"));
    }
    None
}

fn check_stdout(verify: &crate::core::types::VerifyCommand, stdout: &str) -> Option<String> {
    let expected = verify.stdout.as_ref()?;
    if stdout.trim() != expected.trim() {
        return Some(format!(
            "stdout mismatch: got {:?}, expected {:?}",
            stdout.trim(),
            expected.trim()
        ));
    }
    None
}

fn check_stderr(verify: &crate::core::types::VerifyCommand, stderr: &str) -> Option<String> {
    let expected = verify.stderr_contains.as_ref()?;
    if !stderr.contains(expected.as_str()) {
        return Some(format!("stderr missing {:?}", expected));
    }
    None
}

fn check_file_exists(verify: &crate::core::types::VerifyCommand) -> Option<String> {
    let path = verify.file_exists.as_ref()?;
    if !std::path::Path::new(path).exists() {
        return Some(format!("file not found: {path}"));
    }
    None
}

fn check_file_content_assertion(verify: &crate::core::types::VerifyCommand) -> Option<String> {
    let expected_content = verify.file_content.as_ref()?;
    let path = verify.file_exists.as_ref()?;
    check_file_content(path, expected_content)
}

fn check_port_assertion(verify: &crate::core::types::VerifyCommand) -> Option<String> {
    check_port_open(verify.port_open?)
}

/// Check verify assertions against command output. Returns None if all pass.
pub(crate) fn check_verify_assertions(
    verify: &crate::core::types::VerifyCommand,
    code: i32,
    stdout: &str,
    stderr: &str,
) -> Option<String> {
    None.or_else(|| check_exit_code(verify, code))
        .or_else(|| check_stdout(verify, stdout))
        .or_else(|| check_stderr(verify, stderr))
        .or_else(|| check_file_exists(verify))
        .or_else(|| check_file_content_assertion(verify))
        .or_else(|| check_port_assertion(verify))
}

/// Execute a single behavior entry, running verify commands if present.
// Behavior, mutation, and convergence runners extracted to check_test_runners.rs (500-line limit)
pub(crate) use super::check_test_runners::{
    cmd_test_behavior, cmd_test_convergence, cmd_test_mutation,
};
