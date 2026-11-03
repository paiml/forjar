//! Data-source fault isolation for `plan` and `apply`.
//!
//! One failing `data:` source must not stop unrelated resources. The failure
//! is recorded and printed; only resources whose templates reference the
//! source (and what `depends_on` them) are dropped from the run.

use crate::core::{resolver, types};

/// Resolve data sources without aborting. Drops affected resources from
/// `config`, prints each failure and each skipped resource to stderr, and
/// returns the messages (empty when everything resolved).
pub(crate) fn resolve_isolated(config: &mut types::ForjarConfig) -> Vec<String> {
    let failures = resolver::resolve_data_sources_isolated(config);
    if failures.is_empty() {
        return Vec::new();
    }
    let skipped = resolver::dependents_of_failures(config, &failures);
    let mut messages: Vec<String> = failures.iter().map(|f| f.message.clone()).collect();
    for (id, why) in &skipped {
        eprintln!("FAILED  {id}: {why} (resource skipped)");
        config.resources.shift_remove(id);
    }
    for f in &failures {
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
