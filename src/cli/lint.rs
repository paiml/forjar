//! Linting.

use super::commands::LintGateArgs;
use super::helpers::*;
use crate::core::quality_gate::{self, GateThresholds};
use crate::core::{codegen, types};
use std::path::Path;

// `lint --fix`'s machinery lives in `lint_fix.rs` (the 500-line ceiling split it
// out). Re-exported because every caller reaches it through
// `use super::lint::*`, and moving the module should not move the import.
pub(crate) use super::lint_fix::lint_auto_fix;

fn lint_unused_machines(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut referenced = std::collections::HashSet::new();
    for resource in config.resources.values() {
        for m in resource.machine.iter() {
            referenced.insert(m.to_owned());
        }
    }
    for machine_name in config.machines.keys() {
        if !referenced.contains(machine_name) {
            warnings.push(format!(
                "machine '{machine_name}' is defined but not referenced by any resource"
            ));
        }
    }
    warnings
}

fn lint_untagged_resources(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut untagged = 0usize;
    for (id, resource) in &config.resources {
        if resource.tags.is_empty() {
            untagged += 1;
            if config.resources.len() > 3 {
                warnings.push(format!("resource '{id}' has no tags"));
            }
        }
    }
    if untagged > 0 && config.resources.len() > 3 && untagged == config.resources.len() {
        warnings.retain(|w| !w.starts_with("resource '") || !w.ends_with("has no tags"));
        warnings.push(format!(
            "all {untagged} resources have no tags — consider adding tags for selective filtering"
        ));
    }
    warnings
}

fn lint_duplicate_content(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut content_map: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for (id, resource) in &config.resources {
        if let Some(ref content) = resource.content {
            content_map
                .entry(content.as_str())
                .or_default()
                .push(id.as_str());
        }
    }
    for ids in content_map.values() {
        if ids.len() > 1 {
            warnings.push(format!(
                "resources {} have identical content — consider using a recipe or template",
                ids.join(", ")
            ));
        }
    }
    warnings
}

fn lint_dependency_issues(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    for (id, resource) in &config.resources {
        for dep in &resource.depends_on {
            if !config.resources.contains_key(dep) {
                warnings.push(format!(
                    "resource '{id}' depends on '{dep}' which does not exist"
                ));
            }
        }
        let my_machines: std::collections::HashSet<&str> = resource.machine.iter().collect();
        for dep in &resource.depends_on {
            if let Some(dep_resource) = config.resources.get(dep) {
                let dep_machines: std::collections::HashSet<&str> =
                    dep_resource.machine.iter().collect();
                if my_machines.is_disjoint(&dep_machines) {
                    warnings.push(format!(
                        "resource '{id}' depends on '{dep}' but they target different machines"
                    ));
                }
            }
        }
    }
    warnings
}

fn lint_empty_packages(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    for (id, resource) in &config.resources {
        if resource.resource_type == types::ResourceType::Package && resource.packages.is_empty() {
            warnings.push(format!("package resource '{id}' has no packages listed"));
        }
    }
    warnings
}

/// Strict rules that inspect resources: root-owned files, then untagged resources.
fn lint_strict_resource_rules(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    for (id, resource) in &config.resources {
        if resource.resource_type == types::ResourceType::File
            && resource.owner.as_deref() == Some("root")
            && !resource.tags.iter().any(|t| t == "system")
        {
            warnings.push(format!(
                "strict: file '{id}' is owned by root — tag as 'system' or use a non-root owner"
            ));
        }
    }
    for (id, resource) in &config.resources {
        if resource.tags.is_empty() {
            warnings.push(format!("strict: resource '{id}' has no tags"));
        }
    }
    warnings
}

/// Strict rules that inspect machines: privileged containers, then missing ssh keys.
fn lint_strict_machine_rules(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    for (name, machine) in &config.machines {
        if let Some(ref container) = machine.container {
            if container.privileged {
                warnings.push(format!(
                    "strict: machine '{name}' uses privileged container mode"
                ));
            }
        }
    }
    for (name, machine) in &config.machines {
        if machine.addr != "127.0.0.1"
            && machine.addr != "localhost"
            && machine.addr != "container"
            && machine.ssh_key.is_none()
        {
            warnings.push(format!(
                "strict: machine '{name}' has no ssh_key configured"
            ));
        }
    }
    warnings
}

fn lint_strict_rules(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = lint_strict_resource_rules(config);
    warnings.extend(lint_strict_machine_rules(config));
    warnings
}

