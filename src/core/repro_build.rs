//! FJ-095: Reproducible binary builds.
//!
//! Verifies that forjar binaries are bit-for-bit reproducible from source.
//! Checks build environment, compiler flags, and generates reproducibility
//! evidence for supply chain verification.

use std::collections::HashMap;

/// Build reproducibility report.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReproReport {
    pub source_hash: String,
    pub binary_hash: String,
    pub toolchain: String,
    pub target: String,
    pub profile: String,
    pub rustflags: Vec<String>,
    pub env_vars: HashMap<String, String>,
    pub checks: Vec<ReproCheck>,
    pub reproducible: bool,
}

/// Individual reproducibility check.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReproCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

/// Required environment variables for reproducible builds.
const REPRO_ENV: &[(&str, &str)] = &[
    ("SOURCE_DATE_EPOCH", "Timestamp for deterministic builds"),
    ("CARGO_INCREMENTAL", "Must be 0 for reproducible builds"),
];

/// Required RUSTFLAGS for reproducible builds.
const REPRO_FLAGS: &[&str] = &[
    "--remap-path-prefix", // Remove absolute paths from binary
];

/// Cargo profile settings that affect reproducibility.
#[allow(dead_code)]
const REPRO_PROFILE: &[(&str, &str)] = &[
    ("panic", "abort"),     // Deterministic panic handling
    ("lto", "thin or fat"), // Link-time optimization for determinism
    ("codegen-units", "1"), // Single codegen unit for determinism
];

/// Check build environment for reproducibility.
pub fn check_environment() -> Vec<ReproCheck> {
    let mut checks = Vec::new();

    // Check SOURCE_DATE_EPOCH
    let sde = std::env::var("SOURCE_DATE_EPOCH").ok();
    checks.push(ReproCheck {
        name: "SOURCE_DATE_EPOCH".to_string(),
        passed: sde.is_some(),
        detail: match &sde {
            Some(v) => format!("Set to {v}"),
            None => "Not set — timestamps will vary between builds".to_string(),
        },
    });

    // Check CARGO_INCREMENTAL
    let ci = std::env::var("CARGO_INCREMENTAL").ok();
    let ci_ok = ci.as_deref() == Some("0");
    checks.push(ReproCheck {
        name: "CARGO_INCREMENTAL".to_string(),
        passed: ci_ok,
        detail: if ci_ok {
            "Set to 0 (good)".to_string()
        } else {
            "Must be set to 0 for reproducible builds".to_string()
        },
    });

    // Check for debug info stripping consistency
    let strip = std::env::var("CARGO_PROFILE_RELEASE_STRIP").ok();
    checks.push(ReproCheck {
        name: "STRIP_SETTING".to_string(),
        passed: strip.is_some(),
        detail: match &strip {
            Some(v) => format!("Strip set to '{v}'"),
            None => "No explicit strip setting — may affect reproducibility".to_string(),
        },
    });

    checks
}

/// Check Cargo.toml profile for reproducibility settings.
pub fn check_cargo_profile(cargo_toml: &str) -> Vec<ReproCheck> {
    let mut checks = Vec::new();

    // Check for codegen-units = 1
    let has_cgu1 = cargo_toml.contains("codegen-units") && cargo_toml.contains("1");
    checks.push(ReproCheck {
        name: "codegen-units".to_string(),
        passed: has_cgu1,
        detail: if has_cgu1 {
            "codegen-units = 1 (deterministic)".to_string()
        } else {
            "Should set codegen-units = 1 in [profile.release]".to_string()
        },
    });

    // Check for LTO
    let has_lto = cargo_toml.contains("lto");
    checks.push(ReproCheck {
        name: "lto".to_string(),
        passed: has_lto,
        detail: if has_lto {
            "LTO enabled (good for reproducibility)".to_string()
        } else {
            "Consider enabling LTO for deterministic linking".to_string()
        },
    });

    // Check for panic = abort
    let has_panic_abort = cargo_toml.contains("panic") && cargo_toml.contains("abort");
    checks.push(ReproCheck {
        name: "panic_abort".to_string(),
        passed: has_panic_abort,
        detail: if has_panic_abort {
            "panic = \"abort\" (deterministic)".to_string()
        } else {
            "Should set panic = \"abort\" in [profile.release]".to_string()
        },
    });

    checks
}

/// Compute BLAKE3 hash of binary file.
pub fn hash_binary(path: &std::path::Path) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read binary: {e}"))?;
    Ok(blake3::hash(&data).to_hex().to_string())
}

