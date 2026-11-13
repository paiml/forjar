//! PMAT-989 (IMG-001 3/N): the base image an `image:` resource declares.
//!
//! forjar has no registry-pull code; the base is an OCI layout the user puts
//! on disk (`skopeo copy docker://<ref> oci:<dir>`, or `docker save <ref> | tar -x`). A declared base that is
//! not usable is an error: building without it would publish an image that is
//! not the one the config describes.

use crate::core::store::base_image::{extract_base_layers, format_base_info, verify_base_blobs};
use crate::core::store::exec_layer::base_fetch_hint;
use crate::core::store::image_assembler::BaseLayout;
use std::path::{Path, PathBuf};

/// The ONE place a base reference becomes a directory name under the image
/// state dir (`ubuntu:22.04` -> `ubuntu_22.04`). Lookup and error text both use it.
pub(super) fn base_layout_dir(state_dir: &Path, base_ref: &str) -> PathBuf {
    state_dir.join(base_ref.replace([':', '/'], "_"))
}

/// A base reference reaches the container runtime as a bare argument
/// (`<rt> image inspect <ref>`, `<rt> run .. <ref>`), names a directory under
/// the image state dir, and is printed inside a shell command hint. So one the
/// runtime could read as a flag, one that names `.` or `..`, or one holding a
/// character outside the image-reference alphabet is refused before any lookup.
fn check_base_ref(resource: &str, base_ref: &str) -> Result<(), String> {
    let in_alphabet =
        |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/' | ':' | '@');
    let why = if base_ref.is_empty() {
        "it is empty"
    } else if base_ref.starts_with('-') {
        "it starts with '-' and a container runtime would read it as an option"
    } else if base_ref
        .chars()
        .any(|c| c.is_whitespace() || c.is_control())
    {
        "it contains whitespace or a control character"
    } else if base_ref == "." || base_ref == ".." {
        "it names a directory, not an image"
    } else if !base_ref.chars().all(in_alphabet) {
        "it contains a character outside [A-Za-z0-9._-/:@]"
    } else {
        return Ok(());
    };
    Err(format!(
        "image '{resource}' declares base image {base_ref:?}, which is not a usable image \
         reference: {why}; refusing to build"
    ))
}

/// Load the declared base, or refuse: absent, unreadable and missing-blob
/// layouts all return an `Err` naming the reference, the directory and the
/// command that would supply it.
pub(super) fn load_declared_base(
    resource: &str,
    state_dir: &Path,
    base_ref: &str,
) -> Result<BaseLayout, String> {
    check_base_ref(resource, base_ref)?;
    let dir = base_layout_dir(state_dir, base_ref);
    let refuse = |why: String| {
        format!(
            "image '{resource}' declares base image '{base_ref}', but {why}; refusing to build \
             without it (the result would not be the declared image). Looked in {}. \
             Fetch it with: {}",
            dir.display(),
            base_fetch_hint(base_ref, &dir)
        )
    };
    if !dir.is_dir() {
        return Err(refuse("its OCI layout directory does not exist".into()));
    }
    let layers = extract_base_layers(&dir)
        .map_err(|e| refuse(format!("its OCI layout cannot be read ({e})")))?;
    let missing = verify_base_blobs(&dir, &layers);
    if !missing.is_empty() {
        return Err(refuse(format!(
            "its OCI layout is missing {} blob(s): {}",
            missing.len(),
            missing.join(", ")
        )));
    }
    Ok(BaseLayout {
        layout_dir: dir,
        layers,
    })
}

/// Feed the base's identity into the build-cache key: every layer digest and
/// diff_id in order, plus what the image config inherits (arch, os, runtime
/// config, history). A scratch build adds nothing, so its key is unchanged.
pub(super) fn hash_base_identity(hasher: &mut blake3::Hasher, base: Option<&BaseLayout>) {
    let Some(base) = base else { return };
    let b = &base.layers;
    hasher.update(b"forjar-base-v1\0");
    for layer in &b.layers {
        hasher.update(layer.digest.as_bytes());
        hasher.update(b"\0");
    }
    for diff_id in &b.diff_ids {
        hasher.update(diff_id.as_bytes());
        hasher.update(b"\0");
    }
    hasher.update(b.architecture.as_bytes());
    hasher.update(b"\0");
    hasher.update(b.os.as_bytes());
    hasher.update(b"\0");
    hasher.update(&serde_json::to_vec(&b.config).unwrap_or_default());
    hasher.update(&serde_json::to_vec(&b.history).unwrap_or_default());
}

/// B7: the (now truthful) "Base: ref (arch/os, N layers, X MB)" line.
pub(super) fn print_base_line(base_ref: Option<&str>, base: Option<&BaseLayout>) {
    if let (Some(r), Some(b)) = (base_ref, base) {
        println!("  {}", format_base_info(r, &b.layers));
    }
}
