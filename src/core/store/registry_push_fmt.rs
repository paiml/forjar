//! FJ-2105: Registry-push config validation and CLI summary formatting.
//!
//! Split out of `registry_push.rs` to keep that file under the 500-line health
//! limit; pure (no I/O, no network), so trivially unit-testable.

use super::registry_push::RegistryPushConfig;
use crate::core::types::{PushKind, PushResult};

/// Validate a registry push config.
pub fn validate_push_config(config: &RegistryPushConfig) -> Vec<String> {
    let mut errors = Vec::new();
    if config.registry.is_empty() {
        errors.push("registry hostname is required".into());
    }
    if config.name.is_empty() {
        errors.push("image name is required".into());
    }
    if config.tag.is_empty() {
        errors.push("image tag is required".into());
    }
    if config.registry.contains("://") {
        errors.push("registry should be hostname only, not a URL".into());
    }
    errors
}

/// Format a push summary for CLI output.
pub fn format_push_summary(results: &[PushResult]) -> String {
    let mut out = String::new();
    let uploaded: Vec<_> = results.iter().filter(|r| !r.existed).collect();
    let skipped: Vec<_> = results.iter().filter(|r| r.existed).collect();

    out.push_str(&format!(
        "Push complete: {} uploaded, {} skipped (already exist)\n",
        uploaded.len(),
        skipped.len(),
    ));

    let total_bytes: u64 = uploaded.iter().map(|r| r.size).sum();
    let total_secs: f64 = uploaded.iter().map(|r| r.duration_secs).sum();
    if !uploaded.is_empty() {
        out.push_str(&format!(
            "  Uploaded {:.1} MB in {:.1}s\n",
            total_bytes as f64 / (1024.0 * 1024.0),
            total_secs,
        ));
    }

    for r in results {
        let status = if r.existed { "skip" } else { "push" };
        let kind = match r.kind {
            PushKind::Layer => "layer",
            PushKind::Config => "config",
            PushKind::Manifest => "manifest",
            PushKind::Index => "index",
        };
        out.push_str(&format!(
            "  [{status}] {kind}: {} ({} bytes)\n",
            r.digest, r.size
        ));
    }

    out
}
