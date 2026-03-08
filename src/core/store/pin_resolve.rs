//! FJ-1364: Pin resolution execution.
//!
//! Bridges lockfile types → actual version resolution via provider CLIs.
//! Generates provider-specific queries, parses version output, and
//! produces updated lock files with resolved hashes.

use super::lockfile::{LockFile, Pin, StalenessEntry};
use crate::core::types::Machine;
use crate::transport;
use crate::tripwire::hasher::composite_hash;
use std::collections::BTreeMap;

/// A resolved pin from querying a provider.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPin {
    /// Package/resource name
    pub name: String,
    /// Provider used for resolution
    pub provider: String,
    /// Resolved version string
    pub version: String,
    /// BLAKE3 hash of (provider + name + version) for determinism
    pub hash: String,
}

/// Generate the CLI command to resolve a package's current version.
///
/// Returns `None` for providers that don't support CLI version queries.
pub fn resolution_command(provider: &str, name: &str) -> Option<String> {
    match provider {
        "apt" => Some(format!("apt-cache policy {name}")),
        "cargo" => Some(format!("cargo search {name} --limit 1")),
        "nix" => Some(format!("nix eval nixpkgs#{name}.version --raw 2>/dev/null")),
        "uv" | "pip" => Some(format!("pip index versions {name} 2>/dev/null")),
        "docker" => Some(format!(
            "docker image inspect {name} --format '{{{{.RepoDigests}}}}'"
        )),
        "apr" => Some(format!("apr info {name} --format version")),
        _ => None,
    }
}

/// Parse apt-cache policy output for the Candidate version.
fn parse_apt_version(output: &str) -> Option<String> {
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("Candidate:") {
            return line
                .strip_prefix("Candidate:")
                .map(|v| v.trim().to_string());
        }
    }
    None
}

/// Parse cargo search output: 'name = "version"   # ...'.
fn parse_cargo_version(output: &str) -> Option<String> {
    let first_line = output.lines().next()?;
    let eq_pos = first_line.find('=')?;
    let after_eq = first_line[eq_pos + 1..].trim();
    let version = after_eq.trim_start_matches('"').split('"').next()?;
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

/// Parse pip/uv "Available versions:" output, falling back to the first line.
fn parse_pip_version(output: &str) -> Option<String> {
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("Available versions:") {
            let versions = line.strip_prefix("Available versions:")?;
            return versions.split(',').next().map(|v| v.trim().to_string());
        }
    }
    output.lines().next().map(|l| l.trim().to_string())
}

/// Parse a version from provider CLI output.
///
/// Each provider has a different output format; this function extracts
/// the version string from the first relevant line.
pub fn parse_resolved_version(provider: &str, stdout: &str) -> Option<String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }

    match provider {
        "apt" => parse_apt_version(trimmed),
        "cargo" => parse_cargo_version(trimmed),
        "nix" | "docker" => Some(trimmed.to_string()),
        "uv" | "pip" => parse_pip_version(trimmed),
        "apr" => Some(trimmed.lines().next()?.trim().to_string()),
        _ => None,
    }
}

/// Compute a deterministic hash for a resolved pin.
pub fn pin_hash(provider: &str, name: &str, version: &str) -> String {
    composite_hash(&[provider, name, version])
}

/// Resolve all pins by querying providers via transport.
///
/// `inputs`: `(name, provider, current_version)` tuples.
/// Returns a fully resolved `LockFile`.
pub fn resolve_all_pins(
    inputs: &[(String, String, Option<String>)],
    machine: &Machine,
    timeout_secs: Option<u64>,
) -> Result<LockFile, String> {
    let mut pins = BTreeMap::new();

    for (name, provider, _current_ver) in inputs {
        let version = resolve_single_pin(name, provider, machine, timeout_secs)?;
        let hash = pin_hash(provider, name, &version);

        pins.insert(
            name.clone(),
            Pin {
                provider: provider.clone(),
                version: Some(version),
                hash,
                git_rev: None,
                pin_type: None,
            },
        );
    }

    Ok(LockFile {
        schema: "1.0".to_string(),
        pins,
    })
}

