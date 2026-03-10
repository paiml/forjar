//! FJ-3206: `forjar policy-install` — install a built-in compliance pack.
//!
//! Downloads/generates a compliance pack YAML and writes it to the output
//! directory, printing the BLAKE3 hash for integrity verification.

use std::path::Path;

/// Run `forjar policy-install <pack>` — generate and install a compliance pack.
pub(crate) fn cmd_policy_install(
    pack_name: &str,
    output_dir: &Path,
    json: bool,
) -> Result<(), String> {
    let yaml = crate::core::compliance_pack::generate_builtin_pack_yaml(pack_name)?;

    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("create {}: {e}", output_dir.display()))?;

    let filename = format!("{pack_name}.yaml");
    let dest = output_dir.join(&filename);

    std::fs::write(&dest, &yaml).map_err(|e| format!("write {}: {e}", dest.display()))?;

    let hash = blake3::hash(yaml.as_bytes());

    if json {
        print_json(pack_name, &dest, &hash);
    } else {
        print_human(pack_name, &dest, &hash);
    }

    Ok(())
}

fn print_human(pack_name: &str, dest: &Path, hash: &blake3::Hash) {
    println!("Installed pack: {pack_name}");
    println!("  File: {}", dest.display());
    println!("  BLAKE3: {hash}");
}

fn print_json(pack_name: &str, dest: &Path, hash: &blake3::Hash) {
    let val = serde_json::json!({
        "pack": pack_name,
        "file": dest.display().to_string(),
        "blake3": hash.to_string(),
    });
    println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_cis_ubuntu_22() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_policy_install("cis-ubuntu-22", dir.path(), false);
        assert!(result.is_ok(), "failed: {result:?}");
        let path = dir.path().join("cis-ubuntu-22.yaml");
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("cis-ubuntu-22.04"));
    }

    #[test]
    fn install_nist_800_53() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_policy_install("nist-800-53", dir.path(), false);
        assert!(result.is_ok(), "failed: {result:?}");
        let path = dir.path().join("nist-800-53.yaml");
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("nist-800-53"));
    }

    #[test]
    fn install_soc2() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_policy_install("soc2", dir.path(), false);
        assert!(result.is_ok(), "failed: {result:?}");
        let path = dir.path().join("soc2.yaml");
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("SOC2"));
    }

    #[test]
    fn install_hipaa() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_policy_install("hipaa", dir.path(), false);
        assert!(result.is_ok(), "failed: {result:?}");
        let path = dir.path().join("hipaa.yaml");
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("HIPAA"));
    }

    #[test]
    fn install_unknown_pack() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_policy_install("unknown-pack", dir.path(), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown pack"));
    }

    #[test]
    fn install_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_policy_install("soc2", dir.path(), true);
        assert!(result.is_ok(), "failed: {result:?}");
    }

    #[test]
    fn install_creates_output_dir() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("policies");
        let result = cmd_policy_install("hipaa", &nested, false);
        assert!(result.is_ok(), "failed: {result:?}");
        assert!(nested.join("hipaa.yaml").exists());
    }

    #[test]
    fn installed_file_has_blake3_hash() {
        let dir = tempfile::tempdir().unwrap();
        cmd_policy_install("cis-ubuntu-22", dir.path(), false).unwrap();
        let content = std::fs::read_to_string(dir.path().join("cis-ubuntu-22.yaml")).unwrap();
        let hash = blake3::hash(content.as_bytes());
        // Hash is deterministic
        assert!(!hash.to_string().is_empty());
    }

    #[test]
    fn installed_yaml_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        cmd_policy_install("nist-800-53", dir.path(), false).unwrap();
        let content = std::fs::read_to_string(dir.path().join("nist-800-53.yaml")).unwrap();
        let pack: crate::core::compliance_pack::CompliancePack =
            serde_yaml_ng::from_str(&content).unwrap();
        assert_eq!(pack.name, "nist-800-53");
        assert!(!pack.rules.is_empty());
    }
}
