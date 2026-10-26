//! ISO payload helpers split out of `image_cmd` (500-line file limit): extract a file
//! from the base ISO, embed the forjar binary, repack with the staged payload.

use std::path::Path;

/// Extract one file from an ISO. Ok(false) when the ISO does not have it.
pub(super) fn extract_iso_file(
    base_iso: &Path,
    iso_path: &str,
    dst: &Path,
) -> Result<bool, String> {
    let out = std::process::Command::new("xorriso")
        .args(["-osirrox", "on", "-indev"])
        .arg(base_iso)
        .args(["-extract", iso_path])
        .arg(dst)
        .output()
        .map_err(|e| format!("run xorriso extract: {e}"))?;
    Ok(out.status.success() && dst.exists())
}

/// Copy the current forjar binary into the staged payload.
pub(super) fn embed_forjar_binary(work: &Path) -> Result<(), crate::core::error::ForjarError> {
    let bin_dir = work.join("forjar").join("bin");
    std::fs::create_dir_all(&bin_dir).map_err(|e| format!("create bin dir: {e}"))?;
    if let Ok(exe) = std::env::current_exe() {
        std::fs::copy(&exe, bin_dir.join("forjar"))
            .map_err(|e| format!("copy forjar binary: {e}"))?;
    }
    Ok(())
}

/// Write `output` as the base ISO plus the staged payload.
///
/// #731: `-boot_image any replay` reproduces the base ISO's El Torito, MBR/GPT
/// and appended EFI partition exactly, so a signed shim/grub/kernel boot as
/// they did. The old `-as mkisofs` repack named `boot/grub/efi.img` and
/// `boot_hybrid.img`, which Ubuntu 24.04 does not have in its ISO tree.
pub(super) fn repack_iso(
    work: &Path,
    base_iso: &Path,
    output: &Path,
    grub_cfgs: &[(String, std::path::PathBuf)],
) -> Result<(), String> {
    if output.exists() {
        std::fs::remove_file(output).map_err(|e| format!("remove {}: {e}", output.display()))?;
    }
    let mut cmd = std::process::Command::new("xorriso");
    cmd.arg("-indev")
        .arg(base_iso)
        .arg("-outdev")
        .arg(output)
        .args(["-boot_image", "any", "replay", "-map"])
        .arg(work.join("nocloud"))
        .arg("/nocloud")
        .arg("-map")
        .arg(work.join("forjar"))
        .arg("/forjar");
    for (iso_path, staged) in grub_cfgs {
        cmd.arg("-map").arg(staged).arg(iso_path);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("run xorriso repack: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "xorriso ISO repack failed: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .last()
                .unwrap_or("")
        ));
    }
    Ok(())
}
