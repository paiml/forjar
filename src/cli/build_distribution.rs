//! FJ-2105/FJ-2106: Image distribution — load, push, FAR archive.
//!
//! Split from `build_image.rs` to stay under 500-line limit.

use crate::core::types::Resource;
use std::path::Path;

/// FJ-2106: Handle --load flag — tar OCI layout and pipe to docker/podman load.
pub(crate) fn cmd_build_load(oci_dir: &Path) -> Result<(), String> {
    let runtime = if super::dispatch_misc_b::which_runtime("docker") {
        "docker"
    } else if super::dispatch_misc_b::which_runtime("podman") {
        "podman"
    } else {
        return Err("--load requires docker or podman on PATH".into());
    };

    println!("\n--load: piping OCI tarball to `{runtime} load`...");
    let tar_output = std::process::Command::new("tar")
        .arg("-cf")
        .arg("-")
        .arg("-C")
        .arg(oci_dir)
        .arg(".")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn tar: {e}"))?;

    let status = std::process::Command::new(runtime)
        .arg("load")
        .stdin(tar_output.stdout.ok_or("tar stdout unavailable")?)
        .status()
        .map_err(|e| format!("{runtime} load: {e}"))?;

    if status.success() {
        println!("  loaded into {runtime}");
        Ok(())
    } else {
        Err(format!("{runtime} load exited with {status}"))
    }
}

/// FJ-2107: Handle --far flag — wrap OCI layout in a FAR archive.
pub(crate) fn cmd_build_far(resource: &str, oci_dir: &Path) -> Result<(), String> {
    use crate::core::store::far::{encode_far, FarManifest, FarProvenance};

    let mut files = Vec::new();
    let mut chunks = Vec::new();
    let mut total_size: u64 = 0;

    collect_far_files(oci_dir, oci_dir, &mut files, &mut chunks, &mut total_size)?;

    let tree_hash = if chunks.is_empty() {
        blake3::hash(b"empty").to_hex().to_string()
    } else {
        let mut hasher = blake3::Hasher::new();
        for (h, _) in &chunks {
            hasher.update(h);
        }
        hasher.finalize().to_hex().to_string()
    };

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let manifest = FarManifest {
        name: resource.to_string(),
        version: "1.0.0".to_string(),
        arch: std::env::consts::ARCH.to_string(),
        store_hash: tree_hash.clone(),
        tree_hash,
        file_count: files.len() as u64,
        total_size,
        files,
        provenance: FarProvenance {
            origin_provider: "forjar-build".to_string(),
            origin_ref: None,
            origin_hash: None,
            created_at: format!("{ts}"),
            generator: format!("forjar {}", env!("CARGO_PKG_VERSION")),
        },
        kernel_contracts: None,
    };

    let far_path = oci_dir.with_extension("far");
    let file = std::fs::File::create(&far_path).map_err(|e| format!("create FAR: {e}"))?;
    let writer = std::io::BufWriter::new(file);
    encode_far(&manifest, &chunks, writer)?;

    let far_size = std::fs::metadata(&far_path).map(|m| m.len()).unwrap_or(0);
    println!("\n--far: {}", far_path.display());
    println!(
        "  {} files, {} bytes -> {} bytes FAR",
        manifest.file_count, total_size, far_size
    );
    Ok(())
}

/// Recursively collect files from OCI dir into FAR entries and chunks.
fn collect_far_files(
    base: &Path,
    dir: &Path,
    files: &mut Vec<crate::core::store::far::FarFileEntry>,
    chunks: &mut Vec<([u8; 32], Vec<u8>)>,
    total_size: &mut u64,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_far_files(base, &path, files, chunks, total_size)?;
        } else {
            let data = std::fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let hash = blake3::hash(&data);
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            *total_size += data.len() as u64;
            files.push(crate::core::store::far::FarFileEntry {
                path: rel,
                size: data.len() as u64,
                blake3: hash.to_hex().to_string(),
            });
            chunks.push((*hash.as_bytes(), data));
        }
    }
    Ok(())
}

/// FJ-2105: Handle --push flag for registry push.
pub(crate) fn cmd_build_push(res: &Resource, oci_dir: &Path) -> Result<(), String> {
    use crate::core::store::registry_push;

    let image_name = res.name.as_deref().unwrap_or("app");
    let tag = res.version.as_deref().unwrap_or("latest");

    let (registry, name) = if let Some(idx) = image_name.find('/') {
        (&image_name[..idx], &image_name[idx + 1..])
    } else {
        ("docker.io", image_name)
    };

    let push_config = registry_push::RegistryPushConfig {
        registry: registry.to_string(),
        name: name.to_string(),
        tag: tag.to_string(),
        check_existing: true,
    };

    let errors = registry_push::validate_push_config(&push_config);
    if !errors.is_empty() {
        return Err(format!("push config invalid: {}", errors.join(", ")));
    }

    println!("\n--push: OCI Distribution v1.1");
    println!("  registry: {registry}");
    println!("  name: {name}");
    println!("  tag: {tag}");

    let blobs = registry_push::discover_blobs(oci_dir)?;
    if blobs.is_empty() {
        println!("  no blobs to push");
        return Ok(());
    }
    println!("  blobs: {} to push", blobs.len());

    match registry_push::push_image(oci_dir, &push_config) {
        Ok(results) => print!("{}", registry_push::format_push_summary(&results)),
        Err(e) if e.contains("Location header") || e.contains("curl") => {
            println!("  push skipped: registry unreachable ({e})");
        }
        Err(e) => return Err(e),
    }
    Ok(())
}
