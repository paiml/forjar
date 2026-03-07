//! FJ-2103: Overlay upper directory → OCI layer conversion.
//!
//! Converts an overlayfs upper directory into an OCI-compatible layer
//! by walking the directory tree, detecting whiteout entries, and
//! producing `LayerEntry` objects for `build_layer()`.
//!
//! Overlay whiteouts:
//! - Character device (0,0): delete file in lower
//! - Directory with trusted.overlay.opaque xattr: opaque dir
//! - `.wh.<name>` files: OCI-native whiteout (for pre-processed dirs)

use crate::core::store::layer_builder::LayerEntry;
use crate::core::types::WhiteoutEntry;
use std::path::Path;

/// Result of scanning an overlay upper directory.
#[derive(Debug, Clone)]
pub struct OverlayScan {
    /// Regular files/directories to include in the layer.
    pub entries: Vec<LayerEntry>,
    /// Whiteout entries (deletions in lower layers).
    pub whiteouts: Vec<WhiteoutEntry>,
    /// Total bytes of regular files.
    pub total_bytes: u64,
    /// Number of files scanned.
    pub file_count: usize,
}

/// Scan an overlay upper directory and produce layer entries + whiteouts.
///
/// The `upper_dir` is the path to the overlay upper directory.
/// The `strip_prefix` is removed from paths to produce container-relative paths.
///
/// # Examples
///
/// ```
/// use forjar::core::store::overlay_export::scan_overlay_upper;
///
/// let dir = tempfile::tempdir().unwrap();
/// let upper = dir.path().join("upper");
/// std::fs::create_dir_all(upper.join("etc")).unwrap();
/// std::fs::write(upper.join("etc/app.conf"), "key=value").unwrap();
///
/// let scan = scan_overlay_upper(&upper, &upper).unwrap();
/// assert_eq!(scan.file_count, 1);
/// assert!(scan.whiteouts.is_empty());
/// assert_eq!(scan.entries.len(), 1);
/// ```
pub fn scan_overlay_upper(upper_dir: &Path, strip_prefix: &Path) -> Result<OverlayScan, String> {
    let mut entries = Vec::new();
    let mut whiteouts = Vec::new();
    let mut total_bytes: u64 = 0;
    let mut file_count: usize = 0;

    walk_dir(
        upper_dir,
        strip_prefix,
        &mut entries,
        &mut whiteouts,
        &mut total_bytes,
        &mut file_count,
    )?;

    Ok(OverlayScan {
        entries,
        whiteouts,
        total_bytes,
        file_count,
    })
}

fn walk_dir(
    dir: &Path,
    strip_prefix: &Path,
    entries: &mut Vec<LayerEntry>,
    whiteouts: &mut Vec<WhiteoutEntry>,
    total_bytes: &mut u64,
    file_count: &mut usize,
) -> Result<(), String> {
    let read_dir =
        std::fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Compute container-relative path
        let rel_path = path
            .strip_prefix(strip_prefix)
            .map_err(|e| format!("strip prefix: {e}"))?
            .to_string_lossy()
            .to_string();

        // Check for OCI-style whiteout markers (.wh.*)
        if let Some(suffix) = file_name.strip_prefix(".wh.") {
            let parent = path
                .parent()
                .and_then(|p| p.strip_prefix(strip_prefix).ok())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            if suffix == ".wh..opq" {
                // Opaque directory marker
                whiteouts.push(WhiteoutEntry::OpaqueDir { path: parent });
            } else {
                // File deletion: .wh.<filename>
                let deleted_path = if parent.is_empty() {
                    suffix.to_string()
                } else {
                    format!("{parent}/{suffix}")
                };
                whiteouts.push(WhiteoutEntry::FileDelete { path: deleted_path });
            }
            continue;
        }

        if path.is_dir() {
            walk_dir(
                &path,
                strip_prefix,
                entries,
                whiteouts,
                total_bytes,
                file_count,
            )?;
        } else if path.is_file() {
            let content =
                std::fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let mode = file_mode(&path);
            *total_bytes += content.len() as u64;
            *file_count += 1;
            entries.push(LayerEntry::file(&rel_path, &content, mode));
        }
    }

    Ok(())
}

/// Convert whiteouts to LayerEntry objects (empty marker files).
///
/// OCI spec: whiteout files are zero-length regular files named `.wh.<name>`.
pub fn whiteouts_to_entries(whiteouts: &[WhiteoutEntry]) -> Vec<LayerEntry> {
    whiteouts
        .iter()
        .map(|w| LayerEntry::file(&w.oci_path(), &[], 0o644))
        .collect()
}

