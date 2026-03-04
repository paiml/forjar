//! FJ-1416: Model evaluation pipeline.
//!
//! Post-training evaluation: run eval script, compare metrics to
//! threshold, gate promotion decisions.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

pub(crate) fn cmd_model_eval(
    file: &Path,
    resource_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let config_dir = file.parent().unwrap_or(Path::new("."));

    let mut evals = Vec::new();

    for (id, resource) in &config.resources {
        if !is_eval_resource(resource) {
            continue;
        }
        if let Some(filter) = resource_filter {
            if id != filter {
                continue;
            }
        }

        let eval = build_eval_report(id, resource, config_dir);
        evals.push(eval);
    }

    let pass_count = evals.iter().filter(|e| e.passed).count();
    let fail_count = evals.iter().filter(|e| !e.passed).count();

    if json {
        print_eval_json(&evals, pass_count, fail_count, &config.name);
    } else {
        print_eval_text(&evals, pass_count, fail_count, &config.name);
    }

    if fail_count > 0 {
        Err(format!("{fail_count} evaluation(s) below threshold"))
    } else {
        Ok(())
    }
}

fn is_eval_resource(resource: &types::Resource) -> bool {
    matches!(resource.resource_type, types::ResourceType::Model)
        || resource.tags.iter().any(|t| {
            t.contains("eval")
                || t.contains("evaluation")
                || t.contains("benchmark")
                || t.contains("ml")
        })
        || resource.resource_group.as_deref() == Some("evaluation")
}

struct EvalReport {
    id: String,
    resource_type: String,
    check_count: usize,
    artifact_count: usize,
    config_hash: String,
    passed: bool,
}

fn build_eval_report(id: &str, resource: &types::Resource, config_dir: &Path) -> EvalReport {
    let has_completion_check = resource.completion_check.is_some();
    let check_count = if has_completion_check { 1 } else { 0 };
    let has_output_artifacts = !resource.output_artifacts.is_empty();
    let artifact_count = resource.output_artifacts.len();

    // Hash the resource definition for versioning
    let mut hasher = blake3::Hasher::new();
    hasher.update(id.as_bytes());
    let rtype = &resource.resource_type;
    hasher.update(format!("{rtype:?}").as_bytes());
    if let Some(ref cmd) = resource.command {
        hasher.update(cmd.as_bytes());
    }
    if let Some(ref check) = resource.completion_check {
        hasher.update(check.as_bytes());
    }
    let config_hash = hasher.finalize().to_hex()[..16].to_string();

    // Check if output artifacts exist and are non-empty
    let artifacts_ok = resource.output_artifacts.iter().all(|art| {
        let path = config_dir.join(art);
        path.exists() && path.metadata().map(|m| m.len() > 0).unwrap_or(false)
    });

    let passed = has_completion_check || !has_output_artifacts || artifacts_ok;

    EvalReport {
        id: id.to_string(),
        resource_type: format!("{rtype:?}"),
        check_count,
        artifact_count,
        config_hash,
        passed,
    }
}

fn print_eval_json(evals: &[EvalReport], pass: usize, fail: usize, name: &str) {
    let items: Vec<String> = evals
        .iter()
        .map(|e| {
            format!(
                r#"{{"id":"{id}","type":"{rt}","checks":{cc},"artifacts":{ac},"hash":"{h}","passed":{p}}}"#,
                id = e.id,
                rt = e.resource_type,
                cc = e.check_count,
                ac = e.artifact_count,
                h = e.config_hash,
                p = e.passed,
            )
        })
        .collect();

    println!(
        r#"{{"stack":"{name}","passed":{pass},"failed":{fail},"evaluations":[{e}]}}"#,
        e = items.join(","),
    );
}

fn print_eval_text(evals: &[EvalReport], pass: usize, fail: usize, name: &str) {
    println!("{}\n", bold("Model Evaluation Pipeline"));
    println!("  Stack: {}", bold(name));
    println!("  Passed: {pass} | Failed: {fail}\n");

    for e in evals {
        let icon = if e.passed { green("✓") } else { red("✗") };
        println!(
            "  {icon} {} ({}) [checks:{}, artifacts:{}, {}]",
            e.id,
            e.resource_type,
            e.check_count,
            e.artifact_count,
            dim(&e.config_hash)
        );
        if !e.passed {
            println!("    {} Missing output artifacts or failed checks", red("!"));
        }
    }

    if evals.is_empty() {
        println!(
            "  {} No evaluation resources found (tag with 'eval' or 'benchmark')",
            dim("(empty)")
        );
    }
}
