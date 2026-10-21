//! IMG-001 (forjar#756): resolve a declared base image to an OCI layout on disk
//! so its layers can sit under the built ones, and prove the base is the one
//! the spec declared.

use super::base_image::extract_base_layers;
use super::image_assembler::BaseLayout;
use std::path::Path;
use std::process::Command;

/// Directory name under `state/images/` for a base reference (`:` and `/` -> `_`).
pub fn local_base_dir_name(base: &str) -> String {
    base.replace([':', '/'], "_")
}

/// A pinned base (`name@sha256:...`) must resolve to exactly that digest.
///
/// `seen` is what the runtime reports for the pulled image (`RepoDigests`).
/// A tag-only base makes no digest claim and passes. A pinned base for which
/// nothing could be measured FAILS: an unverifiable pin is not a verified one.
pub fn verify_declared_digest(base_ref: &str, seen: &[String]) -> Result<(), String> {
    let Some((_, declared)) = base_ref.split_once("@sha256:") else {
        return Ok(());
    };
    let declared = format!("sha256:{declared}");
    if seen.is_empty() {
        return Err(format!(
            "base {base_ref} could not be verified: the runtime reported no digest for it"
        ));
    }
    if seen.iter().any(|s| s.ends_with(&format!("@{declared}"))) {
        return Ok(());
    }
    Err(format!(
        "base digest mismatch: spec declares {declared} but the runtime resolved {}",
        seen.join(", ")
    ))
}

/// Layout already present under `state/images/<base>` (e.g. from `skopeo copy`).
///
/// `Ok(None)` only when no layout exists. A layout that exists but cannot be
/// read, or that is not the digest a pinned base declares (or whose digest
/// cannot be measured), is an error: it is never silently skipped or layered.
pub fn load_local_base(state_images: &Path, base: &str) -> Result<Option<BaseLayout>, String> {
    let dir = state_images.join(local_base_dir_name(base));
    let index_path = dir.join("index.json");
    if !index_path.exists() {
        return Ok(None);
    }
    let layers = extract_base_layers(&dir)
        .map_err(|e| format!("local base layout {}: {e}", dir.display()))?;
    let raw = std::fs::read(&index_path).map_err(|e| format!("read index.json: {e}"))?;
    let idx: serde_json::Value =
        serde_json::from_slice(&raw).map_err(|e| format!("parse index.json: {e}"))?;
    let repo = base.split('@').next().unwrap_or(base);
    let seen: Vec<String> = idx["manifests"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["digest"].as_str())
        .map(|d| format!("{repo}@{d}"))
        .collect();
    verify_declared_digest(base, &seen)?;
    Ok(Some(BaseLayout {
        layout_dir: dir,
        layers,
    }))
}

/// The base for a files-only build: a local layout if present (verified), else
/// exported from the container runtime (verified). With neither, FAIL: a declared
/// base that cannot be produced must not become a scratch image.
pub fn resolve_build_base(
    base: &str,
    state_images: &Path,
    work_dir: &Path,
    runtime: Option<&str>,
) -> Result<BaseLayout, String> {
    if let Some(b) = load_local_base(state_images, base)? {
        return Ok(b);
    }
    let rt = runtime.ok_or_else(|| {
        format!(
            "cannot produce base {base}: no layout under {} and no container runtime to export it",
            state_images.display()
        )
    })?;
    load_runtime_base(rt, base, work_dir)
}

/// Export `base_ref` from the container runtime into `work_dir` as an OCI layout
/// (`<runtime> save`), after checking it is the declared digest.
pub fn load_runtime_base(
    runtime: &str,
    base_ref: &str,
    work_dir: &Path,
) -> Result<BaseLayout, String> {
    let inspect = Command::new(runtime)
        .args([
            "image",
            "inspect",
            "--format",
            "{{json .RepoDigests}}",
            base_ref,
        ])
        .output()
        .map_err(|e| format!("{runtime} image inspect: {e}"))?;
    if !inspect.status.success() {
        return Err(format!(
            "{runtime} image inspect {base_ref}: {}",
            String::from_utf8_lossy(&inspect.stderr).trim()
        ));
    }
    let seen: Vec<String> = serde_json::from_slice(&inspect.stdout).unwrap_or_default();
    verify_declared_digest(base_ref, &seen)?;

    std::fs::create_dir_all(work_dir).map_err(|e| format!("create base dir: {e}"))?;
    let tar_path = work_dir.join("base.tar");
    let save = Command::new(runtime)
        .args(["save", "-o"])
        .arg(&tar_path)
        .arg(base_ref)
        .output()
        .map_err(|e| format!("{runtime} save: {e}"))?;
    if !save.status.success() {
        return Err(format!(
            "{runtime} save {base_ref}: {}",
            String::from_utf8_lossy(&save.stderr).trim()
        ));
    }
    let layout = work_dir.join("layout");
    std::fs::create_dir_all(&layout).map_err(|e| format!("create layout dir: {e}"))?;
    let untar = Command::new("tar")
        .arg("-xf")
        .arg(&tar_path)
        .arg("-C")
        .arg(&layout)
        .output()
        .map_err(|e| format!("tar: {e}"))?;
    if !untar.status.success() {
        return Err(format!(
            "untar base: {}",
            String::from_utf8_lossy(&untar.stderr).trim()
        ));
    }
    let layers = extract_base_layers(&layout).map_err(|e| {
        format!("base {base_ref}: `{runtime} save` did not produce an OCI layout: {e}")
    })?;
    Ok(BaseLayout {
        layout_dir: layout,
        layers,
    })
}
