//! Tests for FJ-1364: Pin resolution execution.

use super::pin_resolve::{parse_resolved_version, pin_hash, resolution_command, ResolvedPin};

#[test]
fn resolution_command_apt() {
    let cmd = resolution_command("apt", "curl").unwrap();
    assert_eq!(cmd, "apt-cache policy curl");
}

#[test]
fn resolution_command_cargo() {
    let cmd = resolution_command("cargo", "ripgrep").unwrap();
    assert_eq!(cmd, "cargo search ripgrep --limit 1");
}

#[test]
fn resolution_command_nix() {
    let cmd = resolution_command("nix", "ripgrep").unwrap();
    assert!(cmd.contains("nix eval"));
    assert!(cmd.contains("ripgrep"));
}

#[test]
fn resolution_command_uv() {
    let cmd = resolution_command("uv", "requests").unwrap();
    assert!(cmd.contains("pip index versions requests"));
}

#[test]
fn resolution_command_pip() {
    let cmd = resolution_command("pip", "flask").unwrap();
    assert!(cmd.contains("pip index versions flask"));
}

#[test]
fn resolution_command_docker() {
    let cmd = resolution_command("docker", "alpine").unwrap();
    assert!(cmd.contains("docker image inspect"));
}

#[test]
fn resolution_command_apr() {
    let cmd = resolution_command("apr", "mistral-7b").unwrap();
    assert!(cmd.contains("apr info mistral-7b"));
}

#[test]
fn resolution_command_unknown_returns_none() {
    assert!(resolution_command("unknown_provider", "pkg").is_none());
}

#[test]
fn parse_apt_candidate() {
    let output = r#"curl:
  Installed: 7.88.1-10+deb12u5
  Candidate: 7.88.1-10+deb12u6
  Version table:
     7.88.1-10+deb12u6 500"#;

    let version = parse_resolved_version("apt", output).unwrap();
    assert_eq!(version, "7.88.1-10+deb12u6");
}

#[test]
fn parse_cargo_search() {
    let output = r#"ripgrep = "14.1.0"    # Line-oriented search tool"#;
    let version = parse_resolved_version("cargo", output).unwrap();
    assert_eq!(version, "14.1.0");
}

#[test]
fn parse_nix_version() {
    let output = "1.4.2";
    let version = parse_resolved_version("nix", output).unwrap();
    assert_eq!(version, "1.4.2");
}

#[test]
fn parse_uv_versions() {
    let output = "Available versions: 2.31.0, 2.30.0, 2.29.0\nINFO: ...";
    let version = parse_resolved_version("uv", output).unwrap();
    assert_eq!(version, "2.31.0");
}

#[test]
fn parse_pip_versions() {
    let output = "Available versions: 3.0.0, 2.3.3, 2.3.2";
    let version = parse_resolved_version("pip", output).unwrap();
    assert_eq!(version, "3.0.0");
}

#[test]
fn parse_docker_digest() {
    let output = "[alpine@sha256:abc123]";
    let version = parse_resolved_version("docker", output).unwrap();
    assert_eq!(version, "[alpine@sha256:abc123]");
}

#[test]
fn parse_apr_version() {
    let output = "7b-v0.3\n";
    let version = parse_resolved_version("apr", output).unwrap();
    assert_eq!(version, "7b-v0.3");
}

#[test]
fn parse_empty_output_returns_none() {
    assert!(parse_resolved_version("apt", "").is_none());
    assert!(parse_resolved_version("cargo", "").is_none());
    assert!(parse_resolved_version("nix", "").is_none());
}

#[test]
fn parse_apt_no_candidate_line_returns_none() {
    let output = "curl:\n  Installed: (none)\n  Version table:";
    assert!(parse_resolved_version("apt", output).is_none());
}

#[test]
fn parse_unknown_provider_returns_none() {
    assert!(parse_resolved_version("unknown", "some output").is_none());
}

#[test]
fn pin_hash_deterministic() {
    let h1 = pin_hash("apt", "curl", "7.88.1");
    let h2 = pin_hash("apt", "curl", "7.88.1");
    assert_eq!(h1, h2);
    assert!(h1.starts_with("blake3:"));
}

#[test]
fn pin_hash_different_for_different_versions() {
    let h1 = pin_hash("apt", "curl", "7.88.1");
    let h2 = pin_hash("apt", "curl", "7.88.2");
    assert_ne!(h1, h2);
}

#[test]
fn pin_hash_different_for_different_providers() {
    let h1 = pin_hash("apt", "curl", "1.0");
    let h2 = pin_hash("cargo", "curl", "1.0");
    assert_ne!(h1, h2);
}

#[test]
fn pin_hash_different_for_different_names() {
    let h1 = pin_hash("apt", "curl", "1.0");
    let h2 = pin_hash("apt", "wget", "1.0");
    assert_ne!(h1, h2);
}

#[test]
fn resolved_pin_fields() {
    let pin = ResolvedPin {
        name: "curl".to_string(),
        provider: "apt".to_string(),
        version: "7.88.1".to_string(),
        hash: pin_hash("apt", "curl", "7.88.1"),
    };
    assert_eq!(pin.name, "curl");
    assert!(pin.hash.starts_with("blake3:"));
}

#[test]
fn parse_cargo_multiline() {
    let output = r#"ripgrep = "14.1.0"    # Line-oriented search tool
    ... and 42 crates more (use --limit N to see more)"#;
    let version = parse_resolved_version("cargo", output).unwrap();
    assert_eq!(version, "14.1.0");
}

#[test]
fn parse_nix_with_trailing_newline() {
    let output = "23.11\n";
    let version = parse_resolved_version("nix", output).unwrap();
    assert_eq!(version, "23.11");
}

#[test]
fn parse_apt_with_multiple_candidates() {
    let output = r#"nginx:
  Installed: (none)
  Candidate: 1.22.1-9
  Version table:
     1.22.1-9 500
        500 http://deb.debian.org/debian bookworm/main amd64 Packages"#;
    let version = parse_resolved_version("apt", output).unwrap();
    assert_eq!(version, "1.22.1-9");
}

#[test]
fn resolution_command_for_all_known_providers() {
    let providers = ["apt", "cargo", "nix", "uv", "pip", "docker", "apr"];
    for provider in providers {
        assert!(
            resolution_command(provider, "test-pkg").is_some(),
            "{provider} should have a resolution command"
        );
    }
}

#[test]
fn parse_uv_fallback_single_line() {
    // When no "Available versions:" prefix, use first line
    let output = "2.31.0";
    let version = parse_resolved_version("uv", output).unwrap();
    assert_eq!(version, "2.31.0");
}
