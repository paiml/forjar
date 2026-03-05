//! `forjar test` — formatted test runner (FJ-273).

use super::check::{check_resource_filters, localhost_machine, skip_machine};
use super::helpers::*;
use crate::core::{codegen, resolver, types};
use crate::transport;
use std::path::Path;

/// Test result row for the formatted test summary.
struct TestRow {
    resource_id: String,
    machine: String,
    resource_type: String,
    status: String,
    detail: String,
    duration_secs: f64,
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
fn print_test_table(
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
fn print_test_json(
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
) -> Result<(), String> {
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
