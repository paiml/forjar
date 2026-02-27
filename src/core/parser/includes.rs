//! FJ-254: Merge included config files into the base config.

use super::*;
use std::path::Path;

/// FJ-254: Merge included config files into the base config.
/// Later includes override earlier ones. params/machines/resources merge by key.
/// policy is replaced wholesale. includes are not recursive (single level).
pub(super) fn merge_includes(base: ForjarConfig, base_dir: &Path) -> Result<ForjarConfig, String> {
    let mut merged = base.clone();
    merged.includes = vec![]; // Clear includes from merged result

    for include_path in &base.includes {
        let full_path = base_dir.join(include_path);
        let included = super::parse_config_file(&full_path)
            .map_err(|e| format!("include '{}': {}", include_path, e))?;

        // Merge params (later overrides earlier)
        for (k, v) in included.params {
            merged.params.insert(k, v);
        }

        // Merge machines (later overrides earlier)
        for (k, v) in included.machines {
            merged.machines.insert(k, v);
        }

        // Merge resources (later overrides earlier)
        for (k, v) in included.resources {
            merged.resources.insert(k, v);
        }

        // Policy: replace wholesale from include
        merged.policy = included.policy;

        // Merge outputs
        for (k, v) in included.outputs {
            merged.outputs.insert(k, v);
        }

        // Merge policy rules
        merged.policies.extend(included.policies);

        // Merge data sources
        for (k, v) in included.data {
            merged.data.insert(k, v);
        }
    }

    Ok(merged)
}
