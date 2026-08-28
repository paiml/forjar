//! Plan printing and diff display helpers.

use super::helpers::*;
use crate::core::{codegen, resolver, types};
use std::path::Path;

/// Format the action symbol for a plan change.
fn action_symbol(action: &types::PlanAction) -> String {
    match action {
        types::PlanAction::Create => green("+"),
        types::PlanAction::Update => yellow("~"),
        types::PlanAction::Destroy => red("-"),
        types::PlanAction::NoOp => dim(" "),
    }
}

/// Format the description text for a plan change with color.
fn action_desc(action: &types::PlanAction, description: &str) -> String {
    match action {
        types::PlanAction::Create => green(description),
        types::PlanAction::Update => yellow(description),
        types::PlanAction::Destroy => red(description),
        types::PlanAction::NoOp => dim(description),
    }
}

/// Print a single plan change entry, including optional content diff.
fn print_plan_change(change: &types::PlannedChange, config: Option<&types::ForjarConfig>) {
    println!(
        "  {} {}",
        action_symbol(&change.action),
        action_desc(&change.action, &change.description)
    );

    // FJ-255/274: Show content diff for file resources on create/update
    if let Some(cfg) = config {
        if matches!(
            change.action,
            types::PlanAction::Create | types::PlanAction::Update
        ) {
            if let Some(resource) = cfg.resources.get(&change.resource_id) {
                if let Some(ref content) = resource.content {
                    let old_content = if matches!(change.action, types::PlanAction::Update) {
                        resource
                            .path
                            .as_ref()
                            .and_then(|p| std::fs::read_to_string(p).ok())
                    } else {
                        None
                    };
                    print_content_diff(content, &change.action, old_content.as_deref());
                }
            }
        }
    }
}

/// Format a count with color: non-zero values get colored, zero stays plain.
fn colored_count(count: u32, color_fn: fn(&str) -> String) -> String {
    if count > 0 {
        color_fn(&count.to_string())
    } else {
        count.to_string()
    }
}

/// How many locked resources carry observed state that `plan` did not consult.
///
/// forjar#342. `plan` compares the config to the LOCK; `drift` compares the
/// lock to the HOST. Both are correct for the question they answer, and they
/// disagree about the same machine — on intel, `plan` reported 52 changes while
/// `drift` found 28 drifted resources, and neither number contains the other.
///
/// This counts what plan is BLIND TO, not what is wrong. A resource is counted
/// when its lock entry holds an observation from the target, because that is
/// exactly the state a lock-relative diff cannot range over. It says nothing
/// about whether that resource has drifted — establishing that requires
/// reaching the machine, which is `drift`'s job and deliberately not plan's.
pub(crate) fn unconsulted_observations(
    locks: &std::collections::HashMap<String, types::StateLock>,
) -> usize {
    locks
        .values()
        .flat_map(|l| l.resources.values())
        .filter(|rl| rl.observed_state().is_some())
        .count()
}

/// State the quantifier this report ranges over.
///
/// SILENCE IS THE BUG. `plan` printing `0 to change` reads as "nothing is
/// wrong", when it means "nothing in the lock disagrees with the config" — and
/// the mutated-file case proves those are different claims. Naming the blind
/// spot converts an absence into a statement.
///
/// Produced for a CLEAN plan too, on purpose: the dangerous case is precisely
/// the one where plan has nothing to report. A disclosure that only appeared
/// alongside pending changes would be missing exactly when it is needed.
///
/// forjar#342: this is a VALUE, not a `println!`. It was written as a
/// side-effecting printer, so the only way to consume it was to be inside
/// `print_plan` — which is why the disclosure reached the TTY rendering and
/// structurally could not reach `plan --json` or the MCP/HTTP `plan` verb.
/// Returning `None` at zero is the contract's biconditional, not an
/// optimisation: an unconditional banner is noise, and noise is how a warning
/// stops being read.
pub(crate) fn scope_disclosure(unconsulted: usize) -> Option<String> {
    if unconsulted == 0 {
        return None;
    }
    Some(format!(
        "This plan is lock-relative: it compares the config to the lock, and \
         did not\ncontact any machine. {unconsulted} locked resource(s) carry \
         state observed on a\ntarget that this plan did not consult — run \
         `forjar drift` for what the machines\nactually hold."
    ))
}

/// Print the disclosure, when there is one.
fn print_scope_disclosure(unconsulted: usize) {
    if let Some(msg) = scope_disclosure(unconsulted) {
        println!("\n{msg}");
    }
}

/// Print the plan summary line.
fn print_plan_summary(plan: &types::ExecutionPlan) {
    println!(
        "Plan: {} to add, {} to change, {} to destroy, {} unchanged.",
        colored_count(plan.to_create, green),
        colored_count(plan.to_update, yellow),
        colored_count(plan.to_destroy, red),
        plan.unchanged
    );
}

