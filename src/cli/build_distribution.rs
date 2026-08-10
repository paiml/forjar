//! FJ-2105/FJ-2106: Image distribution — load, push, FAR archive.
//!
//! Split from `build_image.rs` to stay under 500-line limit.

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
///
/// `image_reference` is the reference the build just stamped into the image
/// (`plan.tag`), so the push cannot target anything other than what was built.
///
/// Refs #210: this function reports success only after the registry has been
/// asked, independently, whether the tag now resolves to the manifest we
/// pushed. Every failure — unreachable registry, authentication required,
/// rejected upload, unverifiable tag — is an `Err`, i.e. a non-zero exit.
/// It previously swallowed "no Location header" and "curl …" into
/// `push skipped: registry unreachable` and returned `Ok`, which is how a push
/// that never uploaded a byte exited 0.
pub(crate) fn cmd_build_push(image_reference: &str, oci_dir: &Path) -> Result<(), String> {
    use crate::core::store::{image_ref, registry_push, registry_push_http};

    let iref = image_ref::parse_image_ref(image_reference)?;
    let push_config = registry_push::RegistryPushConfig {
        registry: iref.api_host().to_string(),
        name: iref.repository.clone(),
        tag: iref.tag.clone(),
        check_existing: true,
    };

    let errors = registry_push::validate_push_config(&push_config);
    if !errors.is_empty() {
        return Err(format!("push config invalid: {}", errors.join(", ")));
    }

    println!("\n--push: OCI Distribution v1.1");
    println!("  reference: {}", iref.to_reference());
    println!(
        "  endpoint:  {}",
        registry_push_http::registry_url(iref.api_host(), &format!("v2/{}", iref.repository))
    );

    let blobs = registry_push::discover_blobs(oci_dir)?;
    if blobs.is_empty() {
        return Err(format!(
            "nothing to push: no blobs in OCI layout {} (build the image first)",
            oci_dir.display()
        ));
    }
    println!("  blobs: {} to push", blobs.len());

    let results = registry_push::push_image(oci_dir, &push_config)?;

    let manifest_digest = results
        .iter()
        .find(|r| r.kind == crate::core::types::PushKind::Manifest)
        .map(|r| r.digest.clone())
        .ok_or("push produced no manifest: refusing to report a completed push")?;
    registry_push_http::verify_manifest_pushed(&push_config, &manifest_digest)?;

    print!("{}", registry_push::format_push_summary(&results));
    println!(
        "  Verified: {} resolves to {manifest_digest} at the registry",
        iref.to_reference()
    );
    Ok(())
}