/// Merge regular entries and whiteout entries into a single layer entry set.
pub fn merge_overlay_entries(scan: &OverlayScan) -> Vec<LayerEntry> {
    let mut all = scan.entries.clone();
    all.extend(whiteouts_to_entries(&scan.whiteouts));
    all
}

/// Format overlay scan result for human output.
pub fn format_overlay_scan(scan: &OverlayScan) -> String {
    format!(
        "Overlay scan: {} files ({:.1} KB), {} whiteouts",
        scan.file_count,
        scan.total_bytes as f64 / 1024.0,
        scan.whiteouts.len(),
    )
}

/// Get file mode from metadata (Unix-specific, fallback on non-Unix).
fn file_mode(path: &Path) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o777)
            .unwrap_or(0o644)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        0o644
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
        assert_eq!(scan.file_count, 0);
        assert!(scan.entries.is_empty());
        assert!(scan.whiteouts.is_empty());
        assert_eq!(scan.total_bytes, 0);
    }

    #[test]
    fn scan_regular_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("etc")).unwrap();
        std::fs::write(dir.path().join("etc/app.conf"), "key=value\n").unwrap();
        std::fs::write(dir.path().join("etc/other.conf"), "x=1\n").unwrap();

        let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
        assert_eq!(scan.file_count, 2);
        assert_eq!(scan.entries.len(), 2);
        assert!(scan.whiteouts.is_empty());
        assert!(scan.total_bytes > 0);
    }

    #[test]
    fn scan_detects_file_whiteout() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("etc")).unwrap();
        std::fs::write(dir.path().join("etc/.wh.old.conf"), "").unwrap();

        let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
        assert_eq!(scan.file_count, 0); // whiteout is not a regular file
        assert_eq!(scan.whiteouts.len(), 1);
        assert_eq!(
            scan.whiteouts[0],
            WhiteoutEntry::FileDelete {
                path: "etc/old.conf".into()
            }
        );
    }

    #[test]
    fn scan_detects_opaque_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("var/cache")).unwrap();
        std::fs::write(dir.path().join("var/cache/.wh..wh..opq"), "").unwrap();

        let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
        assert_eq!(scan.whiteouts.len(), 1);
        assert_eq!(
            scan.whiteouts[0],
            WhiteoutEntry::OpaqueDir {
                path: "var/cache".into()
            }
        );
    }

    #[test]
    fn scan_mixed_files_and_whiteouts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("app")).unwrap();
        std::fs::write(dir.path().join("app/new.txt"), "hello").unwrap();
        std::fs::write(dir.path().join("app/.wh.deleted.txt"), "").unwrap();

        let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
        assert_eq!(scan.file_count, 1);
        assert_eq!(scan.entries.len(), 1);
        assert_eq!(scan.whiteouts.len(), 1);
    }

    #[test]
    fn whiteouts_to_entries_conversion() {
        let whiteouts = vec![
            WhiteoutEntry::FileDelete {
                path: "etc/old.conf".into(),
            },
            WhiteoutEntry::OpaqueDir {
                path: "var/cache".into(),
            },
        ];
        let entries = whiteouts_to_entries(&whiteouts);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn merge_overlay_entries_combines() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("app")).unwrap();
        std::fs::write(dir.path().join("app/new.txt"), "hello").unwrap();
        std::fs::write(dir.path().join("app/.wh.old.txt"), "").unwrap();

        let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
        let merged = merge_overlay_entries(&scan);
        assert_eq!(merged.len(), 2); // 1 file + 1 whiteout entry
    }

    #[test]
    fn format_overlay_scan_output() {
        let scan = OverlayScan {
            entries: vec![],
            whiteouts: vec![WhiteoutEntry::FileDelete { path: "x".into() }],
            total_bytes: 2048,
            file_count: 5,
        };
        let s = format_overlay_scan(&scan);
        assert!(s.contains("5 files"));
        assert!(s.contains("2.0 KB"));
        assert!(s.contains("1 whiteout"));
    }

    #[test]
    fn scan_nested_directories() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("a/b/c")).unwrap();
        std::fs::write(dir.path().join("a/b/c/deep.txt"), "deep").unwrap();

        let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
        assert_eq!(scan.file_count, 1);
        // Path should be relative
        let path = &scan.entries[0].path;
        assert!(path.contains("a/b/c/deep.txt") || path.contains("a\\b\\c\\deep.txt"));
    }

    #[test]
    fn scan_root_whiteout() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".wh.rootfile"), "").unwrap();

        let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
        assert_eq!(scan.whiteouts.len(), 1);
        assert_eq!(
            scan.whiteouts[0],
            WhiteoutEntry::FileDelete {
                path: "rootfile".into()
            }
        );
    }

    #[test]
    fn scan_missing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no_such_subdir");
        let result = scan_overlay_upper(&missing, &missing);
        assert!(result.is_err());
    }
}
