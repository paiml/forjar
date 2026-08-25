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

/// FJ-2602/2604: Decides whether `--group` names a specialized test mode rather
/// than a resource group, and runs it; `None` means "run the default sweep".
/// Exists so `cmd_test` carries only the sweep those modes bypass entirely.
fn dispatch_test_group(
    file: &Path,
    group_filter: Option<&str>,
    runner_opts: &super::check_test_runners::RunnerOpts,
) -> Option<Result<(), String>> {
    match group_filter {
        Some("behavior") => Some(cmd_test_behavior(file)),
        Some("mutation") => Some(cmd_test_mutation(file, runner_opts)),
        Some("convergence") => Some(cmd_test_convergence(file, runner_opts)),
        Some("coverage") => Some(super::check_test_runners::cmd_test_coverage(file)),
        _ => None,
    }
}

/// Running pass/fail/skip counts for one `forjar test` sweep.
#[derive(Default)]
struct TestTally {
    pass: usize,
    fail: usize,
    skip: usize,
}

/// One resource's identity for the sweep: the script to run and how to label
/// the rows it produces.
struct ResourceCheck<'a> {
    resource_id: &'a str,
    resource_type: &'a str,
    script: &'a str,
}

/// A sweep's running output: one row per executed check, plus the tally.
#[derive(Default)]
struct SweepOutcome {
    rows: Vec<TestRow>,
    tally: TestTally,
}

/// The row recorded for a resource whose type generates no check script.
fn no_check_script_row(resource_id: &str, resource_type: &str) -> TestRow {
    TestRow {
        resource_id: resource_id.to_string(),
        machine: "-".to_string(),
        resource_type: resource_type.to_string(),
        status: "skip".to_string(),
        detail: "no check script".to_string(),
        duration_secs: 0.0,
    }
}

/// Runs one resource's check script on every machine it targets, appending a
/// row per execution and folding each outcome into the tally.
fn run_check_on_machines(
    config: &types::ForjarConfig,
    localhost: &types::Machine,
    resource: &types::Resource,
    check: &ResourceCheck<'_>,
    machine_filter: Option<&str>,
    out: &mut SweepOutcome,
) {
    for machine_name in resource.machine.to_vec() {
        let machine = config.machines.get(&machine_name).unwrap_or(localhost);
        if skip_machine(&machine_name, machine_filter, resource, machine) {
            out.tally.skip += 1;
            continue;
        }

        let (row, passed) = run_test_check(
            machine,
            check.script,
            check.resource_id,
            &machine_name,
            check.resource_type,
        );
        if passed {
            out.tally.pass += 1;
        } else {
            out.tally.fail += 1;
        }
        out.rows.push(row);
    }
}

/// Decides whether a resource is excluded before any check script is generated,
/// and whether it counts toward the skip tally (`Some(true)`) or is silent
/// (`Some(false)`). Exists to keep the three exclusion rules out of the sweep.
fn resource_excluded(
    resource_id: &str,
    resource: &types::Resource,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
    group_filter: Option<&str>,
) -> Option<bool> {
    let (skip, count) = check_resource_filters(resource_id, resource, resource_filter, tag_filter);
    if skip {
        return Some(count);
    }

    if let Some(group) = group_filter {
        if resource.resource_group.as_deref() != Some(group) {
            return Some(true);
        }
    }

    // FJ-2725: skip phony resources, exactly as `cli::check` does. A phony
    // target names an ACTION with no artifact to observe, so since FJ-2720
    // made "no evidence of convergence" a failure, testing one reports a
    // permanent FAIL for something that has nothing to observe. `check` and
    // `test` disagreeing about the same resource is the class of defect
    // this release exists to remove — found by dogfooding an imported
    // Makefile, where `all`, `clean` and `test` all failed.
    if resource.phony {
        return Some(true);
    }

    None
}

/// Runs every resource in dependency order, returning one row per check actually
/// executed plus the pass/fail/skip tally. Exists to separate the sweep —
/// filtering, script generation, per-machine execution — from `cmd_test`.
fn run_test_sweep(
    config: &types::ForjarConfig,
    execution_order: &[String],
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
    group_filter: Option<&str>,
) -> Result<(Vec<TestRow>, TestTally), String> {
    let localhost = localhost_machine();
    let mut out = SweepOutcome::default();

    for resource_id in execution_order {
        let Some(resource) = config.resources.get(resource_id) else {
            continue;
        };

        if let Some(counts_as_skip) = resource_excluded(
            resource_id,
            resource,
            resource_filter,
            tag_filter,
            group_filter,
        ) {
            out.tally.skip += usize::from(counts_as_skip);
            continue;
        }

        let resolved =
            resolver::resolve_resource_templates(resource, &config.params, &config.machines)?;

        let rtype = format!("{:?}", resource.resource_type).to_lowercase();
        // A resource type with no check script is skipped, never failed.
        let Ok(check_script) = codegen::check_script(&resolved) else {
            out.tally.skip += 1;
            out.rows.push(no_check_script_row(resource_id, &rtype));
            continue;
        };

        run_check_on_machines(
            config,
            &localhost,
            resource,
            &ResourceCheck {
                resource_id,
                resource_type: &rtype,
                script: &check_script,
            },
            machine_filter,
            &mut out,
        );
    }

    Ok((out.rows, out.tally))
}

/// FJ-2606: Writes the test artifacts next to the config file and announces the
/// directory on stderr. Exists to keep that verbose-only side effect off `cmd_test`.
fn write_test_artifacts(file: &Path, results: &[TestRow]) {
    let artifact_dir = file
        .parent()
        .unwrap_or(Path::new("."))
        .join(".forjar-test-artifacts");
    let artifacts = collect_test_artifacts(results, &artifact_dir);
    if !artifacts.is_empty() {
        eprintln!("Artifacts written to {}", artifact_dir.display());
    }
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
    if let Some(specialized) = dispatch_test_group(file, group_filter, runner_opts) {
        return specialized;
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
    let (results, tally) = run_test_sweep(
        &config,
        &execution_order,
        machine_filter,
        resource_filter,
        tag_filter,
        group_filter,
    )?;

    let elapsed = t0.elapsed();

    if verbose {
        write_test_artifacts(file, &results);
    }

    if json {
        print_test_json(&results, tally.pass, tally.fail, tally.skip, &elapsed)?;
    } else {
        print_test_table(&results, tally.pass, tally.fail, tally.skip, &elapsed);
    }

    if tally.fail > 0 {
        Err(format!("{} test(s) failed", tally.fail))
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
