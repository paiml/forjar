//! FJ-034: GitHub Release resource type falsification.
//!
//! Popperian rejection criteria for:
//! - Script generation (check, apply, state_query)
//! - Config validation (missing repo/binary)
//! - Planner integration (default state, proof obligations)
//! - Absent state (removal script)
//! - Default values (tag, install_dir)
//!
//! Usage: cargo test --test falsification_github_release

#![allow(clippy::field_reassign_with_default)]

use forjar::core::parser::parse_config;
use forjar::core::types::{Resource, ResourceType};
use forjar::resources::github_release::{apply_script, check_script, state_query_script};

// ============================================================================
// Helpers
// ============================================================================

fn github_resource(repo: &str, binary: &str) -> Resource {
    let mut r = Resource::default();
    r.resource_type = ResourceType::GithubRelease;
    r.repo = Some(repo.into());
    r.binary = Some(binary.into());
    r.tag = Some("v1.0.0".into());
    r.asset_pattern = Some("*x86_64-unknown-linux-gnu*".into());
    r.install_dir = Some("/usr/local/bin".into());
    r
}

// ============================================================================
// FJ-034: check_script
// ============================================================================

#[test]
fn check_script_contains_binary_path() {
    let r = github_resource("org/tool", "mytool");
    let script = check_script(&r);
    assert!(
        script.contains("/usr/local/bin/mytool"),
        "check must reference binary at install_dir"
    );
}

#[test]
fn check_script_reports_installed_or_missing() {
    let r = github_resource("org/tool", "mytool");
    let script = check_script(&r);
    assert!(script.contains("installed:org/tool"));
    assert!(script.contains("missing:org/tool"));
}

#[test]
fn check_script_uses_executable_test() {
    let r = github_resource("org/tool", "mytool");
    let script = check_script(&r);
    assert!(
        script.contains("-x '"),
        "check must test executable permission"
    );
}

// ============================================================================
// FJ-034: apply_script — present
// ============================================================================

#[test]
fn apply_present_downloads_from_correct_repo() {
    let r = github_resource("paiml/aprender", "apr");
    let script = apply_script(&r);
    assert!(script.contains("paiml/aprender"));
    assert!(script.contains("v1.0.0"));
}

#[test]
fn apply_present_installs_binary() {
    let r = github_resource("org/tool", "mytool");
    let script = apply_script(&r);
    assert!(script.contains("chmod +x '/usr/local/bin/mytool'"));
}

#[test]
fn apply_present_handles_tarball() {
    let r = github_resource("org/tool", "mytool");
    let script = apply_script(&r);
    assert!(script.contains("tar xzf"));
    assert!(script.contains("*.tar.gz|*.tgz)"));
}

#[test]
fn apply_present_handles_zip() {
    let r = github_resource("org/tool", "mytool");
    let script = apply_script(&r);
    assert!(script.contains("unzip"));
    assert!(script.contains("*.zip)"));
}

#[test]
fn apply_present_cleans_up_tmpdir() {
    let r = github_resource("org/tool", "mytool");
    let script = apply_script(&r);
    assert!(script.contains("trap 'rm -rf"));
}

#[test]
fn apply_present_verifies_binary() {
    let r = github_resource("org/tool", "mytool");
    let script = apply_script(&r);
    assert!(script.contains("--version"));
}

// ============================================================================
// FJ-034: apply_script — absent
// ============================================================================

#[test]
fn apply_absent_removes_binary() {
    let mut r = github_resource("org/tool", "mytool");
    r.state = Some("absent".into());
    let script = apply_script(&r);
    assert!(script.contains("rm -f '/usr/local/bin/mytool'"));
    assert!(script.contains("removed:org/tool"));
}

// ============================================================================
// FJ-034: state_query_script
// ============================================================================

#[test]
fn state_query_reports_version_and_size() {
    let r = github_resource("org/tool", "mytool");
    let script = state_query_script(&r);
    assert!(script.contains("github_release=org/tool"));
    assert!(script.contains("--version"));
    assert!(script.contains("stat"));
}

