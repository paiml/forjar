//! Linting.

use super::helpers::*;
use crate::core::{codegen, types};
use std::path::Path;

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

fn lint_strict_rules(config: &types::ForjarConfig) -> Vec<String> {
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

/// FJ-3000: Detect semicolon-chained commands in task resources.
///
/// Semicolons mask exit codes — `cmd1 ; cmd2` runs cmd2 even if cmd1 fails.
/// Under `set -euo pipefail`, only the last command's exit code matters.
/// Warns users to use `&&` or multiline `|` instead.
pub(crate) fn lint_semicolon_chains(config: &types::ForjarConfig) -> Vec<String> {
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
pub(crate) fn has_bare_semicolon(cmd: &str) -> bool {
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
pub(crate) fn lint_nohup_ld_path(config: &types::ForjarConfig) -> Vec<String> {
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
pub(crate) fn lint_nohup_sleep_health(config: &types::ForjarConfig) -> Vec<String> {
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

fn lint_scripts(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut script_errors = 0usize;
    let mut script_warnings_count = 0usize;
    for (id, resource) in &config.resources {
        for (kind, result) in [
            ("check", codegen::check_script(resource)),
            ("apply", codegen::apply_script(resource)),
            ("state_query", codegen::state_query_script(resource)),
        ] {
            if let Ok(script) = result {
                let lint_result = crate::core::purifier::lint_script(&script);
                for d in &lint_result.diagnostics {
                    use bashrs::linter::Severity;
                    match d.severity {
                        Severity::Error => {
                            script_errors += 1;
                            warnings.push(format!(
                                "bashrs: {}/{} [{}] {}",
                                id, kind, d.code, d.message
                            ));
                        }
                        _ => {
                            script_warnings_count += 1;
                        }
                    }
                }
            }
        }
    }
    if script_errors > 0 || script_warnings_count > 0 {
        warnings.push(format!(
            "bashrs script lint: {} error(s), {} warning(s) across {} resources",
            script_errors,
            script_warnings_count,
            config.resources.len()
        ));
    }
    warnings
}

pub(crate) fn lint_auto_fix(file: &Path) -> Result<Vec<String>, String> {
    let mut fixes_applied = Vec::new();
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {}", file.display(), e))?;
    let mut doc: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("YAML parse error: {e}"))?;
    if let Some(serde_yaml_ng::Value::Mapping(map)) = doc.get_mut("resources") {
        let mut pairs: Vec<_> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        pairs.sort_by(|(a, _), (b, _)| a.as_str().unwrap_or("").cmp(b.as_str().unwrap_or("")));
        *map = serde_yaml_ng::Mapping::new();
        for (k, v) in pairs {
            map.insert(k, v);
        }
        fixes_applied.push("sorted resource keys alphabetically".to_string());
    }
    if !fixes_applied.is_empty() {
        let normalized =
            serde_yaml_ng::to_string(&doc).map_err(|e| format!("serialization error: {e}"))?;
        std::fs::write(file, normalized)
            .map_err(|e| format!("cannot write {}: {}", file.display(), e))?;
    }
    Ok(fixes_applied)
}

pub(crate) fn cmd_lint(file: &Path, json: bool, strict: bool, fix: bool) -> Result<(), String> {
    cmd_lint_with_writer(file, json, strict, fix, &mut super::output::StdoutWriter)
}

/// Inner lint with injectable OutputWriter (FJ-2920).
pub(crate) fn cmd_lint_with_writer(
    file: &Path,
    json: bool,
    strict: bool,
    fix: bool,
    out: &mut dyn super::output::OutputWriter,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

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
    warnings.extend(lint_scripts(&config));

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
            for f in &fixes {
                out.success(&format!("fixed: {f}"));
            }
            if !fixes.is_empty() {
                out.status(&format!("Wrote normalized config to {}", file.display()));
            }
        }
        out.result(&format!("\nLint: {} warning(s)", warnings.len()));
    }
    out.flush();

    Ok(())
}
