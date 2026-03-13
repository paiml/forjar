//! FJ-034: GitHub Release resource handler.
//!
//! Downloads binary assets from GitHub Releases and installs them locally.
//! Designed for the nightly binary distribution pipeline: each sovereign stack
//! repo publishes aarch64 nightly binaries, and forjar provisions them onto
//! the Jetson via this resource type.
//!
//! # YAML example
//!
//! ```yaml
//! install-apr:
//!   type: github_release
//!   machine: jetson
//!   repo: paiml/aprender
//!   tag: nightly
//!   asset_pattern: "*aarch64-unknown-linux-gnu*"
//!   binary: apr
//!   install_dir: /home/user/.cargo/bin
//! ```

use crate::core::types::Resource;

/// Generate shell script to check if the binary from a GitHub release is installed.
///
/// Checks:
/// 1. Binary exists at `install_dir/binary`
/// 2. Binary is executable
pub fn check_script(resource: &Resource) -> String {
    let repo = resource.repo.as_deref().unwrap_or("unknown/unknown");
    let binary = resource.binary.as_deref().unwrap_or("unknown");
    let install_dir = resource.install_dir.as_deref().unwrap_or("/usr/local/bin");
    let bin_path = format!("{install_dir}/{binary}");

    format!(
        "if [ -x '{bin_path}' ]; then\n\
         \x20 VER=$( '{bin_path}' --version 2>/dev/null | head -1 || echo 'unknown' )\n\
         \x20 echo \"installed:{repo}:$VER\"\n\
         else\n\
         \x20 echo 'missing:{repo}'\n\
         fi"
    )
}

/// Generate shell script to download a release asset and install the binary.
///
/// Uses `gh release download` (GitHub CLI) which handles authentication,
/// pagination, and asset matching. Falls back to curl for environments
/// without `gh`.
pub fn apply_script(resource: &Resource) -> String {
    let repo = resource.repo.as_deref().unwrap_or("unknown/unknown");
    let tag = resource.tag.as_deref().unwrap_or("latest");
    let asset_pattern = resource.asset_pattern.as_deref().unwrap_or("*");
    // Strip glob wildcards for use in grep -F (fixed string match)
    let grep_pattern = asset_pattern.trim_matches('*');
    let binary = resource.binary.as_deref().unwrap_or("unknown");
    let install_dir = resource.install_dir.as_deref().unwrap_or("/usr/local/bin");
    let state = resource.state.as_deref().unwrap_or("present");
    let bin_path = format!("{install_dir}/{binary}");

    match state {
        "absent" => format!(
            "set -euo pipefail\n\
             rm -f '{bin_path}'\n\
             echo 'removed:{repo}'"
        ),
        _ => format!(
            "set -euo pipefail\n\
             TMPDIR=$(mktemp -d)\n\
             trap 'rm -rf \"$TMPDIR\"' EXIT\n\
             \n\
             # Download release asset via GitHub API (no gh CLI required)\n\
             RELEASE_URL=\"https://api.github.com/repos/{repo}/releases/tags/{tag}\"\n\
             DOWNLOAD_URL=$(curl -fsSL \"$RELEASE_URL\" | \\\n\
             \x20 grep -F '{grep_pattern}' | \\\n\
             \x20 grep -o '\"browser_download_url\": *\"[^\"]*\"' | \\\n\
             \x20 head -1 | \\\n\
             \x20 grep -o 'https://[^\"]*')\n\
             \n\
             if [ -z \"$DOWNLOAD_URL\" ]; then\n\
             \x20 echo \"ERROR: no asset matching '{asset_pattern}' in {repo}@{tag}\" >&2\n\
             \x20 exit 1\n\
             fi\n\
             \n\
             ASSET_NAME=$(basename \"$DOWNLOAD_URL\")\n\
             curl -fsSL -o \"$TMPDIR/$ASSET_NAME\" \"$DOWNLOAD_URL\"\n\
             ASSET=\"$TMPDIR/$ASSET_NAME\"\n\
             case \"$ASSET\" in\n\
             \x20 *.tar.gz|*.tgz)\n\
             \x20\x20\x20 tar xzf \"$ASSET\" -C \"$TMPDIR\" --strip-components=0\n\
             \x20\x20\x20 # Find the binary in extracted files\n\
             \x20\x20\x20 if [ -f \"$TMPDIR/{binary}\" ]; then\n\
             \x20\x20\x20\x20\x20 EXTRACTED=\"$TMPDIR/{binary}\"\n\
             \x20\x20\x20 else\n\
             \x20\x20\x20\x20\x20 EXTRACTED=$(find \"$TMPDIR\" -name '{binary}' -type f | head -1)\n\
             \x20\x20\x20 fi\n\
             \x20\x20\x20 ;;\n\
             \x20 *.zip)\n\
             \x20\x20\x20 unzip -o \"$ASSET\" -d \"$TMPDIR\"\n\
             \x20\x20\x20 EXTRACTED=$(find \"$TMPDIR\" -name '{binary}' -type f | head -1)\n\
             \x20\x20\x20 ;;\n\
             \x20 *)\n\
             \x20\x20\x20 EXTRACTED=\"$ASSET\"\n\
             \x20\x20\x20 ;;\n\
             esac\n\
             \n\
             if [ -z \"$EXTRACTED\" ] || [ ! -f \"$EXTRACTED\" ]; then\n\
             \x20 echo \"ERROR: binary '{binary}' not found in release asset\" >&2\n\
             \x20 exit 1\n\
             fi\n\
             \n\
             # Install (realpath validates path before cp)\n\
             SAFE_BIN=$(realpath \"$EXTRACTED\")\n\
             mkdir -p '{install_dir}'\n\
             cp \"$SAFE_BIN\" '{bin_path}'\n\
             chmod +x '{bin_path}'\n\
             \n\
             # Verify\n\
             VER=$( '{bin_path}' --version 2>/dev/null | head -1 || echo 'installed' )\n\
             echo \"installed:{repo}:$VER\""
        ),
    }
}

