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
        other => return Err(format!("unknown phase: {other}")),
    };
    print!("{script}");
    Ok(())
}
