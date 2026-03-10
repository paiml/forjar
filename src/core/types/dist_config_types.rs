//! FJ-3600: Distribution artifact config types.
//!
//! Defines the `dist:` section of forjar.yaml for generating
//! shell installers, Homebrew formulas, cargo-binstall metadata,
//! Nix flakes, GitHub Actions, and OS packages.

use serde::{Deserialize, Serialize};

/// FJ-3600: Top-level distribution config from `dist:` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistConfig {
    /// Source type: "github_release", "local", "url", "s3".
    pub source: String,

    /// GitHub org/repo (e.g., "paiml/forjar").
    #[serde(default)]
    pub repo: String,

    /// Binary name after install.
    pub binary: String,

    /// Build targets (OS/arch combos).
    #[serde(default)]
    pub targets: Vec<DistBinaryTarget>,

    /// Primary install directory.
    #[serde(default = "default_install_dir")]
    pub install_dir: String,

    /// Fallback install directory if primary not writable.
    #[serde(default = "default_install_dir_fallback")]
    pub install_dir_fallback: String,

    /// Asset name for checksum file (e.g., "SHA256SUMS").
    #[serde(default)]
    pub checksums: Option<String>,

    /// Checksum algorithm: "sha256" (default) or "blake3".
    #[serde(default = "default_checksum_algo")]
    pub checksum_algo: String,

    /// Package description.
    #[serde(default)]
    pub description: String,

    /// Project homepage URL.
    #[serde(default)]
    pub homepage: String,

    /// License string (e.g., "MIT OR Apache-2.0").
    #[serde(default)]
    pub license: String,

    /// Package maintainer.
    #[serde(default)]
    pub maintainer: String,

    /// Command to verify install (e.g., "forjar --version").
    #[serde(default)]
    pub version_cmd: Option<String>,

    /// Resolve latest GitHub tag automatically.
    #[serde(default)]
    pub latest_tag: bool,

    /// Post-install script.
    #[serde(default)]
    pub post_install: Option<String>,

    /// Homebrew-specific config.
    #[serde(default)]
    pub homebrew: Option<DistHomebrewConfig>,

    /// Nix-specific config.
    #[serde(default)]
    pub nix: Option<DistNixConfig>,
}

/// A single OS/arch build target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistBinaryTarget {
    /// Operating system: "linux", "darwin".
    pub os: String,

    /// Architecture: "x86_64", "aarch64".
    pub arch: String,

    /// Asset filename template (e.g., "forjar-{version}-x86_64-unknown-linux-gnu.tar.gz").
    pub asset: String,

    /// C library variant: "gnu", "musl".
    #[serde(default)]
    pub libc: Option<String>,
}

/// Homebrew-specific distribution config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistHomebrewConfig {
    /// Homebrew tap (e.g., "paiml/tap").
    pub tap: String,

    /// Brew dependencies.
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Caveats text shown after install.
    #[serde(default)]
    pub caveats: Option<String>,
}

/// Nix-specific distribution config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistNixConfig {
    /// Nix flake inputs (e.g., { "nixpkgs": "github:NixOS/nixpkgs/nixos-unstable" }).
    #[serde(default)]
    pub inputs: std::collections::HashMap<String, String>,

    /// Build inputs (Nix packages needed at build time).
    #[serde(default)]
    pub build_inputs: Vec<String>,
}

fn default_install_dir() -> String {
    "/usr/local/bin".to_string()
}

fn default_install_dir_fallback() -> String {
    "~/.local/bin".to_string()
}

fn default_checksum_algo() -> String {
    "sha256".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dist_config_deserialize_minimal() {
        let yaml = r#"
source: github_release
repo: paiml/forjar
binary: forjar
"#;
        let config: DistConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.source, "github_release");
        assert_eq!(config.repo, "paiml/forjar");
        assert_eq!(config.binary, "forjar");
        assert_eq!(config.install_dir, "/usr/local/bin");
        assert_eq!(config.install_dir_fallback, "~/.local/bin");
        assert_eq!(config.checksum_algo, "sha256");
        assert!(config.targets.is_empty());
    }

    #[test]
    fn dist_config_deserialize_full() {
        let yaml = r#"
source: github_release
repo: paiml/forjar
binary: forjar
targets:
  - os: linux
    arch: x86_64
    asset: "forjar-{version}-x86_64-unknown-linux-gnu.tar.gz"
    libc: gnu
  - os: darwin
    arch: aarch64
    asset: "forjar-{version}-aarch64-apple-darwin.tar.gz"
checksums: SHA256SUMS
description: "Rust-native Infrastructure as Code"
homepage: https://forjar.dev
license: "MIT OR Apache-2.0"
version_cmd: "forjar --version"
latest_tag: true
homebrew:
  tap: paiml/tap
  caveats: "Run: forjar init"
post_install: |
  echo done
"#;
        let config: DistConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.targets.len(), 2);
        assert_eq!(config.targets[0].os, "linux");
        assert_eq!(config.targets[0].libc.as_deref(), Some("gnu"));
        assert_eq!(config.targets[1].os, "darwin");
        assert!(config.targets[1].libc.is_none());
        assert_eq!(config.checksums.as_deref(), Some("SHA256SUMS"));
        assert!(config.latest_tag);
        assert!(config.homebrew.is_some());
        assert_eq!(config.homebrew.as_ref().unwrap().tap, "paiml/tap");
        assert!(config.post_install.is_some());
    }

    #[test]
    fn dist_binary_target_serde_roundtrip() {
        let target = DistBinaryTarget {
            os: "linux".into(),
            arch: "x86_64".into(),
            asset: "tool-{version}-x86_64.tar.gz".into(),
            libc: Some("musl".into()),
        };
        let json = serde_json::to_string(&target).unwrap();
        let parsed: DistBinaryTarget = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.libc.as_deref(), Some("musl"));
    }

    #[test]
    fn dist_homebrew_config_defaults() {
        let yaml = "tap: org/tap";
        let config: DistHomebrewConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(config.dependencies.is_empty());
        assert!(config.caveats.is_none());
    }

    #[test]
    fn dist_nix_config_defaults() {
        let yaml = "{}";
        let config: DistNixConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(config.inputs.is_empty());
        assert!(config.build_inputs.is_empty());
    }
}