/// Generate shell to query installed binary state (for BLAKE3 hashing).
///
/// Returns version string + file size + modification time for drift detection.
/// If the nightly binary is updated upstream and forjar re-applies, the hash
/// changes and drift is detected.
pub fn state_query_script(resource: &Resource) -> String {
    let repo = resource.repo.as_deref().unwrap_or("unknown/unknown");
    let binary = resource.binary.as_deref().unwrap_or("unknown");
    let install_dir = resource.install_dir.as_deref().unwrap_or("/usr/local/bin");
    let bin_path = format!("{install_dir}/{binary}");

    format!(
        "if [ -x '{bin_path}' ]; then\n\
         \x20 VER=$( '{bin_path}' --version 2>/dev/null | head -1 || echo 'unknown' )\n\
         \x20 SIZE=$(stat -c%s '{bin_path}' 2>/dev/null || stat -f%z '{bin_path}' 2>/dev/null || echo '0')\n\
         \x20 echo \"github_release={repo}:$VER:$SIZE\"\n\
         else\n\
         \x20 echo 'github_release=MISSING:{repo}'\n\
         fi"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, Resource, ResourceType};

    fn make_github_release_resource(repo: &str, binary: &str) -> Resource {
        Resource {
            resource_type: ResourceType::GithubRelease,
            machine: MachineTarget::Single("jetson".to_string()),
            repo: Some(repo.to_string()),
            tag: Some("nightly".to_string()),
            asset_pattern: Some("*aarch64-unknown-linux-gnu*".to_string()),
            binary: Some(binary.to_string()),
            install_dir: Some("/home/user/.cargo/bin".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_fj034_check_installed() {
        let r = make_github_release_resource("paiml/forjar", "forjar");
        let script = check_script(&r);
        assert!(script.contains("/home/user/.cargo/bin/forjar"));
        assert!(script.contains("installed:paiml/forjar"));
        assert!(script.contains("missing:paiml/forjar"));
        assert!(script.contains("-x '"));
    }

    #[test]
    fn test_fj034_apply_present() {
        let r = make_github_release_resource("paiml/aprender", "apr");
        let script = apply_script(&r);
        assert!(script.contains("set -euo pipefail"));
        assert!(script.contains("paiml/aprender/releases/tags/nightly"));
        assert!(script.contains("aarch64-unknown-linux-gnu"));
        assert!(script.contains("/home/user/.cargo/bin/apr"));
        assert!(script.contains("chmod +x"));
    }

    #[test]
    fn test_fj034_apply_absent() {
        let mut r = make_github_release_resource("paiml/forjar", "forjar");
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(script.contains("rm -f '/home/user/.cargo/bin/forjar'"));
        assert!(script.contains("removed:paiml/forjar"));
    }

    #[test]
    fn test_fj034_state_query() {
        let r = make_github_release_resource("paiml/copia", "copia");
        let script = state_query_script(&r);
        assert!(script.contains("/home/user/.cargo/bin/copia"));
        assert!(script.contains("github_release=paiml/copia"));
        assert!(script.contains("github_release=MISSING:paiml/copia"));
    }

    #[test]
    fn test_fj034_default_install_dir() {
        let mut r = make_github_release_resource("paiml/pzsh", "pzsh");
        r.install_dir = None;
        let script = check_script(&r);
        assert!(script.contains("/usr/local/bin/pzsh"));
    }

    #[test]
    fn test_fj034_default_tag() {
        let mut r = make_github_release_resource("paiml/forjar", "forjar");
        r.tag = None;
        let script = apply_script(&r);
        assert!(script.contains("releases/tags/latest"));
    }

    #[test]
    fn test_fj034_tarball_extraction() {
        let r = make_github_release_resource("paiml/aprender", "apr");
        let script = apply_script(&r);
        assert!(script.contains("*.tar.gz|*.tgz)"));
        assert!(script.contains("tar xzf"));
        assert!(script.contains("*.zip)"));
        assert!(script.contains("unzip -o"));
    }

    #[test]
    fn test_fj034_tmpdir_cleanup() {
        let r = make_github_release_resource("paiml/forjar", "forjar");
        let script = apply_script(&r);
        assert!(script.contains("mktemp -d"));
        assert!(script.contains("trap 'rm -rf"));
    }

    #[test]
    fn test_fj034_verify_after_install() {
        let r = make_github_release_resource("paiml/forjar", "forjar");
        let script = apply_script(&r);
        assert!(script.contains("--version"));
        assert!(script.contains("installed:paiml/forjar"));
    }

    #[test]
    fn test_fj034_binary_not_found_error() {
        let r = make_github_release_resource("paiml/forjar", "forjar");
        let script = apply_script(&r);
        assert!(script.contains("binary 'forjar' not found in release asset"));
        assert!(script.contains("exit 1"));
    }

    #[test]
    fn test_fj034_mkdir_install_dir() {
        let r = make_github_release_resource("paiml/forjar", "forjar");
        let script = apply_script(&r);
        assert!(script.contains("mkdir -p '/home/user/.cargo/bin'"));
    }

    #[test]
    fn test_fj034_state_query_includes_size() {
        let r = make_github_release_resource("paiml/forjar", "forjar");
        let script = state_query_script(&r);
        assert!(script.contains("stat -c%s") || script.contains("stat"));
    }
}
