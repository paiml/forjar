//! PMAT-081 / FJ-3600: JSON Schema for the `dist:` section.
//!
//! Mirrors `DistConfig` / `DistBinaryTarget` / `DistHomebrewConfig` /
//! `DistNixConfig` in `src/core/types/dist_config_types.rs` — keep in
//! sync when fields change. Consumed by `forjar schema` (cmd_schema).

/// Build the JSON Schema block for the top-level `dist:` property.
pub(crate) fn build_dist_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["source", "binary"],
        "description": "FJ-3600: distribution artifact generation (forjar dist)",
        "properties": {
            "source": {
                "type": "string",
                "enum": ["github_release"],
                "description": "Binary source — only github_release is supported"
            },
            "repo": { "type": "string", "description": "GitHub org/repo, e.g. paiml/forjar" },
            "binary": { "type": "string", "description": "Binary name after install" },
            "targets": { "type": "array", "items": build_target_schema() },
            "install_dir": { "type": "string", "default": "/usr/local/bin" },
            "install_dir_fallback": { "type": "string", "default": "~/.local/bin" },
            "checksums": { "type": "string", "description": "Asset name for checksum file, e.g. SHA256SUMS" },
            "checksum_algo": { "type": "string", "enum": ["sha256", "blake3"], "default": "sha256" },
            "description": { "type": "string" },
            "homepage": { "type": "string" },
            "license": { "type": "string" },
            "maintainer": { "type": "string" },
            "version_cmd": { "type": "string", "description": "Command to verify install, e.g. 'forjar --version'" },
            "latest_tag": { "type": "boolean", "default": false },
            "post_install": { "type": "string" },
            "homebrew": build_homebrew_schema(),
            "nix": build_nix_schema()
        }
    })
}

/// Schema for one `dist.targets[]` entry (DistBinaryTarget).
fn build_target_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["os", "arch", "asset"],
        "properties": {
            "os": { "type": "string", "enum": ["linux", "darwin"] },
            "arch": { "type": "string", "enum": ["x86_64", "aarch64"] },
            "asset": { "type": "string", "description": "Asset filename template with {version} placeholder" },
            "libc": { "type": "string", "enum": ["gnu", "musl"] }
        }
    })
}

/// Schema for `dist.homebrew` (DistHomebrewConfig).
fn build_homebrew_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["tap"],
        "properties": {
            "tap": { "type": "string", "description": "Homebrew tap, e.g. paiml/tap" },
            "dependencies": { "type": "array", "items": { "type": "string" } },
            "caveats": { "type": "string" }
        }
    })
}

/// Schema for `dist.nix` (DistNixConfig).
fn build_nix_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "inputs": { "type": "object", "additionalProperties": { "type": "string" } },
            "build_inputs": { "type": "array", "items": { "type": "string" } }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every DistConfig field must appear in the schema (keep-in-sync pin).
    #[test]
    fn dist_schema_mirrors_dist_config_fields() {
        let schema = build_dist_schema();
        let props = schema["properties"].as_object().unwrap();
        for field in [
            "source",
            "repo",
            "binary",
            "targets",
            "install_dir",
            "install_dir_fallback",
            "checksums",
            "checksum_algo",
            "description",
            "homepage",
            "license",
            "maintainer",
            "version_cmd",
            "latest_tag",
            "post_install",
            "homebrew",
            "nix",
        ] {
            assert!(props.contains_key(field), "dist schema missing {field}");
        }
        assert_eq!(props.len(), 17, "schema has fields DistConfig does not");
    }

    #[test]
    fn dist_schema_source_enum_is_github_release_only() {
        let schema = build_dist_schema();
        assert_eq!(
            schema["properties"]["source"]["enum"],
            serde_json::json!(["github_release"])
        );
    }

    #[test]
    fn dist_schema_requires_source_and_binary() {
        let schema = build_dist_schema();
        assert_eq!(schema["required"], serde_json::json!(["source", "binary"]));
    }

    #[test]
    fn target_schema_mirrors_dist_binary_target() {
        let schema = build_target_schema();
        assert_eq!(
            schema["required"],
            serde_json::json!(["os", "arch", "asset"])
        );
        let props = schema["properties"].as_object().unwrap();
        for field in ["os", "arch", "asset", "libc"] {
            assert!(props.contains_key(field), "target schema missing {field}");
        }
    }

    #[test]
    fn homebrew_schema_requires_tap() {
        let schema = build_homebrew_schema();
        assert_eq!(schema["required"], serde_json::json!(["tap"]));
        assert!(schema["properties"]["dependencies"].is_object());
        assert!(schema["properties"]["caveats"].is_object());
    }

    #[test]
    fn nix_schema_has_inputs_and_build_inputs() {
        let schema = build_nix_schema();
        assert!(schema["properties"]["inputs"].is_object());
        assert!(schema["properties"]["build_inputs"].is_object());
    }

    /// Snapshot: serialized schema is stable and contains key markers.
    #[test]
    fn dist_schema_serialized_contains_markers() {
        let text = serde_json::to_string_pretty(&build_dist_schema()).unwrap();
        for marker in [
            "github_release",
            "{version} placeholder",
            "SHA256SUMS",
            "paiml/tap",
            "blake3",
        ] {
            assert!(text.contains(marker), "serialized schema missing {marker}");
        }
    }
}
