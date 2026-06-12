//! PMAT-080 (DIST-1): Checksum resolution from GitHub Release assets.
//!
//! Resolves real per-asset SHA256 checksums and the release version for a
//! pinned tag so generators emit working artifacts instead of
//! `PLACEHOLDER_CHECKSUM` / `VERSION` literals (spec F-3608, F-3610).
//!
//! Resolution order:
//! 1. `--checksums-file <path>` — local SHA256SUMS-format file (offline).
//! 2. Combined `SHA256SUMS` asset from the GitHub release.
//! 3. Per-asset `<archive>.sha256` files from the GitHub release.

use crate::core::types::DistConfig;
use std::path::Path;

/// Release data resolved for a pinned tag: version + per-asset checksums.
#[derive(Debug, Clone)]
pub struct ResolvedRelease {
    /// Version without leading `v` (e.g., "1.4.3").
    pub version: String,
    /// Asset filename → SHA256 hex digest.
    pub checksums: indexmap::IndexMap<String, String>,
}

impl ResolvedRelease {
    /// Look up the checksum for an asset filename; the error names the
    /// missing asset and suggests `--checksums-file`.
    pub fn checksum_for(&self, asset: &str) -> Result<&str, String> {
        self.checksums
            .get(asset)
            .map(String::as_str)
            .ok_or_else(|| {
                format!(
                    "no checksum found for asset '{asset}' — ensure the release ships \
                 SHA256SUMS (or {asset}.sha256), or pass --checksums-file <path>"
                )
            })
    }

    /// Expand `{version}` in an asset template and look up its checksum.
    pub fn asset_checksum(&self, asset_template: &str) -> Result<(String, String), String> {
        let asset = asset_template.replace("{version}", &self.version);
        let sha = self.checksum_for(&asset)?.to_string();
        Ok((asset, sha))
    }
}

/// True if `s` is a 64-char lowercase/uppercase hex SHA256 digest.
fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Parse SHA256SUMS-format content: one `<hex>  <filename>` entry per line.
/// Tolerates the `*filename` binary-mode marker and skips malformed lines.
pub fn parse_sha256sums(content: &str) -> indexmap::IndexMap<String, String> {
    let mut map = indexmap::IndexMap::new();
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let (Some(hash), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        if !is_sha256_hex(hash) {
            continue;
        }
        let name = name.trim_start_matches('*');
        map.insert(name.to_string(), hash.to_lowercase());
    }
    map
}

/// Resolve the release version + checksums for a pinned tag.
///
/// Hard-errors (never falls back to placeholders) when the tag is missing
/// or no checksum data can be resolved.
pub fn resolve_release(
    dist: &DistConfig,
    tag: Option<&str>,
    checksums_file: Option<&Path>,
) -> Result<ResolvedRelease, String> {
    let tag = tag.ok_or_else(|| {
        "--homebrew and --nix embed real checksums and require --version <TAG> \
         (e.g., --version v1.4.3); placeholders are never emitted"
            .to_string()
    })?;
    let version = tag.trim_start_matches('v').to_string();
    if version.is_empty() {
        return Err(format!("invalid --version '{tag}': empty version"));
    }

    let checksums = match checksums_file {
        Some(path) => parse_checksums_file(path)?,
        None => fetch_release_checksums(dist, tag, &version)?,
    };
    Ok(ResolvedRelease { version, checksums })
}

/// Parse a local SHA256SUMS-format file (offline resolution).
fn parse_checksums_file(path: &Path) -> Result<indexmap::IndexMap<String, String>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read checksums file {}: {e}", path.display()))?;
    let map = parse_sha256sums(&content);
    if map.is_empty() {
        return Err(format!(
            "no checksum entries in {} — expected '<sha256>  <filename>' lines",
            path.display()
        ));
    }
    Ok(map)
}

/// Fetch checksums from the GitHub release: combined SHA256SUMS asset first,
/// then per-asset `<archive>.sha256` files as fallback.
fn fetch_release_checksums(
    dist: &DistConfig,
    tag: &str,
    version: &str,
) -> Result<indexmap::IndexMap<String, String>, String> {
    let base = format!("https://github.com/{}/releases/download/{tag}", dist.repo);
    let sums_asset = dist.checksums.as_deref().unwrap_or("SHA256SUMS");

    if let Ok(content) = fetch_url(&format!("{base}/{sums_asset}")) {
        let map = parse_sha256sums(&content);
        if !map.is_empty() {
            return Ok(map);
        }
    }
    fetch_per_asset_checksums(dist, &base, version, sums_asset)
}

