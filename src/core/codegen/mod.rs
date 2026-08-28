//! FJ-005: Script generation — dispatch to resource handlers.
//! FJ-036: bashrs purification pipeline integrated (Invariant I8).

mod dispatch;

pub use dispatch::*;

#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod tests_check_verdict;
#[cfg(test)]
mod tests_completeness;
#[cfg(test)]
mod tests_coverage;
#[cfg(test)]
mod tests_dispatch;
#[cfg(test)]
mod tests_sudo;

/// FJ-038: emit the shell a resource generates, resolved as `apply` resolves it.
///
/// A resource whose real payload is synthesised shell cannot be dogfooded — or
/// debugged — unless the artifact can be got at. Lives here rather than in the
/// CLI dispatcher so the dispatcher stays a dispatcher.
///
/// # Errors
///
/// Returns `Err` when the config cannot be parsed, the resource does not exist,
/// or the phase is unknown.
pub fn emit_for_cli(file: &std::path::Path, resource: &str, phase: &str) -> Result<(), String> {
    let cfg = crate::core::parser::parse_and_validate(file)?;
    let resolved = crate::core::resolver::resolve_all(
        &cfg.resources,
        &cfg.params,
        &cfg.machines,
        &cfg.secrets,
    );
    let r = resolved
        .get(resource)
        .ok_or_else(|| format!("no such resource: {resource}"))?;
    let script = match phase {
        "apply" => apply_script(r)?,
        "check" => check_script(r)?,
        "state-query" => state_query_script(r)?,
        // #334: the only phase that is safe to pipe to `sh`. `apply` emits the
        // INSTALLER — it grants the reclaim opt-in and, for `sudo: true`,
        // re-elevates — so the recipe once documented as a preview deleted.
        "reaper" => reaper_phase(r)?,
        other => return Err(format!("unknown phase: {other}")),
    };
    print!("{script}");
    Ok(())
}

/// `--phase reaper`: the disk-budget reclaim pass on its own.
///
/// Scoped to `disk_budget` because it is the only resource whose apply script
/// wraps a separately-invokable, destructive body. Any other type is an error
/// rather than a silent fallback to `apply`, which is the shape that made the
/// documented preview delete in the first place (#334).
fn reaper_phase(r: &crate::core::types::Resource) -> Result<String, String> {
    if r.resource_type != crate::core::types::ResourceType::DiskBudget {
        return Err(format!(
            "phase `reaper` is only defined for disk_budget resources, not {}",
            r.resource_type
        ));
    }
    crate::resources::disk_budget::reaper_script(r)
}