/// FJ-3000: Detect semicolon-chained commands in task resources.
///
/// Semicolons mask exit codes — `cmd1 ; cmd2` runs cmd2 even if cmd1 fails.
/// Under `set -euo pipefail`, only the last command's exit code matters.
/// Warns users to use `&&` or multiline `|` instead.
pub fn lint_semicolon_chains(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    for (id, resource) in &config.resources {
        if resource.resource_type != types::ResourceType::Task {
            continue;
        }
        if let Some(ref cmd) = resource.command {
            // Skip multiline commands (already using heredoc/script style)
            if cmd.contains('\n') {
                continue;
            }
            // Detect bare semicolons (not inside quotes)
            if has_bare_semicolon(cmd) {
                warnings.push(format!(
                    "task '{id}': command uses ';' which masks exit codes — \
                     use '&&' to fail fast or multiline '|' block"
                ));
            }
        }
    }
    warnings
}

/// Check if a command string contains a bare semicolon (not inside quotes).
pub fn has_bare_semicolon(cmd: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut prev = '\0';
    for ch in cmd.chars() {
        match ch {
            '\'' if !in_double && prev != '\\' => in_single = !in_single,
            '"' if !in_single && prev != '\\' => in_double = !in_double,
            ';' if !in_single && !in_double => return true,
            _ => {}
        }
        prev = ch;
    }
    false
}

/// FJ-3030: Detect nohup launching binaries without LD_LIBRARY_PATH.
///
/// When `nohup /absolute/path/binary` is used, the child process may fail
/// at runtime if shared libraries are in non-standard paths.
/// Warns if nohup uses an absolute path and LD_LIBRARY_PATH isn't set.
pub fn lint_nohup_ld_path(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    for (id, resource) in &config.resources {
        if resource.resource_type != types::ResourceType::Task {
            continue;
        }
        if let Some(ref cmd) = resource.command {
            // Check for nohup with absolute path binary
            if let Some(pos) = cmd.find("nohup ") {
                let after = &cmd[pos + 6..];
                let binary = after.split_whitespace().next().unwrap_or("");
                if binary.starts_with('/') && !cmd.contains("LD_LIBRARY_PATH") {
                    warnings.push(format!(
                        "task '{id}': nohup launches '{}' without LD_LIBRARY_PATH — \
                         if binary uses non-standard .so paths, set LD_LIBRARY_PATH before nohup",
                        binary
                    ));
                }
            }
        }
    }
    warnings
}

/// FJ-3040: Detect nohup + fixed sleep + health check anti-pattern.
///
/// Pattern: `nohup ... & sleep N; curl` or similar fixed-duration waits
/// before health checks. Suggests using `health_check:` field instead.
pub fn lint_nohup_sleep_health(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    for (id, resource) in &config.resources {
        if resource.resource_type != types::ResourceType::Task {
            continue;
        }
        if let Some(ref cmd) = resource.command {
            // Pattern: nohup...&...sleep...curl/wget/health
            let has_nohup = cmd.contains("nohup ");
            let has_sleep = cmd.contains("sleep ");
            let has_health_probe =
                cmd.contains("curl ") || cmd.contains("wget ") || cmd.contains("/health");
            if has_nohup && has_sleep && has_health_probe {
                warnings.push(format!(
                    "task '{id}': nohup + sleep + health probe is fragile — \
                     use task_mode: service with health_check: field for retry-based polling"
                ));
            }
        }
    }
    warnings
}

/// Build a set of line numbers that fall inside heredoc blocks (content, not shell).
fn heredoc_line_set(script: &str) -> std::collections::HashSet<usize> {
    let mut inside = std::collections::HashSet::new();
    let mut in_heredoc = false;
    for (i, line) in script.lines().enumerate() {
        let trimmed = line.trim();
        if in_heredoc {
            if trimmed == "FORJAR_EOF" || trimmed == "FORJAR_SUDO" {
                in_heredoc = false;
            } else {
                inside.insert(i + 1); // 1-based line numbers
            }
        } else if trimmed.contains("<<'FORJAR_EOF'") || trimmed.contains("<<'FORJAR_SUDO'") {
            in_heredoc = true;
        }
    }
    inside
}

/// How many bashrs findings a lint sweep saw, by severity. Errors are also
/// reported individually; the lower severities are only ever counted.
#[derive(Default)]
struct ScriptLintTally {
    errors: usize,
    warnings: usize,
}

