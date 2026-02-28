//! FJ-240: ML model resource handler.
//!
//! Manages ML model downloads, integrity verification, and cache management.
//! Uses BLAKE3 for content-addressed drift detection on model files.

use crate::core::types::Resource;

/// Generate shell script to check if a model exists and matches checksum.
pub fn check_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let state = resource.state.as_deref().unwrap_or("present");

    match state {
        "absent" => format!(
            "[ -f '{}' ] && echo 'exists:{}' || echo 'absent:{}'",
            path, name, name
        ),
        _ => {
            if let Some(ref checksum) = resource.checksum {
                format!(
                    "if [ -f '{}' ]; then\n  HASH=$(b3sum '{}' 2>/dev/null | cut -d' ' -f1 || echo 'NOHASH')\n  if [ \"$HASH\" = '{}' ]; then\n    echo 'match:{}'\n  else\n    echo 'mismatch:{}'\n  fi\nelse\n  echo 'missing:{}'\nfi",
                    path, path, checksum, name, name, name
                )
            } else {
                format!(
                    "[ -f '{}' ] && echo 'exists:{}' || echo 'missing:{}'",
                    path, name, name
                )
            }
        }
    }
}

/// Generate shell script to download/remove a model.
pub fn apply_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let state = resource.state.as_deref().unwrap_or("present");
    let source = resource.source.as_deref().unwrap_or("");
    let cache_dir = resource.cache_dir.as_deref().unwrap_or("~/.cache/apr");

    match state {
        "absent" => format!(
            "set -euo pipefail\nrm -f '{}'\necho 'removed:{}'",
            path, name
        ),
        _ => {
            let mut script = String::from("set -euo pipefail\n");

            // Ensure cache directory exists
            script.push_str(&format!("mkdir -p '{}'\n", cache_dir));

            // Ensure destination directory exists
            if let Some(parent) = std::path::Path::new(path).parent() {
                if let Some(p) = parent.to_str() {
                    if !p.is_empty() {
                        script.push_str(&format!("mkdir -p '{}'\n", p));
                    }
                }
            }

            // Download model based on source type
            if source.starts_with("http://") || source.starts_with("https://") {
                script.push_str(&format!("curl -fSL -o '{}' '{}'\n", path, source));
            } else if source.starts_with('/')
                || source.starts_with("./")
                || source.starts_with("~/")
            {
                // Local path — copy
                script.push_str(&format!("cp '{}' '{}'\n", source, path));
            } else if source.contains('/') {
                // HuggingFace repo ID (e.g., "TheBloke/Llama-2-7B-GGUF")
                // Use apr pull if available, fall back to huggingface-cli
                script.push_str(&format!(
                    "if command -v apr >/dev/null 2>&1; then\n  apr pull '{}' --output '{}'\n\
                     elif command -v huggingface-cli >/dev/null 2>&1; then\n  huggingface-cli download '{}' --local-dir '{}'\n\
                     else\n  echo 'ERROR: no download tool available (need apr or huggingface-cli)' >&2; exit 1\nfi\n",
                    source, path, source, cache_dir
                ));
            } else {
                // Bare name — assume local path
                script.push_str(&format!("cp '{}' '{}'\n", source, path));
            }

            // Verify checksum if provided
            if let Some(ref checksum) = resource.checksum {
                script.push_str(&format!(
                    "HASH=$(b3sum '{}' | cut -d' ' -f1)\n\
                     if [ \"$HASH\" != '{}' ]; then\n\
                       echo 'CHECKSUM MISMATCH: expected {} got '$HASH >&2\n\
                       rm -f '{}'\n\
                       exit 1\n\
                     fi\n",
                    path, checksum, checksum, path
                ));
            }

            // Set ownership if specified
            if let Some(ref owner) = resource.owner {
                script.push_str(&format!("chown '{}' '{}'\n", owner, path));
            }

            script.push_str(&format!("echo 'downloaded:{}'", name));
            script
        }
    }
}

