//! FJ-3607 Tier 2: local tarball staging for offline installer runs.
//!
//! The generated installer extracts `${ASSET%.tar.gz}/$BINARY` from the
//! downloaded archive (the directory-inside-the-tarball layout the real
//! release workflow uses — pinned by `tests/install_sh_parity.rs`). To
//! exercise that path OFFLINE we stage a real `.tar.gz` with exactly that
//! layout: a `<asset-stem>/` directory containing an executable stub
//! `<binary>` that prints a version string when run with `--version`.
//!
//! Staging happens on the host; `dist_verify_tier2` copies the archive
//! into the container with `docker cp`. The stub binary is a POSIX-sh
//! script (works on both ubuntu and alpine without a real ELF).

use std::io::Write;

/// A minimal executable stub the installer can install and `--version`.
/// POSIX sh so it runs on alpine (no bash) and ubuntu alike.
fn stub_binary(binary: &str) -> String {
    format!(
        "#!/bin/sh\ncase \"$1\" in\n  --version|-V|version) echo \"{binary} 0.0.0-tier2\" ;;\n  *) echo \"{binary}: tier2 stub\" ;;\nesac\n"
    )
}

/// Build the gzipped tar bytes with the release-layout directory.
///
/// Layout: `<asset_stem>/<binary>` (executable), where `asset_stem` is the
/// asset name minus the `.tar.gz` suffix — exactly what the installer's
/// `SRC="$TMPDIR/${ASSET%.tar.gz}/$BINARY"` expects.
pub(crate) fn build_tarball_bytes(binary: &str, asset: &str) -> Result<Vec<u8>, String> {
    let stem = asset.strip_suffix(".tar.gz").unwrap_or(asset);
    let stub = stub_binary(binary);
    let stub_bytes = stub.as_bytes();

    let mut tar = tar::Builder::new(Vec::new());

    // Directory entry for <stem>/.
    let mut dir_header = tar::Header::new_gnu();
    dir_header.set_entry_type(tar::EntryType::Directory);
    dir_header.set_mode(0o755);
    dir_header.set_size(0);
    dir_header.set_cksum();
    tar.append_data(&mut dir_header, format!("{stem}/"), std::io::empty())
        .map_err(|e| format!("tar dir entry: {e}"))?;

    // Executable stub at <stem>/<binary>.
    let mut bin_header = tar::Header::new_gnu();
    bin_header.set_mode(0o755);
    bin_header.set_size(stub_bytes.len() as u64);
    bin_header.set_cksum();
    tar.append_data(&mut bin_header, format!("{stem}/{binary}"), stub_bytes)
        .map_err(|e| format!("tar bin entry: {e}"))?;

    let raw = tar.into_inner().map_err(|e| format!("tar finish: {e}"))?;
    gzip(&raw)
}

fn gzip(data: &[u8]) -> Result<Vec<u8>, String> {
    use flate2::{write::GzEncoder, Compression};
    let mut enc = GzEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(data)
        .map_err(|e| format!("gzip write: {e}"))?;
    enc.finish().map_err(|e| format!("gzip finish: {e}"))
}

/// Raw (un-prefixed) sha256 hex of the bytes — the form a SHA256SUMS file
/// uses (`<hex>  <asset>`), distinct from the OCI `sha256:` form.
pub(crate) fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(data))
}

/// One `<sha256hex>  <asset>` line, the SHA256SUMS format the installer's
/// `verify_checksum` greps for the asset name in.
pub(crate) fn sums_line(hex: &str, asset: &str) -> String {
    format!("{hex}  {asset}")
}

/// Stage tarball + its checksum line in one call (the form
/// `dist_verify_tier2` consumes): returns (host_path, sums_line).
pub(crate) fn stage_with_sums(binary: &str, asset: &str) -> Result<(String, String), String> {
    let bytes = build_tarball_bytes(binary, asset)?;
    let hex = sha256_hex(&bytes);
    let line = sums_line(&hex, asset);
    let path = std::env::temp_dir().join(format!("forjar-dist-t2-{}-{asset}", std::process::id()));
    std::fs::write(&path, &bytes).map_err(|e| format!("stage {}: {e}", path.display()))?;
    Ok((path.to_string_lossy().to_string(), line))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_responds_to_version() {
        let s = stub_binary("mytool");
        assert!(s.starts_with("#!/bin/sh"));
        assert!(s.contains("mytool 0.0.0-tier2"));
        assert!(s.contains("--version"));
    }

    #[test]
    fn tarball_has_release_directory_layout() {
        let bytes =
            build_tarball_bytes("mytool", "mytool-0.0.0-x86_64-unknown-linux-gnu.tar.gz").unwrap();
        // Decompress + read entries; the binary must live under <stem>/.
        use flate2::read::GzDecoder;
        let dec = GzDecoder::new(&bytes[..]);
        let mut ar = tar::Archive::new(dec);
        let paths: Vec<String> = ar
            .entries()
            .unwrap()
            .map(|e| e.unwrap().path().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(
            paths
                .iter()
                .any(|p| p == "mytool-0.0.0-x86_64-unknown-linux-gnu/mytool"),
            "binary must be under the asset-stem directory; got {paths:?}"
        );
    }

    #[test]
    fn tarball_binary_is_executable() {
        let bytes = build_tarball_bytes("mytool", "mytool-1-linux.tar.gz").unwrap();
        use flate2::read::GzDecoder;
        let dec = GzDecoder::new(&bytes[..]);
        let mut ar = tar::Archive::new(dec);
        for entry in ar.entries().unwrap() {
            let e = entry.unwrap();
            if e.path().unwrap().to_string_lossy().ends_with("/mytool") {
                assert_eq!(e.header().mode().unwrap() & 0o111, 0o111);
                return;
            }
        }
        panic!("no binary entry found");
    }

    #[test]
    fn sha256_hex_is_64_chars_no_prefix() {
        let hex = sha256_hex(b"hello");
        assert_eq!(hex.len(), 64);
        assert!(!hex.contains(':'));
        // Known SHA-256 of "hello".
        assert_eq!(
            hex,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn sums_line_is_sha256sums_format() {
        let line = sums_line("abc123", "mytool-1-linux.tar.gz");
        assert_eq!(line, "abc123  mytool-1-linux.tar.gz");
        // Two spaces, as `sha256sum` emits and `grep`/`awk` parse.
        assert!(line.contains("  "));
    }

    #[test]
    fn stage_with_sums_writes_file_and_matching_line() {
        let (path, line) = stage_with_sums("mytool", "mytool-1-linux.tar.gz").unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let hex = sha256_hex(&bytes);
        assert!(line.starts_with(&hex), "sums line must match staged bytes");
        assert!(line.ends_with("mytool-1-linux.tar.gz"));
        // The staged file is a real gzip (magic bytes).
        assert_eq!(&bytes[..2], &[0x1f, 0x8b]);
    }
}