/// Lints one generated script, pushing a message per error-severity finding and
/// folding every finding into `tally`.
fn lint_generated_script(
    id: &str,
    kind: &str,
    script: &str,
    warnings: &mut Vec<String>,
    tally: &mut ScriptLintTally,
) {
    use bashrs::linter::Severity;

    let heredoc_lines = heredoc_line_set(script);
    for d in &crate::core::purifier::lint_script(script).diagnostics {
        // SC1xxx rules have false positives on generated scripts (e.g. grep
        // char classes parsed as test expressions); purifier::validate_script
        // already filters these. Anything inside a heredoc is file data, not
        // shell, so it is not the generator's to answer for either.
        if d.code.starts_with("SC1") || heredoc_lines.contains(&d.span.start_line) {
            continue;
        }
        match d.severity {
            Severity::Error => {
                tally.errors += 1;
                warnings.push(format!(
                    "bashrs: {}/{} [{}] {}",
                    id, kind, d.code, d.message
                ));
            }
            _ => tally.warnings += 1,
        }
    }
}

fn lint_scripts(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut tally = ScriptLintTally::default();
    for (id, resource) in &config.resources {
        for (kind, result) in [
            ("check", codegen::check_script(resource)),
            ("apply", codegen::apply_script(resource)),
            ("state_query", codegen::state_query_script(resource)),
        ] {
            if let Ok(script) = result {
                lint_generated_script(id, kind, &script, &mut warnings, &mut tally);
            }
        }
    }
    if tally.errors > 0 || tally.warnings > 0 {
        warnings.push(format!(
            "bashrs script lint: {} error(s), {} warning(s) across {} resources",
            tally.errors,
            tally.warnings,
            config.resources.len()
        ));
    }
    warnings
}

/// `forjar lint`. `gate` carries the knobs the CLI leaf parses; the rest of
/// the lint rules are unconditional.
pub(crate) fn cmd_lint_gated(
    file: &Path,
    json: bool,
    strict: bool,
    fix: bool,
    gate: &LintGateArgs,
) -> Result<(), String> {
    cmd_lint_with_writer(
        file,
        json,
        strict,
        fix,
        gate,
        &mut super::output::StdoutWriter,
    )
}

/// Inner lint with injectable OutputWriter (FJ-2920).
pub(crate) fn cmd_lint_with_writer(
    file: &Path,
    json: bool,
    strict: bool,
    fix: bool,
    gate: &LintGateArgs,
    out: &mut dyn super::output::OutputWriter,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // The gate is computed in core and rendered here. `forjar_lint` calls the
    // same function and renders the same strings, so the two surfaces cannot
    // disagree about a config the way they did before FJQ.
    let thresholds = GateThresholds {
        max_cyclomatic: gate.max_cyclomatic,
        policy_dir: gate.policy_dir.clone(),
        complexity_is_error: false,
    };
    let yaml_text = std::fs::read_to_string(file).ok();
    let report = quality_gate::evaluate(&config, yaml_text.as_deref(), &thresholds);

    if gate.sarif {
        let sarif = report.to_sarif(&file.display().to_string());
        out.result(&serde_json::to_string_pretty(&sarif).map_err(|e| format!("JSON error: {e}"))?);
        out.flush();
        return Ok(());
    }

    let mut warnings: Vec<String> = Vec::new();
    warnings.extend(lint_unused_machines(&config));
    warnings.extend(lint_untagged_resources(&config));
    warnings.extend(lint_duplicate_content(&config));
    warnings.extend(lint_dependency_issues(&config));
    warnings.extend(lint_empty_packages(&config));
    if strict {
        warnings.extend(lint_strict_rules(&config));
    }
    warnings.extend(lint_semicolon_chains(&config));
    warnings.extend(lint_nohup_ld_path(&config));
    warnings.extend(lint_nohup_sleep_health(&config));
    // #359 kept alongside FJQ: `lint_scripts` walks the GENERATED shell with
    // bashrs, which is a different question from the quality gate's report —
    // the merge of the two branches dropped this call once, and the only
    // symptom was a dead-code warning.
    warnings.extend(lint_scripts(&config));
    warnings.extend(report.render());

    if json {
        let report = serde_json::json!({
            "warnings": warnings.len(),
            "findings": warnings,
        });
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        out.result(&output);
    } else if warnings.is_empty() {
        out.success("No lint warnings found.");
    } else {
        for w in &warnings {
            out.warning(w);
        }
        if fix {
            let fixes = lint_auto_fix(file)?;
            for f in &fixes.applied {
                out.success(&format!("fixed: {f}"));
            }
            for r in &fixes.refused {
                out.warning(r);
            }
            if !fixes.applied.is_empty() {
                out.status(&format!(
                    "Rewrote {} in place — comments, quote style and every \
                     untouched line are unchanged",
                    file.display()
                ));
            }
        }
        out.result(&format!("\nLint: {} warning(s)", warnings.len()));
    }
    out.flush();

    Ok(())
}
