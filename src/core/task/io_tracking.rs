//! FJ-2701: Input/output tracking for task framework.
//!
//! Provides BLAKE3-based hashing of input file patterns and output artifacts
//! to enable content-addressed caching. When `cache: true`, stages with
//! unchanged inputs can be skipped.

use crate::tripwire::hasher;
use std::path::Path;

/// Hash all files matching input patterns in a directory.
///
/// Patterns are evaluated as simple globs relative to `base_dir`:
/// - Exact paths: `src/main.rs`
/// - Single wildcard: `src/*.rs`
/// - Recursive wildcard: `src/**/*.rs`
///
/// Returns a composite BLAKE3 hash of all matched files (sorted for determinism).
///
/// # Examples
///
/// ```
/// use forjar::core::task::hash_inputs;
///
/// // Returns None for empty patterns
/// assert!(hash_inputs(&[], std::path::Path::new("/tmp")).unwrap().is_none());
/// ```
pub fn hash_inputs(patterns: &[String], base_dir: &Path) -> Result<Option<String>, String> {
    if patterns.is_empty() {
        return Ok(None);
    }

    let mut matched_files: Vec<String> = Vec::new();

    for pattern in patterns {
        let files = expand_glob(pattern, base_dir)?;
        matched_files.extend(files);
    }

    if matched_files.is_empty() {
        return Ok(None);
    }

    // Sort for deterministic hash
    matched_files.sort();
    matched_files.dedup();

    let mut components: Vec<String> = Vec::new();
    for file_path in &matched_files {
        let path = Path::new(file_path);
        let hash = hasher::hash_file(path)?;
        // Include relative path in hash for rename detection
        components.push(format!("{file_path}\0{hash}"));
    }

    let refs: Vec<&str> = components.iter().map(|s| s.as_str()).collect();
    Ok(Some(hasher::composite_hash(&refs)))
}

/// Hash output artifacts for cache validation.
///
/// # Examples
///
/// ```
/// use forjar::core::task::hash_outputs;
///
/// // Returns None for empty artifacts
/// assert!(hash_outputs(&[]).unwrap().is_none());
/// ```
pub fn hash_outputs(artifacts: &[String]) -> Result<Option<String>, String> {
    if artifacts.is_empty() {
        return Ok(None);
    }

    let mut components: Vec<String> = Vec::new();
    for artifact in artifacts {
        let path = Path::new(artifact);
        if path.exists() {
            let hash = if path.is_dir() {
                hasher::hash_directory(path)?
            } else {
                hasher::hash_file(path)?
            };
            components.push(format!("{artifact}\0{hash}"));
        }
        // Missing artifacts are not an error — they may not exist yet
    }

    if components.is_empty() {
        return Ok(None);
    }

    let refs: Vec<&str> = components.iter().map(|s| s.as_str()).collect();
    Ok(Some(hasher::composite_hash(&refs)))
}

/// Determine if a stage can be skipped based on cached input hash.
///
/// A stage is skippable when all conditions are met:
/// 1. `cache` is true
/// 2. The stage has input patterns
/// 3. The current input hash matches the stored hash
///
/// # Examples
///
/// ```
/// use forjar::core::task::should_skip_cached;
///
/// // No cache → never skip
/// assert!(!should_skip_cached(false, None, None));
///
/// // Cache enabled, hashes match → skip
/// let h = Some("blake3:abc".to_string());
/// assert!(should_skip_cached(true, h.as_deref(), h.as_deref()));
///
/// // Cache enabled, hashes differ → don't skip
/// assert!(!should_skip_cached(true, Some("blake3:abc"), Some("blake3:def")));
/// ```
pub fn should_skip_cached(
    cache: bool,
    current_hash: Option<&str>,
    stored_hash: Option<&str>,
) -> bool {
    if !cache {
        return false;
    }
    match (current_hash, stored_hash) {
        (Some(cur), Some(stored)) => cur == stored,
        _ => false,
    }
}

/// Expand a glob pattern to matching file paths.
fn expand_glob(pattern: &str, base_dir: &Path) -> Result<Vec<String>, String> {
    let full_pattern = if Path::new(pattern).is_absolute() {
        pattern.to_string()
    } else {
        format!("{}/{pattern}", base_dir.display())
    };

    let paths =
        glob::glob(&full_pattern).map_err(|e| format!("invalid glob pattern '{pattern}': {e}"))?;

    let mut result = Vec::new();
    for entry in paths {
        match entry {
            Ok(path) if path.is_file() => {
                result.push(path.to_string_lossy().to_string());
            }
            Ok(_) => {} // skip directories
            Err(e) => {
                return Err(format!("glob error for '{pattern}': {e}"));
            }
        }
    }
    Ok(result)
}
