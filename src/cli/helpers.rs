//! Shared CLI helpers: color re-exports, parsing, state utilities.

use crate::core::{parser, types};
use std::path::Path;

// Re-export color system from colors.rs for backward compatibility.
// All callers that `use super::helpers::*` continue to work unchanged.
#[allow(unused_imports)]
pub(crate) use super::colors::{bold, color_enabled, dim, green, red, yellow, NO_COLOR};

/// Parse, validate, and expand recipes in a forjar config file.
pub(crate) fn parse_and_validate(file: &Path) -> Result<types::ForjarConfig, String> {
    parser::parse_and_validate(file)
}

/// Discover machine names from a state directory by listing subdirectories that contain state.lock.yaml.
pub(crate) fn discover_machines(state_dir: &Path) -> Vec<String> {
    let mut machines = Vec::new();
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if entry.path().join("state.lock.yaml").exists() {
                    machines.push(name);
                }
            }
        }
    }
    machines.sort();
    machines
}