#[test]
fn state_query_handles_missing_binary() {
    let r = github_resource("org/tool", "mytool");
    let script = state_query_script(&r);
    assert!(script.contains("MISSING:org/tool"));
}

// ============================================================================
// FJ-034: Default values
// ============================================================================

#[test]
fn default_install_dir_is_usr_local_bin() {
    let mut r = github_resource("org/tool", "mytool");
    r.install_dir = None;
    let script = check_script(&r);
    assert!(script.contains("/usr/local/bin/mytool"));
}

#[test]
fn default_tag_is_latest() {
    let mut r = github_resource("org/tool", "mytool");
    r.tag = None;
    let script = apply_script(&r);
    assert!(script.contains("releases/tags/latest"));
}

#[test]
fn default_asset_pattern_is_star() {
    let mut r = github_resource("org/tool", "mytool");
    r.asset_pattern = None;
    // With *, the grep_pattern is empty string after trimming
    let script = apply_script(&r);
    assert!(script.contains("curl -fsSL"));
}

// ============================================================================
// FJ-034: Config validation
// ============================================================================

#[test]
fn validate_github_release_valid() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    user: test
resources:
  tool:
    type: github_release
    machine: local
    repo: org/tool
    binary: mytool
    tag: v1.0.0
    asset_pattern: "*linux*"
    install_dir: /usr/local/bin
"#;
    let config = parse_config(yaml);
    assert!(config.is_ok(), "valid github_release config must parse");
}

#[test]
fn validate_github_release_missing_repo_uses_fallback() {
    // Resource with no repo falls back to "unknown/unknown"
    let mut r = Resource::default();
    r.resource_type = ResourceType::GithubRelease;
    r.binary = Some("mytool".into());
    let script = check_script(&r);
    assert!(
        script.contains("unknown/unknown"),
        "missing repo must fall back to unknown/unknown"
    );
}

#[test]
fn validate_github_release_missing_binary_uses_fallback() {
    // Resource with no binary falls back to "unknown"
    let mut r = Resource::default();
    r.resource_type = ResourceType::GithubRelease;
    r.repo = Some("org/tool".into());
    let script = check_script(&r);
    assert!(
        script.contains("unknown"),
        "missing binary must fall back to unknown"
    );
}

// ============================================================================
// FJ-034: Config parsing
// ============================================================================

#[test]
fn parse_github_release_config() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    user: test
resources:
  tool:
    type: github_release
    machine: local
    repo: org/tool
    binary: mytool
    tag: v1.0.0
    asset_pattern: "*linux*"
    install_dir: /opt/bin
"#;
    let config = parse_config(yaml).unwrap();
    let r = config.resources.get("tool").unwrap();
    assert_eq!(r.resource_type, ResourceType::GithubRelease);
    assert_eq!(r.repo.as_deref(), Some("org/tool"));
    assert_eq!(r.binary.as_deref(), Some("mytool"));
    assert_eq!(r.tag.as_deref(), Some("v1.0.0"));
    assert_eq!(r.asset_pattern.as_deref(), Some("*linux*"));
    assert_eq!(r.install_dir.as_deref(), Some("/opt/bin"));
}

// ============================================================================
// FJ-034: Asset pattern matching
// ============================================================================

#[test]
fn asset_pattern_included_in_download() {
    let mut r = github_resource("org/tool", "mytool");
    r.asset_pattern = Some("*aarch64-apple-darwin*".into());
    let script = apply_script(&r);
    assert!(script.contains("aarch64-apple-darwin"));
}

#[test]
fn apply_mkdir_install_dir() {
    let r = github_resource("org/tool", "mytool");
    let script = apply_script(&r);
    assert!(
        script.contains("mkdir -p '/usr/local/bin'"),
        "apply must create install_dir"
    );
}
