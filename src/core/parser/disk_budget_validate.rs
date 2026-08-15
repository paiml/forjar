//! FJ-036: `disk_budget` declaration validation.
//!
//! Split out of `resource_types.rs` to keep that file under the 500-line
//! module gate.

use super::*;

/// FJ-036: a budget must name a filesystem, have coherent watermarks, and be
/// able to actually reclaim something.
pub(super) fn validate_disk_budget(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    if let Err(e) = crate::resources::disk_budget::budget_of(resource) {
        errors.push(ValidationError {
            message: format!("resource '{id}' (disk_budget): {e}"),
        });
        return;
    }
    // A budget with no reclaim rules can observe pressure but never relieve it:
    // every triggered pass would fail its target and the unit would sit failed
    // forever. Refuse it at parse time rather than shipping a permanent alarm.
    if resource.budget_reclaim.is_empty() {
        errors.push(ValidationError {
            message: format!(
                "resource '{id}' (disk_budget) declares no `budget_reclaim` rules — it could \
                 detect pressure but never relieve it"
            ),
        });
    }
    for (i, rule) in resource.budget_reclaim.iter().enumerate() {
        if rule.name.is_empty() {
            errors.push(ValidationError {
                message: format!("resource '{id}' (disk_budget) reclaim rule #{i} has no name"),
            });
        }
        if rule.roots.is_empty() {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (disk_budget) reclaim rule '{}' has no roots",
                    rule.name
                ),
            });
        }
        for root in &rule.roots {
            // Validation runs on the RAW config, before template resolution, so
            // a templated root (`{{params.home}}/src`) is not yet absolute and
            // must be skipped here — same convention as `validate_cron`'s
            // schedule check. The resolver expands `budget_reclaim[].roots`, and
            // an unexpanded template would simply match nothing at reap time.
            if root.contains("{{") {
                continue;
            }
            if !root.starts_with('/') {
                errors.push(ValidationError {
                    message: format!(
                        "resource '{id}' (disk_budget) reclaim rule '{}' root '{root}' is not \
                         absolute",
                        rule.name
                    ),
                });
            }
        }
    }
}