/// Compute BLAKE3 hash of source directory (sorted file list).
pub fn hash_source_dir(dir: &std::path::Path) -> Result<String, String> {
    let mut hasher = blake3::Hasher::new();
    let mut entries: Vec<_> = collect_source_files(dir)?;
    entries.sort();

    for path in &entries {
        let rel = path
            .strip_prefix(dir)
            .map_err(|e| e.to_string())?
            .to_string_lossy();
        hasher.update(rel.as_bytes());
        let content =
            std::fs::read(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        hasher.update(&content);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

fn collect_source_files(dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut files = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| format!("Cannot read dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name == "target" || name.starts_with('.') {
                continue;
            }
            files.extend(collect_source_files(&path)?);
        } else if path.extension().is_some_and(|e| e == "rs" || e == "toml") {
            files.push(path);
        }
    }
    Ok(files)
}

/// Generate full reproducibility report.
pub fn generate_report(source_hash: &str, binary_hash: &str, cargo_toml: &str) -> ReproReport {
    let mut checks = check_environment();
    checks.extend(check_cargo_profile(cargo_toml));

    let toolchain = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string();

    let target = std::env::var("TARGET").unwrap_or_else(|_| "x86_64-unknown-linux-gnu".to_string());

    let reproducible = checks.iter().all(|c| c.passed);

    ReproReport {
        source_hash: source_hash.to_string(),
        binary_hash: binary_hash.to_string(),
        toolchain,
        target,
        profile: "release".to_string(),
        rustflags: REPRO_FLAGS.iter().map(|s| s.to_string()).collect(),
        env_vars: REPRO_ENV
            .iter()
            .filter_map(|(k, _)| std::env::var(k).ok().map(|v| (k.to_string(), v)))
            .collect(),
        checks,
        reproducible,
    }
}

/// Generate CI workflow for reproducible build verification.
pub fn repro_ci_snippet() -> &'static str {
    r#"# Reproducible Build Verification
repro-verify:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Build 1
      env:
        SOURCE_DATE_EPOCH: "1700000000"
        CARGO_INCREMENTAL: "0"
      run: cargo build --release && cp target/release/forjar /tmp/build1
    - name: Clean
      run: cargo clean
    - name: Build 2
      env:
        SOURCE_DATE_EPOCH: "1700000000"
        CARGO_INCREMENTAL: "0"
      run: cargo build --release && cp target/release/forjar /tmp/build2
    - name: Compare
      run: |
        hash1=$(b3sum /tmp/build1 | cut -d' ' -f1)
        hash2=$(b3sum /tmp/build2 | cut -d' ' -f1)
        echo "Build 1: $hash1"
        echo "Build 2: $hash2"
        [ "$hash1" = "$hash2" ] || exit 1
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_environment() {
        let checks = check_environment();
        assert!(checks.len() >= 3);
        assert!(checks.iter().any(|c| c.name == "SOURCE_DATE_EPOCH"));
        assert!(checks.iter().any(|c| c.name == "CARGO_INCREMENTAL"));
    }

    #[test]
    fn test_check_cargo_profile_good() {
        let toml = r#"
[profile.release]
panic = "abort"
lto = true
codegen-units = 1
"#;
        let checks = check_cargo_profile(toml);
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_check_cargo_profile_missing() {
        let toml = "[package]\nname = \"test\"\n";
        let checks = check_cargo_profile(toml);
        assert!(checks.iter().any(|c| !c.passed));
    }

    #[test]
    fn test_generate_report() {
        let report = generate_report(
            "src123",
            "bin456",
            "[profile.release]\npanic = \"abort\"\nlto = true\ncodegen-units = 1",
        );
        assert_eq!(report.source_hash, "src123");
        assert_eq!(report.binary_hash, "bin456");
        assert_eq!(report.profile, "release");
    }

    #[test]
    fn test_report_serde() {
        let report = generate_report("a", "b", "");
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("source_hash"));
        assert!(json.contains("reproducible"));
        let parsed: ReproReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.source_hash, "a");
    }

    #[test]
    fn test_repro_ci_snippet() {
        let ci = repro_ci_snippet();
        assert!(ci.contains("SOURCE_DATE_EPOCH"));
        assert!(ci.contains("CARGO_INCREMENTAL"));
        assert!(ci.contains("b3sum"));
    }

    #[test]
    fn test_hash_binary_missing() {
        let result = hash_binary(std::path::Path::new("/nonexistent/binary"));
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_source_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        let hash = hash_source_dir(dir.path()).unwrap();
        assert_eq!(hash.len(), 64);
        // Same content = same hash
        let hash2 = hash_source_dir(dir.path()).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_repro_check_display() {
        let check = ReproCheck {
            name: "test".to_string(),
            passed: true,
            detail: "all good".to_string(),
        };
        let json = serde_json::to_string(&check).unwrap();
        assert!(json.contains("\"passed\":true"));
    }
}
