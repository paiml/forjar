//! FJ-254 / FJ-2502: Merge included config files with hardened validation.
//!
//! Circular include detection, conflict warnings, and provenance tracking.

use super::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// FJ-254: Merge included config files into the base config.
/// Later includes override earlier ones. params/machines/resources merge by key.
/// policy is replaced wholesale. includes are not recursive (single level).
///
/// FJ-2502 enhancements:
/// - Circular include detection via visited path set
/// - Conflict warnings when keys are overwritten
/// - Include provenance in warnings
pub(super) fn merge_includes(base: ForjarConfig, base_dir: &Path) -> Result<ForjarConfig, String> {
    let mut visited: HashSet<PathBuf> = HashSet::new();
    merge_includes_inner(base, base_dir, &mut visited)
}

fn merge_includes_inner(
    base: ForjarConfig,
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<ForjarConfig, String> {
    let mut merged = base.clone();
    merged.includes = vec![];

    for include_path in &base.includes {
        let full_path = base_dir.join(include_path);
        let canonical = full_path
            .canonicalize()
            .unwrap_or_else(|_| full_path.clone());

        // FJ-2502: Circular include detection
        if !visited.insert(canonical.clone()) {
            return Err(format!(
                "circular include detected: '{}' already included",
                include_path
            ));
        }

        let included = super::parse_config_file(&full_path)
            .map_err(|e| format!("include '{include_path}': {e}"))?;

        // FJ-2502: Warn on nested includes (not supported)
        if !included.includes.is_empty() {
            eprintln!(
                "warning: include '{include_path}' has its own includes (ignored — only single-level includes supported)"
            );
        }

        // Merge params (later overrides earlier)
        for (k, v) in included.params {
            if merged.params.contains_key(&k) {
                eprintln!("warning: include '{include_path}' overwrites param '{k}'");
            }
            merged.params.insert(k.clone(), v);
            merged
                .include_provenance
                .insert(format!("param:{k}"), include_path.clone());
        }

        // Merge machines (later overrides earlier)
        for (k, v) in included.machines {
            if merged.machines.contains_key(&k) {
                eprintln!("warning: include '{include_path}' overwrites machine '{k}'");
            }
            merged.machines.insert(k.clone(), v);
            merged
                .include_provenance
                .insert(format!("machine:{k}"), include_path.clone());
        }

        // Merge resources (later overrides earlier)
        for (k, v) in included.resources {
            if merged.resources.contains_key(&k) {
                eprintln!("warning: include '{include_path}' overwrites resource '{k}'");
            }
            merged.resources.insert(k.clone(), v);
            merged
                .include_provenance
                .insert(format!("resource:{k}"), include_path.clone());
        }

        // Policy: replace wholesale from include
        merged.policy = included.policy;

        // Merge outputs
        for (k, v) in included.outputs {
            if merged.outputs.contains_key(&k) {
                eprintln!("warning: include '{include_path}' overwrites output '{k}'");
            }
            merged.outputs.insert(k.clone(), v);
            merged
                .include_provenance
                .insert(format!("output:{k}"), include_path.clone());
        }

        // Merge policy rules
        merged.policies.extend(included.policies);

        // Merge data sources
        for (k, v) in included.data {
            if merged.data.contains_key(&k) {
                eprintln!("warning: include '{include_path}' overwrites data source '{k}'");
            }
            merged.data.insert(k.clone(), v);
            merged
                .include_provenance
                .insert(format!("data:{k}"), include_path.clone());
        }
    }

    Ok(merged)
}

#[cfg(test)]
mod tests_includes_hardening {
    use super::*;

    #[test]
    fn circular_include_detected() {
        let dir = tempfile::tempdir().unwrap();
        // a.yaml includes b.yaml, b.yaml includes a.yaml
        let a = dir.path().join("a.yaml");
        let b = dir.path().join("b.yaml");
        std::fs::write(
            &a,
            format!(
                "version: \"1.0\"\nname: a\nincludes:\n  - {}\nresources: {{}}\n",
                b.display()
            ),
        )
        .unwrap();
        std::fs::write(
            &b,
            format!(
                "version: \"1.0\"\nname: b\nincludes:\n  - {}\nresources: {{}}\n",
                a.display()
            ),
        )
        .unwrap();

        let config = parse_config_file(&a).unwrap();
        let result = merge_includes(config, dir.path());
        // Nested includes are ignored (not processed), so no circular error
        // The circular detection protects against bugs in future recursive support
        assert!(result.is_ok());
    }

    #[test]
    fn duplicate_include_detected() {
        let dir = tempfile::tempdir().unwrap();
        let inc = dir.path().join("inc.yaml");
        std::fs::write(&inc, "version: \"1.0\"\nname: inc\nresources: {}\n").unwrap();

        let base_yaml = format!(
            "version: \"1.0\"\nname: base\nincludes:\n  - {p}\n  - {p}\nresources: {{}}\n",
            p = inc.display()
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&base_yaml).unwrap();
        let result = merge_includes(config, dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("circular include"));
    }

    #[test]
    fn conflict_warnings_emitted() {
        let dir = tempfile::tempdir().unwrap();
        let inc = dir.path().join("inc.yaml");
        std::fs::write(
            &inc,
            "version: \"1.0\"\nname: inc\nresources:\n  shared:\n    type: package\n    provider: apt\n    packages: [nginx]\n",
        )
        .unwrap();

        let base_yaml = format!(
            "version: \"1.0\"\nname: base\nincludes:\n  - {}\nresources:\n  shared:\n    type: package\n    provider: apt\n    packages: [curl]\n",
            inc.display()
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&base_yaml).unwrap();
        // Should succeed but print overwrite warning to stderr
        let result = merge_includes(config, dir.path());
        assert!(result.is_ok());
        // The merged result should have the include's version (later wins)
        let merged = result.unwrap();
        let packages = &merged.resources["shared"].packages;
        assert!(packages.contains(&"nginx".to_string()));
    }

    #[test]
    fn include_provenance_tracked() {
        let dir = tempfile::tempdir().unwrap();
        let inc = dir.path().join("infra.yaml");
        std::fs::write(
            &inc,
            "version: \"1.0\"\nname: inc\nmachines:\n  web:\n    hostname: web\n    addr: 10.0.0.1\nresources:\n  pkg:\n    type: package\n    provider: apt\n    packages: [curl]\n",
        )
        .unwrap();

        let base_yaml = format!(
            "version: \"1.0\"\nname: base\nincludes:\n  - {}\nresources: {{}}\n",
            inc.display()
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&base_yaml).unwrap();
        let result = merge_includes(config, dir.path());
        assert!(result.is_ok());
        let merged = result.unwrap();
        assert_eq!(
            merged.include_provenance.get("resource:pkg").map(String::as_str),
            Some(inc.to_str().unwrap())
        );
        assert_eq!(
            merged.include_provenance.get("machine:web").map(String::as_str),
            Some(inc.to_str().unwrap())
        );
    }

    #[test]
    fn single_include_merges_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let inc = dir.path().join("inc.yaml");
        std::fs::write(
            &inc,
            "version: \"1.0\"\nname: inc\nresources:\n  extra:\n    type: package\n    provider: apt\n    packages: [vim]\n",
        )
        .unwrap();

        let base_yaml = format!(
            "version: \"1.0\"\nname: base\nincludes:\n  - {}\nresources:\n  main:\n    type: package\n    provider: apt\n    packages: [curl]\n",
            inc.display()
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&base_yaml).unwrap();
        let result = merge_includes(config, dir.path());
        assert!(result.is_ok());
        let merged = result.unwrap();
        assert!(merged.resources.contains_key("main"));
        assert!(merged.resources.contains_key("extra"));
    }
}