/// Resolve a single pin by querying its provider.
fn resolve_single_pin(
    name: &str,
    provider: &str,
    machine: &Machine,
    timeout_secs: Option<u64>,
) -> Result<String, String> {
    let cmd = resolution_command(provider, name)
        .ok_or_else(|| format!("no resolution command for provider '{provider}'"))?;

    let output = transport::exec_script_timeout(machine, &cmd, timeout_secs)
        .map_err(|e| format!("resolve {name} via {provider}: {e}"))?;

    if !output.success() {
        return Err(format!(
            "resolve {name} via {provider}: exit code {}, {}",
            output.exit_code,
            output.stderr.trim()
        ));
    }

    parse_resolved_version(provider, &output.stdout)
        .ok_or_else(|| format!("cannot parse version for {name} from {provider} output"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolution_command_apt() {
        let cmd = resolution_command("apt", "curl");
        assert_eq!(cmd.unwrap(), "apt-cache policy curl");
    }

    #[test]
    fn resolution_command_cargo() {
        let cmd = resolution_command("cargo", "serde");
        assert_eq!(cmd.unwrap(), "cargo search serde --limit 1");
    }

    #[test]
    fn resolution_command_nix() {
        let cmd = resolution_command("nix", "hello");
        assert!(cmd.unwrap().contains("nix eval"));
    }

    #[test]
    fn resolution_command_pip() {
        assert!(resolution_command("pip", "requests").is_some());
        assert!(resolution_command("uv", "numpy").is_some());
    }

    #[test]
    fn resolution_command_docker() {
        let cmd = resolution_command("docker", "nginx");
        assert!(cmd.unwrap().contains("docker image inspect"));
    }

    #[test]
    fn resolution_command_apr() {
        let cmd = resolution_command("apr", "llama-3.1");
        assert!(cmd.unwrap().contains("apr info"));
    }

    #[test]
    fn resolution_command_unknown() {
        assert!(resolution_command("unknown-provider", "pkg").is_none());
    }

    #[test]
    fn parse_apt_version_candidate() {
        let output = "curl:\n  Installed: 7.88.1-10+deb12u5\n  Candidate: 7.88.1-10+deb12u7\n  Version table:";
        assert_eq!(parse_apt_version(output), Some("7.88.1-10+deb12u7".into()));
    }

    #[test]
    fn parse_apt_version_none() {
        assert_eq!(parse_apt_version("no candidate here"), None);
    }

    #[test]
    fn parse_cargo_version_normal() {
        let output = r#"serde = "1.0.215"    # A serialization framework"#;
        assert_eq!(parse_cargo_version(output), Some("1.0.215".into()));
    }

    #[test]
    fn parse_cargo_version_empty() {
        assert_eq!(parse_cargo_version(""), None);
    }

    #[test]
    fn parse_cargo_version_no_equals() {
        assert_eq!(parse_cargo_version("something without equals"), None);
    }

    #[test]
    fn parse_pip_version_available() {
        let output = "Available versions: 2.31.0, 2.30.0, 2.29.0";
        assert_eq!(parse_pip_version(output), Some("2.31.0".into()));
    }

    #[test]
    fn parse_pip_version_fallback() {
        let output = "1.2.3\nsome other line";
        assert_eq!(parse_pip_version(output), Some("1.2.3".into()));
    }

    #[test]
    fn parse_pip_version_empty() {
        assert_eq!(parse_pip_version(""), None);
    }

    #[test]
    fn parse_resolved_version_empty_input() {
        assert_eq!(parse_resolved_version("apt", ""), None);
        assert_eq!(parse_resolved_version("apt", "  \n"), None);
    }

    #[test]
    fn parse_resolved_version_nix() {
        assert_eq!(parse_resolved_version("nix", "23.11"), Some("23.11".into()));
    }

    #[test]
    fn parse_resolved_version_docker() {
        assert_eq!(
            parse_resolved_version("docker", "sha256:abc123"),
            Some("sha256:abc123".into())
        );
    }

    #[test]
    fn parse_resolved_version_apr() {
        assert_eq!(
            parse_resolved_version("apr", "3.1-Q4"),
            Some("3.1-Q4".into())
        );
    }

    #[test]
    fn parse_resolved_version_unknown() {
        assert_eq!(parse_resolved_version("brew", "1.0"), None);
    }

    #[test]
    fn pin_hash_deterministic() {
        let h1 = pin_hash("apt", "curl", "7.88.1");
        let h2 = pin_hash("apt", "curl", "7.88.1");
        assert_eq!(h1, h2);
        assert!(h1.starts_with("blake3:"));
    }

    #[test]
    fn pin_hash_differs_on_version() {
        let h1 = pin_hash("apt", "curl", "7.88.1");
        let h2 = pin_hash("apt", "curl", "7.99.0");
        assert_ne!(h1, h2);
    }

    #[test]
    fn pin_hash_differs_on_provider() {
        let h1 = pin_hash("apt", "curl", "7.88.1");
        let h2 = pin_hash("cargo", "curl", "7.88.1");
        assert_ne!(h1, h2);
    }

    #[test]
    fn resolved_pin_struct() {
        let pin = ResolvedPin {
            name: "curl".into(),
            provider: "apt".into(),
            version: "7.88.1".into(),
            hash: pin_hash("apt", "curl", "7.88.1"),
        };
        assert_eq!(pin.name, "curl");
        assert!(pin.hash.starts_with("blake3:"));
    }
}

/// Check pin staleness by re-resolving and comparing hashes.
///
/// Returns entries where the locked hash differs from the live-resolved hash.
pub fn check_pins_live(
    lockfile: &LockFile,
    machine: &Machine,
    timeout_secs: Option<u64>,
) -> Result<Vec<StalenessEntry>, String> {
    let mut stale = Vec::new();

    for (name, pin) in &lockfile.pins {
        let cmd = match resolution_command(&pin.provider, name) {
            Some(c) => c,
            None => continue, // Skip providers we can't query
        };

        let output = match transport::exec_script_timeout(machine, &cmd, timeout_secs) {
            Ok(o) if o.success() => o,
            _ => continue, // Skip unreachable providers
        };

        if let Some(version) = parse_resolved_version(&pin.provider, &output.stdout) {
            let current_hash = pin_hash(&pin.provider, name, &version);
            if current_hash != pin.hash {
                stale.push(StalenessEntry {
                    name: name.clone(),
                    locked_hash: pin.hash.clone(),
                    current_hash,
                });
            }
        }
    }

    stale.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(stale)
}
