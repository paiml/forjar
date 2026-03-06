//! Demonstrates FJ-2103 overlay-to-OCI conversion: scanning overlay upper
//! directories, detecting whiteouts, and producing OCI-compatible layer entries.

use forjar::core::store::overlay_export::{
    format_overlay_scan, merge_overlay_entries, scan_overlay_upper, whiteouts_to_entries,
};
use forjar::core::types::WhiteoutEntry;

fn main() {
    println!("=== FJ-2103: Overlay → OCI Layer Conversion ===\n");

    // Create a temporary overlay upper directory with mixed content
    let dir = tempfile::tempdir().unwrap();
    let upper = dir.path();

    // Regular files (new or modified in upper)
    std::fs::create_dir_all(upper.join("etc/nginx")).unwrap();
    std::fs::write(upper.join("etc/nginx/nginx.conf"), "worker_processes auto;\n").unwrap();
    std::fs::create_dir_all(upper.join("usr/local/bin")).unwrap();
    std::fs::write(upper.join("usr/local/bin/app"), "#!/bin/sh\nexec /app\n").unwrap();

    // OCI whiteout: file deleted from lower layer
    std::fs::write(upper.join("etc/nginx/.wh.old-site.conf"), "").unwrap();

    // OCI opaque whiteout: entire directory replaced
    std::fs::create_dir_all(upper.join("var/cache/apt")).unwrap();
    std::fs::write(upper.join("var/cache/apt/.wh..wh..opq"), "").unwrap();
    std::fs::write(upper.join("var/cache/apt/pkgcache.bin"), "cached").unwrap();

    // Scan the overlay upper directory
    println!("--- Scanning overlay upper directory ---");
    let scan = scan_overlay_upper(upper, upper).unwrap();

    println!("  {}", format_overlay_scan(&scan));
    println!("  Regular entries: {}", scan.entries.len());
    println!("  Whiteout entries: {}", scan.whiteouts.len());
    println!();

    // Show detected whiteouts
    println!("--- Detected Whiteouts ---");
    for w in &scan.whiteouts {
        match w {
            WhiteoutEntry::FileDelete { path } => {
                println!("  DELETE  {path}  ->  OCI: {}", w.oci_path());
            }
            WhiteoutEntry::OpaqueDir { path } => {
                println!("  OPAQUE  {path}/  ->  OCI: {}", w.oci_path());
            }
        }
    }
    println!();

    // Show regular file entries
    println!("--- Regular File Entries ---");
    for entry in &scan.entries {
        println!("  {:>6} bytes  {:04o}  {}", entry.content.len(), entry.mode, entry.path);
    }
    println!();

    // Convert whiteouts to OCI marker entries
    println!("--- Whiteout → OCI Marker Entries ---");
    let markers = whiteouts_to_entries(&scan.whiteouts);
    for marker in &markers {
        println!("  {:>6} bytes  {:04o}  {}", marker.content.len(), marker.mode, marker.path);
    }
    println!();

    // Merge all entries for layer construction
    println!("--- Merged Layer Entries ---");
    let merged = merge_overlay_entries(&scan);
    println!("  Total entries: {} (regular) + {} (whiteout markers) = {}",
        scan.entries.len(), scan.whiteouts.len(), merged.len());
    for entry in &merged {
        let kind = if entry.content.is_empty() && entry.path.contains(".wh.") {
            "whiteout"
        } else {
            "file"
        };
        println!("  [{kind:>8}]  {}", entry.path);
    }
    println!();

    // Demonstrate standalone whiteout conversion
    println!("--- Standalone Whiteout Conversion ---");
    let whiteouts = vec![
        WhiteoutEntry::FileDelete { path: "etc/removed.conf".into() },
        WhiteoutEntry::FileDelete { path: "usr/bin/old-tool".into() },
        WhiteoutEntry::OpaqueDir { path: "tmp".into() },
    ];
    for w in &whiteouts {
        println!("  {:?}  ->  {}", w, w.oci_path());
    }

    println!("\nDone.");
}
