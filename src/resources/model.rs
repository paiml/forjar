//! FJ-240: ML model resource handler.
//!
//! Manages ML model downloads, integrity verification, and cache management.
//! Uses BLAKE3 for content-addressed drift detection on model files.

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;

/// Generate shell script to check if a model exists and matches checksum.
pub fn check_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let state = resource.state.as_deref().unwrap_or("present");
    let p = sh_squote(path);

    match state {
        "absent" => format!(
            "[ -f {p} ] && echo {} || echo {}",
            sh_squote(&format!("exists:{name}")),
            sh_squote(&format!("absent:{name}"))
        ),
        _ => {
            if let Some(ref checksum) = resource.checksum {
                let cs = sh_squote(checksum);
                format!(
                    "if [ -f {p} ]; then\n  HASH=$(b3sum {p} 2>/dev/null | cut -d' ' -f1 || echo 'NOHASH')\n  if [ \"$HASH\" = {cs} ]; then\n    echo {}\n  else\n    echo {}\n  fi\nelse\n  echo {}\nfi",
                    sh_squote(&format!("match:{name}")),
                    sh_squote(&format!("mismatch:{name}")),
                    sh_squote(&format!("missing:{name}"))
                )
            } else {
                format!(
                    "[ -f {p} ] && echo {} || echo {}",
                    sh_squote(&format!("exists:{name}")),
                    sh_squote(&format!("missing:{name}"))
                )
            }
        }
    }
}

/// Generate the download command fragment based on source type.
fn download_command(source: &str, path: &str, cache_dir: &str) -> String {
    let p = sh_squote(path);
    let s = sh_squote(source);
    let cd = sh_squote(cache_dir);
    if source.starts_with("http://") || source.starts_with("https://") {
        format!("curl -fSL -o {p} {s}\n")
    } else if source.starts_with('/') || source.starts_with("./") || source.starts_with("~/") {
        // Local path copy; in containers the source may not exist if it's a
        // placeholder path — create a stub file so dependent resources can proceed.
        // The NOTICE message embeds the source as a quoted literal too.
        let notice = sh_squote(&format!("NOTICE: source {source} not found, creating stub"));
        format!("if [ -f {s} ]; then cp {s} {p}; else echo {notice}; touch {p}; fi\n")
    } else if source.contains('/') {
        // HuggingFace repo ID — use apr pull with fallback to huggingface-cli
        format!(
            "if command -v apr >/dev/null 2>&1; then\n\
             \x20 APR_OUT=$(apr pull {s} 2>&1)\n\
             \x20 CACHED=$(echo \"$APR_OUT\" | sed 's/\\x1b\\[[0-9;]*m//g' | grep 'Path:' | head -1 | sed 's/.*Path: *//')\n\
             \x20 if [ -n \"$CACHED\" ] && [ -f \"$CACHED\" ]; then\n\
             \x20   ln -sf \"$CACHED\" {p}\n\
             \x20 else\n\
             \x20   echo \"ERROR: apr pull did not return a valid cached path\" >&2\n\
             \x20   echo \"apr output: $APR_OUT\" >&2\n\
             \x20   exit 1\n\
             \x20 fi\n\
             elif command -v huggingface-cli >/dev/null 2>&1; then\n\
             \x20 huggingface-cli download {s} --local-dir {cd}\n\
             else\n\
             \x20 echo 'ERROR: no download tool available (need apr or huggingface-cli)' >&2; exit 1\n\
             fi\n"
        )
    } else {
        format!("cp {s} {p}\n")
    }
}

/// Generate shell script to download/remove a model.
pub fn apply_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let state = resource.state.as_deref().unwrap_or("present");
    let source = resource.source.as_deref().unwrap_or("");
    let cache_dir = resource.cache_dir.as_deref().unwrap_or("~/.cache/apr");

    let p = sh_squote(path);

    match state {
        "absent" => format!(
            "set -euo pipefail\nrm -f {p}\necho {}",
            sh_squote(&format!("removed:{name}"))
        ),
        _ => {
            let mut script = String::from("set -euo pipefail\n");
            script.push_str("export PATH=\"$HOME/.cargo/bin:$PATH\"\n");
            script.push_str(&format!("mkdir -p {}\n", sh_squote(cache_dir)));

            if let Some(parent) = std::path::Path::new(path).parent() {
                if let Some(pp) = parent.to_str() {
                    if !pp.is_empty() {
                        script.push_str(&format!("mkdir -p {}\n", sh_squote(pp)));
                    }
                }
            }

            script.push_str(&download_command(source, path, cache_dir));

            if let Some(ref checksum) = resource.checksum {
                let cs = sh_squote(checksum);
                let mismatch = sh_squote(&format!("CHECKSUM MISMATCH: expected {checksum} got "));
                script.push_str(&format!(
                    "HASH=$(b3sum {p} | cut -d' ' -f1)\n\
                     if [ \"$HASH\" != {cs} ]; then\n\
                       echo {mismatch}$HASH >&2\n\
                       rm -f {p}\n\
                       exit 1\n\
                     fi\n"
                ));
            }

            if let Some(ref owner) = resource.owner {
                script.push_str(&format!("chown {} {p}\n", sh_squote(owner)));
            }

            script.push_str(&format!(
                "echo {}",
                sh_squote(&format!("downloaded:{name}"))
            ));
            script
        }
    }
}

