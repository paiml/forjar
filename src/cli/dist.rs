//! FJ-3600: Distribution artifact generation.
//!
//! Generates shell installers, Homebrew formulas, cargo-binstall metadata,
//! Nix flakes, GitHub Actions, and OS package specs from `dist:` config.

use super::dist_generators::*;
use super::dist_generators_b::*;
use std::path::Path;

/// Entry point for `forjar dist`.
pub(crate) fn cmd_dist(args: &super::commands::DistArgs) -> Result<(), String> {
    let file = &args.file;
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let config: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("YAML parse error: {e}"))?;

    let dist = config
        .dist
        .as_ref()
        .ok_or_else(|| "no 'dist:' section in config — add dist: to forjar.yaml".to_string())?;

    // PMAT-081: only github_release is implemented — fail fast instead of
    // generating artifacts with broken empty-repo github.com URLs.
    super::dist_verify::validate_dist_source(dist)?;

    // PMAT-082/FJ-3607 Tier 1: --verify generates to a temp dir and
    // statically verifies instead of writing artifacts.
    if args.verify {
        return super::dist_verify::run_verify(dist, args);
    }

    let gen_all = args.all;
    let gen_installer = args.installer || gen_all;
    let gen_homebrew = args.homebrew || gen_all;
    let gen_binstall = args.binstall || gen_all;
    let gen_nix = args.nix || gen_all;
    let gen_github_action = args.github_action || gen_all;
    let gen_deb = args.deb || gen_all;
    let gen_rpm = args.rpm || gen_all;
    let output = args.output.as_deref();
    let output_dir = args.output_dir.as_deref();
    let json = args.json;

    if !gen_installer
        && !gen_homebrew
        && !gen_binstall
        && !gen_nix
        && !gen_github_action
        && !gen_deb
        && !gen_rpm
    {
        return Err(
            "specify at least one artifact: --installer, --homebrew, --binstall, --nix, --github-action, --deb, --rpm, or --all"
                .to_string(),
        );
    }

    // PMAT-080: Homebrew + Nix embed real checksums — resolve them up front
    // for the pinned --version tag (hard error instead of placeholders).
    let release = if gen_homebrew || gen_nix {
        Some(super::dist_checksums::resolve_release(
            dist,
            args.version.as_deref(),
            args.checksums_file.as_deref(),
        )?)
    } else {
        None
    };

    let out_dir = output_dir.unwrap_or(Path::new("dist"));
    let mut artifacts: Vec<GeneratedArtifact> = Vec::new();

    if gen_installer {
        let default_path = out_dir.join("install.sh");
        let path = output.unwrap_or(&default_path);
        let content = generate_installer(dist);
        write_artifact(path, &content)?;
        artifacts.push(GeneratedArtifact::new("installer", path, content.len()));
    }

    if gen_homebrew {
        let rel = release.as_ref().ok_or("internal: release not resolved")?;
        let path = out_dir.join("homebrew.rb");
        let content = generate_homebrew(dist, rel)?;
        write_artifact(&path, &content)?;
        artifacts.push(GeneratedArtifact::new("homebrew", &path, content.len()));
    }

    if gen_binstall {
        let path = out_dir.join("binstall.toml");
        let content = generate_binstall(dist);
        write_artifact(&path, &content)?;
        artifacts.push(GeneratedArtifact::new("binstall", &path, content.len()));
    }

    if gen_nix {
        let rel = release.as_ref().ok_or("internal: release not resolved")?;
        let path = out_dir.join("flake.nix");
        let content = generate_nix(dist, rel)?;
        write_artifact(&path, &content)?;
        artifacts.push(GeneratedArtifact::new("nix", &path, content.len()));
    }

    if gen_github_action {
        let path = out_dir.join("action.yml");
        let content = generate_github_action(dist);
        write_artifact(&path, &content)?;
        artifacts.push(GeneratedArtifact::new(
            "github-action",
            &path,
            content.len(),
        ));
    }

    if gen_deb {
        let dir = out_dir.join("debian");
        generate_deb(dist, &dir)?;
        artifacts.push(GeneratedArtifact::new("deb", &dir, 0));
    }

    if gen_rpm {
        let path = out_dir.join(format!("{}.spec", dist.binary));
        let content = generate_rpm(dist);
        write_artifact(&path, &content)?;
        artifacts.push(GeneratedArtifact::new("rpm", &path, content.len()));
    }

    if json {
        print_json(&artifacts);
    } else {
        print_summary(&artifacts);
    }

    Ok(())
}

