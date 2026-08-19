//! FJ-037: `backup_sync` declaration validation.

use super::*;

/// A backup must name a real remote and at least one existing-looking source.
pub(super) fn validate_backup_sync(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    if let Err(e) = crate::resources::backup_sync::backup_of(resource) {
        errors.push(ValidationError {
            message: format!("resource '{id}' (backup_sync): {e}"),
        });
        return;
    }
    // A literal token in the repo is a committed bearer credential. Templates
    // (`{{secrets.x}}`) are the supported form; anything else is refused here
    // rather than discovered in a git history audit later.
    if let Some(t) = resource.backup.token.as_deref() {
        if !t.contains("{{") && t.len() > 24 {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (backup_sync) has a literal `backup_token`. Use \
                     `{{{{secrets.NAME}}}}` so the credential is resolved through the \
                     secrets provider instead of being committed."
                ),
            });
        }
    }
}