/// Fallback: fetch `<asset>.sha256` for every target asset.
fn fetch_per_asset_checksums(
    dist: &DistConfig,
    base: &str,
    version: &str,
    sums_asset: &str,
) -> Result<indexmap::IndexMap<String, String>, String> {
    let mut map = indexmap::IndexMap::new();
    let mut missing: Vec<String> = Vec::new();
    for t in &dist.targets {
        let asset = t.asset.replace("{version}", version);
        match fetch_url(&format!("{base}/{asset}.sha256")) {
            Ok(content) => match parse_single_checksum(&content) {
                Some(hash) => {
                    map.insert(asset, hash);
                }
                None => missing.push(asset),
            },
            Err(_) => missing.push(asset),
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "cannot resolve checksums for release asset(s) {} — neither '{sums_asset}' \
             nor per-asset '.sha256' files found at {base}; \
             use --checksums-file <path> to provide checksums offline",
            missing.join(", ")
        ));
    }
    Ok(map)
}

/// Parse a single-checksum file: either a bare hash or `<hash>  <filename>`.
fn parse_single_checksum(content: &str) -> Option<String> {
    let hash = content.split_whitespace().next()?;
    is_sha256_hex(hash).then(|| hash.to_lowercase())
}

/// Download a URL via curl (same pattern as `github_release` and OCI push).
fn fetch_url(url: &str) -> Result<String, String> {
    let output = std::process::Command::new("curl")
        .args(["-fsSL", url])
        .output()
        .map_err(|e| format!("curl {url}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "download failed: {url} (curl exit {})",
            output.status.code().unwrap_or(-1)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{DistBinaryTarget, DistConfig};

    const SHA_A: &str = "a3f5c2e88b1d4a6f9c0e7b2d5a8f1c4e7b0d3a6f9c2e5b8d1a4f7c0e3b6d9a2f";
    const SHA_B: &str = "b4e6d3f99c2e5b7fad1f8c3e6b9f2d5f8c1e4b7fad3f6c9e2b5d8f1c4e7bad3f";

    fn sample_dist() -> DistConfig {
        DistConfig {
            source: "github_release".into(),
            repo: "acme/tool".into(),
            binary: "mytool".into(),
            targets: vec![DistBinaryTarget {
                os: "linux".into(),
                arch: "x86_64".into(),
                asset: "mytool-{version}-x86_64-unknown-linux-gnu.tar.gz".into(),
                libc: Some("gnu".into()),
            }],
            install_dir: "/usr/local/bin".into(),
            install_dir_fallback: "~/.local/bin".into(),
            checksums: Some("SHA256SUMS".into()),
            checksum_algo: "sha256".into(),
            description: "A test tool".into(),
            homepage: "https://example.com".into(),
            license: "MIT".into(),
            maintainer: "Test".into(),
            version_cmd: None,
            latest_tag: true,
            post_install: None,
            homebrew: None,
            nix: None,
        }
    }

    fn sums_content() -> String {
        format!(
            "{SHA_A}  mytool-1.2.3-x86_64-unknown-linux-gnu.tar.gz\n\
             {SHA_B}  mytool-1.2.3-aarch64-apple-darwin.tar.gz\n"
        )
    }

    #[test]
    fn parse_sha256sums_two_space_format() {
        let map = parse_sha256sums(&sums_content());
        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get("mytool-1.2.3-x86_64-unknown-linux-gnu.tar.gz"),
            Some(&SHA_A.to_string())
        );
    }

    #[test]
    fn parse_sha256sums_binary_mode_marker() {
        let content = format!("{SHA_A} *mytool-1.2.3-x86_64-unknown-linux-gnu.tar.gz\n");
        let map = parse_sha256sums(&content);
        assert_eq!(
            map.get("mytool-1.2.3-x86_64-unknown-linux-gnu.tar.gz"),
            Some(&SHA_A.to_string())
        );
    }

    #[test]
    fn parse_sha256sums_skips_malformed_lines() {
        let content = format!("not-a-hash  file.tar.gz\n\n{SHA_A}  good.tar.gz\njusthash\n");
        let map = parse_sha256sums(&content);
        assert_eq!(map.len(), 1);
        assert!(map.contains_key("good.tar.gz"));
    }

    #[test]
    fn parse_sha256sums_lowercases_hash() {
        let content = format!("{}  file.tar.gz\n", SHA_A.to_uppercase());
        let map = parse_sha256sums(&content);
        assert_eq!(map.get("file.tar.gz"), Some(&SHA_A.to_string()));
    }

    #[test]
    fn parse_sha256sums_empty_input() {
        assert!(parse_sha256sums("").is_empty());
    }

    #[test]
    fn is_sha256_hex_accepts_valid() {
        assert!(is_sha256_hex(SHA_A));
    }

    #[test]
    fn is_sha256_hex_rejects_short_and_nonhex() {
        assert!(!is_sha256_hex("abc123"));
        assert!(!is_sha256_hex(&"z".repeat(64)));
    }

    #[test]
    fn parse_single_checksum_bare_hash() {
        assert_eq!(parse_single_checksum(SHA_A), Some(SHA_A.to_string()));
    }

    #[test]
    fn parse_single_checksum_with_filename() {
        let content = format!("{SHA_A}  mytool-1.2.3.tar.gz\n");
        assert_eq!(parse_single_checksum(&content), Some(SHA_A.to_string()));
    }

    #[test]
    fn parse_single_checksum_rejects_garbage() {
        assert_eq!(parse_single_checksum("not a hash"), None);
        assert_eq!(parse_single_checksum(""), None);
    }

    #[test]
    fn resolve_release_requires_version_tag() {
        let err = resolve_release(&sample_dist(), None, None).unwrap_err();
        assert!(
            err.contains("--version"),
            "error must name --version: {err}"
        );
        assert!(
            err.contains("placeholders are never emitted"),
            "error must explain why: {err}"
        );
    }

    #[test]
    fn resolve_release_rejects_empty_version() {
        let err = resolve_release(&sample_dist(), Some("v"), None).unwrap_err();
        assert!(err.contains("empty version"), "got: {err}");
    }

    #[test]
    fn resolve_release_from_checksums_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("SHA256SUMS");
        std::fs::write(&path, sums_content()).unwrap();
        let release = resolve_release(&sample_dist(), Some("v1.2.3"), Some(&path)).unwrap();
        assert_eq!(release.version, "1.2.3");
        assert_eq!(release.checksums.len(), 2);
    }

    #[test]
    fn resolve_release_strips_v_prefix_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("SHA256SUMS");
        std::fs::write(&path, sums_content()).unwrap();
        let release = resolve_release(&sample_dist(), Some("1.2.3"), Some(&path)).unwrap();
        assert_eq!(release.version, "1.2.3");
    }

    #[test]
    fn resolve_release_missing_file_errors() {
        let err = resolve_release(
            &sample_dist(),
            Some("v1.2.3"),
            Some(Path::new("/nonexistent/SHA256SUMS")),
        )
        .unwrap_err();
        assert!(err.contains("cannot read checksums file"), "got: {err}");
    }

    #[test]
    fn resolve_release_empty_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("SHA256SUMS");
        std::fs::write(&path, "no valid lines here\n").unwrap();
        let err = resolve_release(&sample_dist(), Some("v1.2.3"), Some(&path)).unwrap_err();
        assert!(err.contains("no checksum entries"), "got: {err}");
    }

    #[test]
    fn checksum_for_known_asset() {
        let release = ResolvedRelease {
            version: "1.2.3".into(),
            checksums: parse_sha256sums(&sums_content()),
        };
        assert_eq!(
            release
                .checksum_for("mytool-1.2.3-x86_64-unknown-linux-gnu.tar.gz")
                .unwrap(),
            SHA_A
        );
    }

    #[test]
    fn checksum_for_missing_asset_names_it() {
        let release = ResolvedRelease {
            version: "1.2.3".into(),
            checksums: parse_sha256sums(&sums_content()),
        };
        let err = release
            .checksum_for("mytool-1.2.3-missing.tar.gz")
            .unwrap_err();
        assert!(err.contains("mytool-1.2.3-missing.tar.gz"), "got: {err}");
        assert!(err.contains("--checksums-file"), "got: {err}");
    }

    #[test]
    fn asset_checksum_expands_version_template() {
        let release = ResolvedRelease {
            version: "1.2.3".into(),
            checksums: parse_sha256sums(&sums_content()),
        };
        let (asset, sha) = release
            .asset_checksum("mytool-{version}-x86_64-unknown-linux-gnu.tar.gz")
            .unwrap();
        assert_eq!(asset, "mytool-1.2.3-x86_64-unknown-linux-gnu.tar.gz");
        assert_eq!(sha, SHA_A);
    }
}
