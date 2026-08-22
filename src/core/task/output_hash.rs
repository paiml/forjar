//! GH-246: hashing an output under a declared equivalence predicate.
//!
//! Split from `io_tracking` so that module keeps its size and its TDG grade —
//! and because "how an artifact is compared" is a different concern from "which
//! files are tracked".

use super::super::types::OutputEquivalence;
use crate::tripwire::hasher;
use indexmap::IndexMap;
use std::path::Path;

/// GH-246: hash outputs under a per-artifact equivalence predicate.
///
/// Byte-identity is right for anything reproducible and wrong for producers
/// that cannot reach it. An artifact declared `none` or `external` drops out of
/// the CONTENT hash entirely — it is still tracked for existence, because a
/// missing artifact is staleness under any predicate. `command` substitutes a
/// declared normaliser's stdout for the file bytes, which covers structural
/// equivalence without teaching forjar any media formats.
pub fn hash_outputs_with(
    artifacts: &[String],
    base_dir: &Path,
    equivalence: &IndexMap<String, OutputEquivalence>,
) -> Result<Option<String>, String> {
    if artifacts.is_empty() {
        return Ok(None);
    }

    let mut components: Vec<String> = Vec::new();
    for artifact in artifacts {
        let rule = equivalence.get(artifact).cloned().unwrap_or_default();
        if let Some(c) = artifact_component(artifact, &rule, base_dir)? {
            components.push(c);
        }
    }

    if components.is_empty() {
        return Ok(None);
    }

    let refs: Vec<&str> = components.iter().map(|s| s.as_str()).collect();
    Ok(Some(hasher::composite_hash(&refs)))
}

/// The hash component for one artifact, or `None` if it does not exist yet.
///
/// Extracted from `hash_outputs_with` to keep that function's complexity within
/// the repo's TDG budget — the loop body carried three independent decisions
/// (resolve, exists, predicate) and the file dropped A+ -> A with them inline.
fn artifact_component(
    artifact: &str,
    rule: &OutputEquivalence,
    base_dir: &Path,
) -> Result<Option<String>, String> {
    let joined;
    let path = {
        let p = Path::new(artifact);
        if p.is_absolute() {
            p
        } else {
            joined = base_dir.join(p);
            joined.as_path()
        }
    };
    // Missing artifacts are not an error — they may not exist yet.
    if !path.exists() {
        return Ok(None);
    }
    if !rule.contributes_content() {
        // Tracked, but deliberately not keyed on content. Recorded so the hash
        // still changes if the DECLARATION changes: flipping an artifact from
        // `external` back to `bytes` must not silently look identical to never
        // having declared it.
        return Ok(Some(format!("{artifact}\0<{}>", rule.as_str())));
    }
    let hash = match rule {
        OutputEquivalence::Command(script) => normalised_hash(script, path, base_dir)?,
        _ if path.is_dir() => hasher::hash_directory(path)?,
        _ => hasher::hash_file(path)?,
    };
    Ok(Some(format!("{artifact}\0{hash}")))
}

/// Run a declared normaliser over `path` and hash its stdout.
///
/// The artifact path is passed as `$1` and also as `FORJAR_ARTIFACT`. A
/// normaliser that fails is an error rather than a fallback to bytes: silently
/// reverting to the predicate the author explicitly replaced would reintroduce
/// exactly the spurious staleness they declared it to avoid.
fn normalised_hash(script: &str, path: &Path, base_dir: &Path) -> Result<String, String> {
    let out = std::process::Command::new("bash")
        .arg("-c")
        .arg(script)
        .arg("bash")
        .arg(path)
        .env("FORJAR_ARTIFACT", path)
        .current_dir(base_dir)
        .output()
        .map_err(|e| format!("output_equivalence command for {}: {e}", path.display()))?;
    if !out.status.success() {
        return Err(format!(
            "output_equivalence command for {} failed ({}): {}",
            path.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(hasher::hash_string(&String::from_utf8_lossy(&out.stdout)))
}