/// Display a plan to stdout.
/// If `config` is Some, show content diff for file resources (FJ-255).
pub(crate) fn print_plan(
    plan: &types::ExecutionPlan,
    machine_filter: Option<&str>,
    config: Option<&types::ForjarConfig>,
    unconsulted: usize,
) {
    println!("Planning: {} ({} resources)", plan.name, plan.changes.len());
    println!();

    let mut current_machine = String::new();
    for change in &plan.changes {
        if let Some(filter) = machine_filter {
            if change.machine != filter {
                continue;
            }
        }
        if change.machine != current_machine {
            current_machine.clone_from(&change.machine);
            println!("{current_machine}:");
        }
        print_plan_change(change, config);
    }

    println!();
    print_plan_summary(plan);
    print_scope_disclosure(unconsulted);
}

/// FJ-255/274: Print a content diff block for a file resource.
/// For Creates: shows all new lines with `+` prefix.
/// For Updates with known old content (FJ-274): shows unified diff.
/// Limited to 50 lines; truncated with "[... N more lines]".
pub(crate) fn print_content_diff(
    content: &str,
    action: &types::PlanAction,
    old_content: Option<&str>,
) {
    // FJ-274: For updates with old content, show unified diff
    if matches!(action, types::PlanAction::Update) {
        if let Some(old) = old_content {
            print_unified_diff(old, content);
            return;
        }
    }

    let lines: Vec<&str> = content.lines().collect();
    let prefix = match action {
        types::PlanAction::Create => "+",
        types::PlanAction::Update => "~",
        _ => " ",
    };
    let max_lines = 50;
    let show = lines.len().min(max_lines);
    println!("    ---");
    for line in &lines[..show] {
        println!("    {prefix} {line}");
    }
    if lines.len() > max_lines {
        println!("    [... {} more lines]", lines.len() - max_lines);
    }
    println!("    ---");
}

/// FJ-274: Print a simple unified diff between old and new content.
pub(crate) fn print_unified_diff(old: &str, new: &str) {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let max_lines = 50;
    let mut shown = 0;

    println!("    ---");
    // Simple line-by-line comparison
    let max_len = old_lines.len().max(new_lines.len());
    for i in 0..max_len {
        if shown >= max_lines {
            println!("    [... {} more lines]", max_len - shown);
            break;
        }
        match (old_lines.get(i), new_lines.get(i)) {
            (Some(o), Some(n)) if o == n => {
                println!("      {o}");
                shown += 1;
            }
            (Some(o), Some(n)) => {
                println!("    {} {}", red("-"), o);
                println!("    {} {}", green("+"), n);
                shown += 2;
            }
            (Some(o), None) => {
                println!("    {} {}", red("-"), o);
                shown += 1;
            }
            (None, Some(n)) => {
                println!("    {} {}", green("+"), n);
                shown += 1;
            }
            (None, None) => break,
        }
    }
    println!("    ---");
}

/// Export generated scripts (check, apply, state_query) to a directory for auditing.
/// Templates (params, secrets, machine refs) are resolved before export.
pub(crate) fn export_scripts(config: &types::ForjarConfig, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("cannot create output dir {}: {}", dir.display(), e))?;

    let mut count = 0;
    for (id, resource) in &config.resources {
        // Resolve templates (params, secrets, machine refs) before codegen
        let resolved =
            resolver::resolve_resource_templates(resource, &config.params, &config.machines)?;

        // Sanitize resource ID for filesystem (replace / with --)
        let safe_id = id.replace('/', "--");

        // FJ-297: Build metadata header for exported scripts
        let machine_str = match &resource.machine {
            types::MachineTarget::Single(m) => m.clone(),
            types::MachineTarget::Multiple(ms) => ms.join(","),
        };
        let mut header = format!(
            "# forjar: {} ({})\n# machine: {}\n# type: {}\n",
            id, config.name, machine_str, resource.resource_type
        );
        if let Some(ref rg) = resource.resource_group {
            header.push_str(&format!("# group: {rg}\n"));
        }
        if !resource.tags.is_empty() {
            header.push_str(&format!("# tags: {}\n", resource.tags.join(", ")));
        }
        if !resource.depends_on.is_empty() {
            header.push_str(&format!(
                "# depends_on: {}\n",
                resource.depends_on.join(", ")
            ));
        }

        if let Ok(script) = codegen::check_script(&resolved) {
            let path = dir.join(format!("{safe_id}.check.sh"));
            std::fs::write(&path, format!("{header}{script}"))
                .map_err(|e| format!("write {}: {}", path.display(), e))?;
            count += 1;
        }

        if let Ok(script) = codegen::apply_script(&resolved) {
            let path = dir.join(format!("{safe_id}.apply.sh"));
            std::fs::write(&path, format!("{header}{script}"))
                .map_err(|e| format!("write {}: {}", path.display(), e))?;
            count += 1;
        }

        if let Ok(script) = codegen::state_query_script(&resolved) {
            let path = dir.join(format!("{safe_id}.state_query.sh"));
            std::fs::write(&path, format!("{header}{script}"))
                .map_err(|e| format!("write {}: {}", path.display(), e))?;
            count += 1;
        }
    }

    println!("Exported {} scripts to {}", count, dir.display());
    Ok(())
}