/// Generate shell to query model state (for BLAKE3 hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let p = sh_squote(path);
    let model_label = sh_squote(&format!("model={name}"));
    format!(
        "if [ -f {p} ]; then\n  SIZE=$(stat -c%s {p} 2>/dev/null || stat -f%z {p})\n  HASH=$(b3sum {p} 2>/dev/null | cut -d' ' -f1 || echo 'NOHASH')\n  echo {model_label}\":size=$SIZE:hash=$HASH\"\nelse\n  echo {}\nfi",
        sh_squote(&format!("model=MISSING:{name}"))
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
            output_artifacts: vec![],
            completion_check: None,
            timeout: None,
            working_dir: None,
            task_mode: None,
            task_inputs: vec![],
            stages: vec![],
            cache: false,
            gpu_device: None,
            restart_delay: None,
            quality_gate: None,
            health_check: None,
            restart_policy: None,
            pre_apply: None,
            post_apply: None,
            lifecycle: None,
            store: false,
            sudo: false,
            script: None,
            gather: vec![],
            scatter: vec![],
            build_machine: None,
            repo: None,
            tag: None,
            asset_pattern: None,
            binary: None,
            install_dir: None,
            overlay_ip: None,
            overlay_iface: None,
            overlay_hosts: None,
            overlay_firewall: None,
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

    /// PMAT-042: apr pull output parsing — extract cached path from `Path:` line
    #[test]
    fn test_pmat042_apr_pull_parses_path_from_output() {
        let r = make_model_resource("llama-7b");
        let script = apply_script(&r);
        // Must capture apr pull output and grep for "Path:" to find cached file
        assert!(
            script.contains("Path:"),
            "apr pull branch must parse Path: from apr pull output"
        );
        // Must NOT use `apr cache dir` (doesn't exist)
        assert!(
            !script.contains("apr cache dir"),
            "must not call non-existent 'apr cache dir' subcommand"
        );
    }

    /// PMAT-042: apr pull must create symlink from parsed path
    #[test]
    fn test_pmat042_apr_pull_symlinks_from_parsed_path() {
        let r = make_model_resource("llama-7b");
        let script = apply_script(&r);
        // Must symlink the parsed path to the target
        assert!(
            script.contains("ln -sf"),
            "apr pull branch must symlink cached file to target path"
        );
    }

    /// PMAT-042: apr pull output may contain ANSI escape codes — must strip them
    #[test]
    fn test_pmat042_apr_pull_strips_ansi_codes() {
        let r = make_model_resource("llama-7b");
        let script = apply_script(&r);
        assert!(
            script.contains("\\x1b"),
            "must strip ANSI escape codes from apr pull output: {script}"
        );
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

    // ── FJ-154: shell-injection hardening ─────────────────────────

    #[test]
    fn fj154_model_path_injection_neutralized() {
        let mut r = make_model_resource("m");
        r.path = Some("/models/x';reboot;'".to_string());
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(script.contains("'\\''"), "{script}");
        assert!(!script.contains("rm -f '/models/x';reboot"), "{script}");
    }

    #[test]
    fn fj154_model_owner_and_cache_dir_quoted() {
        let mut r = make_model_resource("m");
        r.source = Some("https://example.com/m.gguf".to_string());
        r.owner = Some("u';id;'".to_string());
        r.cache_dir = Some("/cache".to_string());
        let script = apply_script(&r);
        assert!(script.contains("'\\''"), "{script}");
        assert!(script.contains("mkdir -p '/cache'"), "{script}");
    }

    #[test]
    fn fj154_model_checksum_quoted() {
        let mut r = make_model_resource("m");
        r.source = Some("https://example.com/m.gguf".to_string());
        r.checksum = Some("deadbeef';id;'".to_string());
        let script = apply_script(&r);
        assert!(script.contains("'\\''"), "{script}");
        assert!(!script.contains("!= 'deadbeef';id"), "{script}");
    }

    #[test]
    fn fj154_model_url_source_quoted() {
        let mut r = make_model_resource("m");
        r.source = Some("https://evil/$(id).gguf".to_string());
        let script = apply_script(&r);
        // $(id) stays literal inside the single-quoted curl argument.
        assert!(
            script.contains("curl -fSL -o '/models/llama-7b.gguf' 'https://evil/$(id).gguf'"),
            "{script}"
        );
    }

    #[test]
    fn fj154_model_benign_unchanged() {
        let r = make_model_resource("llama-7b");
        let script = apply_script(&r);
        assert!(script.contains("mkdir -p '/opt/model-cache'"));
        assert!(script.contains("chown 'noah'"));
        assert!(script.contains("downloaded:llama-7b"));
        assert!(state_query_script(&r).contains("model=llama-7b"));
    }
}
