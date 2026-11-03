//! Data-source fault isolation for `plan` and `apply`.
//!
//! One failing `data:` source must not stop unrelated resources. The failure
//! is recorded and printed; only resources whose templates reference the
//! source (and what `depends_on` them) are dropped from the run.

use crate::core::{resolver, types};

/// Load the manifest, running data sources without aborting (forjar#660 load path, PMAT-828
/// isolation). `prepare` runs before any source. Returns the config and the fault messages
/// (empty when everything resolved); see [`drop_dependents`] for what happens to a fault.
pub(crate) fn load_isolated(
    file: &std::path::Path,
    prepare: impl FnOnce(&mut types::ForjarConfig) -> Result<(), String>,
) -> Result<(types::ForjarConfig, Vec<String>), String> {
    resolver::load_resolved_config_isolated(file, prepare, drop_dependents)
}

/// Quiet load for a preflight reader (`check_operator_auth`, `check_cost_limit`) that runs
/// BEFORE `plan`/`apply` load the manifest and report the faults themselves. A failed data
/// source must not abort the run from here (PMAT-828), so the resources it feeds are
/// dropped silently and the faults are left for the real load to print once.
pub(crate) fn load_tolerant(file: &std::path::Path) -> Result<types::ForjarConfig, String> {
    resolver::load_resolved_config_isolated(
        file,
        |_| Ok(()),
        |config, failures| {
            for id in resolver::dependents_of_failures(config, failures).keys() {
                config.resources.shift_remove(id);
            }
            Vec::new()
        },
    )
    .map(|(config, _)| config)
}

/// Drops the resources a failed source feeds from `config`, prints each failure and each
/// skipped resource to stderr, and returns the messages.
fn drop_dependents(
    config: &mut types::ForjarConfig,
    failures: &[resolver::DataSourceFailure],
) -> Vec<String> {
    let skipped = resolver::dependents_of_failures(config, failures);
    let mut messages: Vec<String> = failures.iter().map(|f| f.message.clone()).collect();
    for (id, why) in &skipped {
        eprintln!("FAILED  {id}: {why} (resource skipped)");
        config.resources.shift_remove(id);
    }
    for f in failures {
        eprintln!("data source '{}' failed: {}", f.key, f.message);
    }
    messages.extend(skipped.iter().map(|(id, why)| format!("{id}: {why}")));
    messages
}

/// Non-zero exit: any failed data source makes the run fail overall, after
/// the unrelated resources have been planned/applied.
pub(crate) fn fail_if_any(messages: &[String]) -> Result<(), String> {
    if messages.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "data source failure isolated; unrelated resources were processed: {}",
            messages.join("; ")
        ))
    }
}
