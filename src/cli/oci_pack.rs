//! FJ-2101: `forjar oci-pack` — pack a directory into a real OCI image layout.
//!
//! # Why this file exists (Refs #210, #213)
//!
//! `oci-pack` used to be a stub. It printed
//!
//! ```text
//! OCI layout generation requires sha2+flate2 crates.
//! ```
//!
//! and exited 0, having created nothing: no `--output` directory, no
//! `oci-layout`, no `index.json`, no blobs. With `--json` it did worse — it
//! serialised a synthesised OCI manifest (schemaVersion 2, `layers[]`,
//! annotations) with no caveat at all, so a machine consumer had *no* signal
//! that the layout it described did not exist.
//!
//! The stated reason was also false: `forjar build` writes a real layout with
//! sha256 blobs from the very same binary, which proves `sha2` and `flate2`
//! are compiled in. So this is not implemented by faking the manifest and it
//! is not refused either — it is implemented on top of the writer `build`
//! already uses (`image_assembler::assemble_image`), and the command now
//! reports only digests it actually wrote.

use crate::core::store::image_assembler::assemble_image;
use crate::core::store::overlay_export::scan_overlay_upper;
use crate::core::types::{ImageBuildPlan, LayerStrategy, OciLayerConfig};
use std::path::Path;

/// FJ-2101: Pack a directory into an OCI image layout.
pub(crate) fn cmd_oci_pack(dir: &Path, tag: &str, output: &Path, json: bool) -> Result<(), String> {
    check_pack_dir(dir)?;
    if tag.trim().is_empty() {
        return Err("--tag must not be empty (expected name:tag)".to_string());
    }

    let scan = scan_overlay_upper(dir, dir).map_err(|e| format!("scan {}: {e}", dir.display()))?;

    let plan = ImageBuildPlan {
        tag: tag.to_string(),
        base_image: None,
        layers: vec![LayerStrategy::Files {
            paths: vec![dir.display().to_string()],
        }],
        labels: vec![],
        entrypoint: None,
    };

    std::fs::create_dir_all(output)
        .map_err(|e| format!("create output dir {}: {e}", output.display()))?;

    let image = assemble_image(
        &plan,
        &[scan.entries],
        output,
        &OciLayerConfig::default(),
        None,
    )?;

    // Refs #210: do not report success on an unverified effect. The layout is
    // the artifact — read it back before claiming it exists.
    verify_layout(output)?;

    let layer_count = image.layers.len();
    let file_count: u32 = image.layers.iter().map(|l| l.file_count).sum();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "source": dir.display().to_string(),
                "tag": tag,
                "output": output.display().to_string(),
                "layers": layer_count,
                "files": file_count,
                "total_size": image.total_size,
                "manifest": image.manifest,
            }))
            .map_err(|e| format!("serialize oci-pack output: {e}"))?
        );
    } else {
        println!("OCI Pack: {} -> {}", dir.display(), output.display());
        println!("  tag: {tag}");
        println!(
            "  layers: {layer_count} ({file_count} files, {} bytes)",
            image.total_size
        );
        println!("  layout: {}", output.display());
        println!("  index:  {}", output.join("index.json").display());
    }
    Ok(())
}

/// Refs #213: distinguish "absent" from "present but not a directory".
///
/// The old guard was `!dir.is_dir()` reported as "does not exist", so an
/// existing FILE produced a message that named the wrong cause and sent the
/// operator looking for a typo in a path that was right there.
fn check_pack_dir(dir: &Path) -> Result<(), String> {
    if dir.is_dir() {
        return Ok(());
    }
    if dir.exists() {
        Err(format!(
            "'{}' is not a directory (oci-pack packs a directory into a layer)",
            dir.display()
        ))
    } else {
        Err(format!("directory '{}' does not exist", dir.display()))
    }
}

/// Postcondition: a claimed OCI layout must have the files an OCI layout has.
fn verify_layout(output: &Path) -> Result<(), String> {
    for required in ["oci-layout", "index.json"] {
        let p = output.join(required);
        if !p.exists() {
            return Err(format!("oci-pack wrote no {required} at {}", p.display()));
        }
    }
    let blobs = output.join("blobs/sha256");
    let blob_count = std::fs::read_dir(&blobs)
        .map_err(|e| format!("read {}: {e}", blobs.display()))?
        .count();
    if blob_count == 0 {
        return Err(format!("oci-pack wrote no blobs under {}", blobs.display()));
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests_oci_pack.rs"]
mod tests;