/// Generate shell to query model state (for BLAKE3 hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let path = resource.path.as_deref().unwrap_or("/dev/null");

    format!(
        "if [ -f '{}' ]; then\n  SIZE=$(stat -c%s '{}' 2>/dev/null || stat -f%z '{}')\n  HASH=$(b3sum '{}' 2>/dev/null | cut -d' ' -f1 || echo 'NOHASH')\n  echo \"model={}:size=$SIZE:hash=$HASH\"\nelse\n  echo 'model=MISSING:{}'\nfi",
        path, path, path, path, name, name
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};
    use std::collections::HashMap;

    fn make_model_resource(name: &str) -> Resource {
        Resource {
            resource_type: ResourceType::Model,
            machine: MachineTarget::Single("gpu-box".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: Some("/models/llama-7b.gguf".to_string()),
            content: None,
            source: Some("TheBloke/Llama-2-7B-GGUF".to_string()),
            target: None,
            owner: Some("noah".to_string()),
            group: None,
            mode: None,
            name: Some(name.to_string()),
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            resource_group: None,
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
            format: Some("gguf".to_string()),
            quantization: Some("q4_k_m".to_string()),
            checksum: None,
            cache_dir: Some("/opt/model-cache".to_string()),
            gpu_backend: None,
            driver_version: None,
            cuda_version: None,
            rocm_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
            pre_apply: None,
            post_apply: None,
        }
    }

    #[test]
    fn test_fj240_check_model_present() {
        let r = make_model_resource("llama-7b");
        let script = check_script(&r);
        assert!(script.contains("/models/llama-7b.gguf"));
        assert!(script.contains("exists:llama-7b") || script.contains("missing:llama-7b"));
    }

    #[test]
    fn test_fj240_check_model_with_checksum() {
        let mut r = make_model_resource("llama-7b");
        r.checksum = Some("abc123def456".to_string());
        let script = check_script(&r);
        assert!(script.contains("b3sum"));
        assert!(script.contains("abc123def456"));
        assert!(script.contains("match:llama-7b"));
        assert!(script.contains("mismatch:llama-7b"));
    }

    #[test]
    fn test_fj240_check_model_absent() {
        let mut r = make_model_resource("old-model");
        r.state = Some("absent".to_string());
        let script = check_script(&r);
        assert!(script.contains("absent:old-model"));
    }

    #[test]
    fn test_fj240_apply_model_download_hf() {
        let r = make_model_resource("llama-7b");
        let script = apply_script(&r);
        assert!(script.contains("set -euo pipefail"));
        assert!(script.contains("mkdir -p '/opt/model-cache'"));
        assert!(script.contains("apr pull"));
        assert!(script.contains("TheBloke/Llama-2-7B-GGUF"));
        assert!(script.contains("chown 'noah'"));
        assert!(script.contains("downloaded:llama-7b"));
    }

    #[test]
    fn test_fj240_apply_model_download_url() {
        let mut r = make_model_resource("mistral");
        r.source = Some("https://example.com/model.gguf".to_string());
        let script = apply_script(&r);
        assert!(script.contains("curl -fSL"));
        assert!(script.contains("https://example.com/model.gguf"));
    }

    #[test]
    fn test_fj240_apply_model_local_copy() {
        let mut r = make_model_resource("local-model");
        r.source = Some("/shared/models/llama.gguf".to_string());
        let script = apply_script(&r);
        assert!(script.contains("cp '/shared/models/llama.gguf'"));
    }

    #[test]
    fn test_fj240_apply_model_absent() {
        let mut r = make_model_resource("old");
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(script.contains("rm -f"));
        assert!(script.contains("removed:old"));
    }

    #[test]
    fn test_fj240_apply_model_checksum_verification() {
        let mut r = make_model_resource("verified");
        r.checksum = Some("deadbeef".to_string());
        let script = apply_script(&r);
        assert!(script.contains("b3sum"));
        assert!(script.contains("deadbeef"));
        assert!(script.contains("CHECKSUM MISMATCH"));
    }

    #[test]
    fn test_fj240_state_query() {
        let r = make_model_resource("llama-7b");
        let script = state_query_script(&r);
        assert!(script.contains("/models/llama-7b.gguf"));
        assert!(script.contains("b3sum"));
        assert!(script.contains("model=llama-7b"));
        assert!(script.contains("model=MISSING"));
    }

    #[test]
    fn test_fj240_model_yaml_parsing() {
        let yaml = r#"
version: "1.0"
name: model-test
machines:
  gpu:
    hostname: gpu
    addr: 10.0.0.1
resources:
  llama:
    type: model
    machine: gpu
    name: llama-7b
    source: "TheBloke/Llama-2-7B-GGUF"
    path: /models/llama-7b.gguf
    format: gguf
    quantization: q4_k_m
    checksum: "abc123"
    cache_dir: /opt/cache
    owner: noah
"#;
        let config: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let r = &config.resources["llama"];
        assert_eq!(r.resource_type, ResourceType::Model);
        assert_eq!(r.name.as_deref(), Some("llama-7b"));
        assert_eq!(r.format.as_deref(), Some("gguf"));
        assert_eq!(r.quantization.as_deref(), Some("q4_k_m"));
        assert_eq!(r.checksum.as_deref(), Some("abc123"));
        assert_eq!(r.cache_dir.as_deref(), Some("/opt/cache"));
    }
}
