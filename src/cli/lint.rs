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

/// What `lint --fix` actually did, and what it refused to do.
///
/// paiml/forjar#359: the previous shape was a bare `Vec<String>` of "fixes
/// applied", and the one entry it could hold was pushed UNCONDITIONALLY —
/// whenever a `resources:` mapping existed, sorted or not. So `--fix` claimed
/// "sorted resource keys alphabetically" on an already-sorted file, and
/// rewrote the file to prove it. Separating what was applied from what was
/// refused is what makes both halves reportable without one lying about the
/// other.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct AutoFix {
    /// Transformations that changed the file. Empty means the file was not
    /// written.
    pub applied: Vec<String>,
    /// Transformations forjar declined, each carrying why.
    pub refused: Vec<String>,
}

/// The result of trying to sort the `resources:` mapping in place.
enum SortOutcome {
    /// Nothing to do: no `resources:` mapping, or its keys are already sorted.
    Unchanged,
    /// The document with the mapping's entries reordered and nothing else
    /// touched.
    Sorted(String),
    /// The reorder could not be proven sound. Carries the reason.
    Refused(String),
}

/// Sort the entries of `resources:` by moving their source byte ranges.
///
/// paiml/forjar#359: this used to parse the whole document into
/// `serde_yaml_ng::Value`, rebuild the `resources` mapping in sorted order and
/// re-emit the file. `Value` does not carry comments, so every comment in the
/// user's config was deleted — silently, by a flag whose contract was to fix
/// lint findings. In an IaC config the comments are the operational reasoning:
/// why a host is pinned, why an ordering matters, which runbook depends on it.
///
/// Sorting does not need a re-serialisation. Each entry owns a contiguous run
/// of source lines, so this is a permutation of byte ranges: no byte is
/// rewritten, only moved, and a comment above an entry travels with it.
fn sort_resources(content: &str) -> SortOutcome {
    use crate::core::yaml_edit::{blocks, verify, AnchorError};

    let blocks = match blocks::key_blocks(content, &["resources"]) {
        Ok(b) => b,
        // No `resources:` mapping at all is not a refusal — there is nothing
        // to sort, which is exactly the same outcome as "already sorted".
        Err(AnchorError::NotFound) => return SortOutcome::Unchanged,
        Err(e) => return SortOutcome::Refused(e.reason().to_string()),
    };
    if blocks::is_sorted(&blocks) {
        return SortOutcome::Unchanged;
    }
    let sorted = match blocks::reorder(content, &blocks, &blocks::sorted_order(&blocks)) {
        Ok(text) => text,
        Err(e) => return SortOutcome::Refused(e.reason().to_string()),
    };
    // Fail closed. Reordering entries must change no value anywhere in the
    // document; if the re-parse disagrees, the edit is discarded rather than
    // written.
    match verify::changed_paths_of_text(content, &sorted) {
        Ok(changed) if changed.is_empty() => SortOutcome::Sorted(sorted),
        Ok(_) => {
            SortOutcome::Refused("the reorder changed a value, so it was discarded".to_string())
        }
        Err(e) => SortOutcome::Refused(e),
    }
}

/// Apply every auto-fix `lint --fix` knows, writing the file only if one of
/// them actually changed something.
pub(crate) fn lint_auto_fix(file: &Path) -> Result<AutoFix, String> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {}", file.display(), e))?;
    // Fail closed before touching anything: an auto-fixer must never rewrite a
    // document it cannot parse. `cmd_lint` parses the config before it gets
    // here, so this guard is for every other caller.
    serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&content)
        .map_err(|e| format!("YAML parse error: {e}"))?;
    let mut out = AutoFix::default();
    match sort_resources(&content) {
        SortOutcome::Unchanged => {}
        SortOutcome::Sorted(sorted) => {
            std::fs::write(file, &sorted)
                .map_err(|e| format!("cannot write {}: {}", file.display(), e))?;
            out.applied
                .push("sorted resource keys alphabetically".to_string());
        }
        SortOutcome::Refused(reason) => out
            .refused
            .push(format!("resource keys left unsorted: {reason}")),
    }
    Ok(out)
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