struct GeneratedArtifact {
    kind: String,
    path: String,
    size: usize,
}

impl GeneratedArtifact {
    fn new(kind: &str, path: &Path, size: usize) -> Self {
        Self {
            kind: kind.to_string(),
            path: path.display().to_string(),
            size,
        }
    }
}

fn print_json(artifacts: &[GeneratedArtifact]) {
    let items: Vec<String> = artifacts
        .iter()
        .map(|a| {
            format!(
                r#"{{"kind":"{}","path":"{}","size":{}}}"#,
                a.kind, a.path, a.size
            )
        })
        .collect();
    println!(
        r#"{{"artifacts":[{}],"count":{}}}"#,
        items.join(","),
        artifacts.len()
    );
}

fn print_summary(artifacts: &[GeneratedArtifact]) {
    println!("Generated {} distribution artifact(s):", artifacts.len());
    for a in artifacts {
        if a.size > 0 {
            println!("  {} → {} ({} bytes)", a.kind, a.path, a.size);
        } else {
            println!("  {} → {}", a.kind, a.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::dist_checksums::{parse_sha256sums, ResolvedRelease};
    use crate::core::types::{DistBinaryTarget, DistConfig, DistHomebrewConfig};

    fn sample_release() -> ResolvedRelease {
        let sums = "\
1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa  forjar-1.4.3-x86_64-unknown-linux-gnu.tar.gz
2222bbbb2222bbbb2222bbbb2222bbbb2222bbbb2222bbbb2222bbbb2222bbbb  forjar-1.4.3-x86_64-unknown-linux-musl.tar.gz
3333cccc3333cccc3333cccc3333cccc3333cccc3333cccc3333cccc3333cccc  forjar-1.4.3-aarch64-apple-darwin.tar.gz
";
        ResolvedRelease {
            version: "1.4.3".into(),
            checksums: parse_sha256sums(sums),
        }
    }

    fn sample_dist() -> DistConfig {
        DistConfig {
            source: "github_release".into(),
            repo: "paiml/forjar".into(),
            binary: "forjar".into(),
            targets: vec![
                DistBinaryTarget {
                    os: "linux".into(),
                    arch: "x86_64".into(),
                    asset: "forjar-{version}-x86_64-unknown-linux-gnu.tar.gz".into(),
                    libc: Some("gnu".into()),
                },
                DistBinaryTarget {
                    os: "linux".into(),
                    arch: "x86_64".into(),
                    asset: "forjar-{version}-x86_64-unknown-linux-musl.tar.gz".into(),
                    libc: Some("musl".into()),
                },
                DistBinaryTarget {
                    os: "darwin".into(),
                    arch: "aarch64".into(),
                    asset: "forjar-{version}-aarch64-apple-darwin.tar.gz".into(),
                    libc: None,
                },
            ],
            install_dir: "/usr/local/bin".into(),
            install_dir_fallback: "~/.local/bin".into(),
            checksums: Some("SHA256SUMS".into()),
            checksum_algo: "sha256".into(),
            description: "Rust-native Infrastructure as Code".into(),
            homepage: "https://forjar.dev".into(),
            license: "MIT OR Apache-2.0".into(),
            maintainer: "Pragmatic AI Labs".into(),
            version_cmd: Some("forjar --version".into()),
            latest_tag: true,
            post_install: Some("echo done".into()),
            homebrew: Some(DistHomebrewConfig {
                tap: "paiml/tap".into(),
                dependencies: vec![],
                caveats: Some("Run: forjar init".into()),
            }),
            nix: None,
        }
    }

    #[test]
    fn installer_contains_shebang() {
        let script = generate_installer(&sample_dist());
        assert!(script.starts_with("#!/bin/sh\n"));
    }

    #[test]
    fn installer_contains_set_eu() {
        let script = generate_installer(&sample_dist());
        assert!(script.contains("set -eu"));
    }

    #[test]
    fn installer_contains_binary_name() {
        let script = generate_installer(&sample_dist());
        assert!(script.contains(r#"BINARY="forjar""#));
    }

    #[test]
    fn installer_contains_repo() {
        let script = generate_installer(&sample_dist());
        assert!(script.contains(r#"REPO="paiml/forjar""#));
    }

    #[test]
    fn installer_contains_detect_os() {
        let script = generate_installer(&sample_dist());
        assert!(script.contains("detect_os()"));
    }

    #[test]
    fn installer_contains_detect_arch() {
        let script = generate_installer(&sample_dist());
        assert!(script.contains("detect_arch()"));
    }

    #[test]
    fn installer_contains_checksum_verify() {
        let script = generate_installer(&sample_dist());
        assert!(script.contains("verify_checksum"));
        assert!(script.contains("SHA256SUMS"));
    }

    #[test]
    fn installer_no_checksum_when_none() {
        let mut dist = sample_dist();
        dist.checksums = None;
        let script = generate_installer(&dist);
        assert!(script.contains("no checksums configured"));
    }

    #[test]
    fn installer_contains_version_verify() {
        let script = generate_installer(&sample_dist());
        // Anchored to the just-installed binary, not PATH resolution.
        assert!(script.contains(r#""$DEST/$BINARY" --version"#));
    }

    #[test]
    fn installer_contains_asset_cases() {
        let script = generate_installer(&sample_dist());
        assert!(script.contains("linux/x86_64)"));
        assert!(script.contains("darwin/aarch64)"));
    }

    #[test]
    fn installer_contains_fallback_dir() {
        let script = generate_installer(&sample_dist());
        // Quoted "~" never tilde-expands; the generator emits $HOME.
        assert!(script.contains("$HOME/.local/bin"));
    }

    #[test]
    fn installer_contains_post_install() {
        let script = generate_installer(&sample_dist());
        assert!(script.contains("post_install"));
        assert!(script.contains("echo done"));
    }

    #[test]
    fn homebrew_contains_class_name() {
        let formula = generate_homebrew(&sample_dist(), &sample_release()).unwrap();
        assert!(formula.contains("class Forjar < Formula"));
    }

    #[test]
    fn homebrew_contains_description() {
        let formula = generate_homebrew(&sample_dist(), &sample_release()).unwrap();
        assert!(formula.contains("Rust-native Infrastructure as Code"));
    }

    #[test]
    fn homebrew_skips_musl_targets() {
        let formula = generate_homebrew(&sample_dist(), &sample_release()).unwrap();
        assert!(!formula.contains("musl"));
    }

    #[test]
    fn homebrew_contains_caveats() {
        let formula = generate_homebrew(&sample_dist(), &sample_release()).unwrap();
        assert!(formula.contains("forjar init"));
    }

    #[test]
    fn homebrew_contains_real_version_and_checksums() {
        let formula = generate_homebrew(&sample_dist(), &sample_release()).unwrap();
        assert!(formula.contains(r#"version "1.4.3""#));
        assert!(
            formula.contains("1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa1111aaaa")
        );
        assert!(!formula.contains("PLACEHOLDER"));
    }

    #[test]
    fn homebrew_missing_checksum_is_hard_error() {
        let mut release = sample_release();
        release.checksums.clear();
        let err = generate_homebrew(&sample_dist(), &release).unwrap_err();
        assert!(err.contains("forjar-1.4.3-x86_64-unknown-linux-gnu.tar.gz"));
        assert!(err.contains("--checksums-file"));
    }

    #[test]
    fn binstall_contains_pkg_url() {
        let toml = generate_binstall(&sample_dist());
        assert!(toml.contains("[package.metadata.binstall]"));
        assert!(toml.contains("pkg-url"));
    }

    #[test]
    fn nix_contains_description() {
        let flake = generate_nix(&sample_dist(), &sample_release()).unwrap();
        assert!(flake.contains("Rust-native Infrastructure as Code"));
    }

    #[test]
    fn nix_skips_musl() {
        let flake = generate_nix(&sample_dist(), &sample_release()).unwrap();
        assert!(!flake.contains("musl"));
    }

    #[test]
    fn nix_contains_real_version_and_checksums() {
        let flake = generate_nix(&sample_dist(), &sample_release()).unwrap();
        assert!(flake.contains(r#"version = "1.4.3";"#));
        assert!(flake.contains("3333cccc3333cccc3333cccc3333cccc3333cccc3333cccc3333cccc3333cccc"));
        assert!(!flake.contains("PLACEHOLDER"));
    }

    #[test]
    fn cmd_dist_homebrew_without_version_errors() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            "version: \"1.0\"\nname: t\nmachines:\n  local:\n    hostname: l\n    addr: localhost\n    user: root\nresources: {}\ndist:\n  source: github_release\n  repo: acme/tool\n  binary: mytool\n  targets:\n    - os: linux\n      arch: x86_64\n      asset: \"mytool-{version}-x86_64-unknown-linux-gnu.tar.gz\"\n  install_dir: /usr/local/bin\n",
        )
        .unwrap();
        let args = super::super::commands::DistArgs {
            file: config,
            installer: false,
            homebrew: true,
            binstall: false,
            nix: false,
            github_action: false,
            deb: false,
            rpm: false,
            all: false,
            verify: false,
            version: None,
            checksums_file: None,
            output: None,
            output_dir: Some(dir.path().join("out")),
            json: false,
        };
        let err = cmd_dist(&args).unwrap_err();
        assert!(err.contains("--version"), "got: {err}");
    }

    #[test]
    fn github_action_contains_name() {
        let action = generate_github_action(&sample_dist());
        assert!(action.contains("name: Setup forjar"));
    }

    #[test]
    fn github_action_only_linux_targets() {
        let action = generate_github_action(&sample_dist());
        // Should have linux target, not darwin
        assert!(action.contains("x86_64-unknown-linux-gnu"));
        assert!(!action.contains("darwin"));
    }

    #[test]
    fn rpm_contains_name() {
        let spec = generate_rpm(&sample_dist());
        assert!(spec.contains("Name:    forjar"));
    }

    #[test]
    fn rpm_contains_license() {
        let spec = generate_rpm(&sample_dist());
        assert!(spec.contains("MIT OR Apache-2.0"));
    }

    #[test]
    fn to_class_name_simple() {
        assert_eq!(to_class_name("forjar"), "Forjar");
    }

    #[test]
    fn to_class_name_hyphenated() {
        assert_eq!(to_class_name("my-tool"), "MyTool");
    }

    #[test]
    fn to_rust_triple_linux_gnu() {
        let t = DistBinaryTarget {
            os: "linux".into(),
            arch: "x86_64".into(),
            asset: "test".into(),
            libc: Some("gnu".into()),
        };
        assert_eq!(to_rust_triple(&t), "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn to_rust_triple_darwin() {
        let t = DistBinaryTarget {
            os: "darwin".into(),
            arch: "aarch64".into(),
            asset: "test".into(),
            libc: None,
        };
        assert_eq!(to_rust_triple(&t), "aarch64-apple-darwin");
    }
}
