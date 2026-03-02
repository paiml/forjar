//! Tests for sandbox lifecycle executor (FJ-1316–FJ-1319).

#[cfg(test)]
mod tests {
    use crate::core::store::sandbox::{SandboxConfig, SandboxLevel};
    use crate::core::store::sandbox_exec::*;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    fn full_config() -> SandboxConfig {
        SandboxConfig {
            level: SandboxLevel::Full,
            memory_mb: 2048,
            cpus: 4.0,
            timeout: 600,
            bind_mounts: Vec::new(),
            env: Vec::new(),
        }
    }

    fn sample_inputs() -> BTreeMap<String, PathBuf> {
        let mut m = BTreeMap::new();
        m.insert("base".to_string(), PathBuf::from("/var/lib/forjar/store/abc123/content"));
        m
    }

    // ── plan_sandbox_build ─────────────────────────────────────

    #[test]
    fn plan_has_ten_steps_minimum() {
        let plan = plan_sandbox_build(
            &full_config(),
            "blake3:abcdef1234567890",
            &sample_inputs(),
            "echo hello > $out/greeting",
            Path::new("/var/lib/forjar/store"),
        );
        // 10 base steps (step 3 has one per input, step 5 for seccomp)
        assert!(plan.steps.len() >= 10);
    }

    #[test]
    fn plan_namespace_id_derived_from_hash() {
        let plan = plan_sandbox_build(
            &full_config(),
            "blake3:abcdef1234567890abcdef",
            &sample_inputs(),
            "true",
            Path::new("/var/lib/forjar/store"),
        );
        assert!(plan.namespace_id.starts_with("forjar-build-"));
        assert!(plan.namespace_id.contains("blake3:abcdef12"));
    }

    #[test]
    fn plan_overlay_has_lower_dirs() {
        let plan = plan_sandbox_build(
            &full_config(),
            "hash123",
            &sample_inputs(),
            "true",
            Path::new("/store"),
        );
        assert_eq!(plan.overlay.lower_dirs.len(), 1);
    }

    #[test]
    fn full_level_has_seccomp_rules() {
        let plan = plan_sandbox_build(
            &full_config(),
            "hash123",
            &sample_inputs(),
            "true",
            Path::new("/store"),
        );
        assert_eq!(plan.seccomp_rules.len(), 3);
        let syscalls: Vec<&str> = plan.seccomp_rules.iter().map(|r| r.syscall.as_str()).collect();
        assert!(syscalls.contains(&"connect"));
        assert!(syscalls.contains(&"mount"));
        assert!(syscalls.contains(&"ptrace"));
    }

    #[test]
    fn minimal_level_no_seccomp() {
        let config = SandboxConfig {
            level: SandboxLevel::Minimal,
            ..full_config()
        };
        let plan = plan_sandbox_build(&config, "hash", &sample_inputs(), "true", Path::new("/s"));
        assert!(plan.seccomp_rules.is_empty());
    }

    #[test]
    fn plan_cgroup_path_set() {
        let plan = plan_sandbox_build(
            &full_config(),
            "blake3:abcdef1234567890",
            &sample_inputs(),
            "true",
            Path::new("/store"),
        );
        assert!(plan.cgroup_path.contains("forjar-build-"));
    }

    // ── seccomp_rules_for_level ────────────────────────────────

    #[test]
    fn seccomp_full_denies_three() {
        let rules = seccomp_rules_for_level(SandboxLevel::Full);
        assert_eq!(rules.len(), 3);
        assert!(rules.iter().all(|r| r.action == "deny"));
    }

    #[test]
    fn seccomp_network_only_empty() {
        assert!(seccomp_rules_for_level(SandboxLevel::NetworkOnly).is_empty());
    }

    #[test]
    fn seccomp_none_empty() {
        assert!(seccomp_rules_for_level(SandboxLevel::None).is_empty());
    }

    // ── validate_plan ──────────────────────────────────────────

    #[test]
    fn valid_plan_passes_validation() {
        let plan = plan_sandbox_build(
            &full_config(),
            "hash123",
            &sample_inputs(),
            "true",
            Path::new("/store"),
        );
        assert!(validate_plan(&plan).is_empty());
    }

    #[test]
    fn empty_plan_fails_validation() {
        let plan = SandboxPlan {
            steps: Vec::new(),
            namespace_id: String::new(),
            overlay: OverlayConfig {
                lower_dirs: Vec::new(),
                upper_dir: PathBuf::new(),
                work_dir: PathBuf::new(),
                merged_dir: PathBuf::new(),
            },
            seccomp_rules: Vec::new(),
            cgroup_path: String::new(),
        };
        let errors = validate_plan(&plan);
        assert!(errors.len() >= 2); // empty steps + empty namespace
    }

    // ── simulate_sandbox_build ─────────────────────────────────

    #[test]
    fn simulate_produces_result() {
        let result = simulate_sandbox_build(
            &full_config(),
            "blake3:abc123",
            &sample_inputs(),
            "echo hello > $out/greeting",
            Path::new("/var/lib/forjar/store"),
        );
        assert!(!result.output_hash.is_empty());
        assert!(!result.store_path.is_empty());
        assert!(!result.steps_executed.is_empty());
    }

    #[test]
    fn simulate_deterministic() {
        let r1 = simulate_sandbox_build(
            &full_config(), "hash1", &sample_inputs(), "script", Path::new("/s"),
        );
        let r2 = simulate_sandbox_build(
            &full_config(), "hash1", &sample_inputs(), "script", Path::new("/s"),
        );
        assert_eq!(r1.output_hash, r2.output_hash);
    }

    #[test]
    fn simulate_different_scripts_different_hashes() {
        let r1 = simulate_sandbox_build(
            &full_config(), "hash1", &sample_inputs(), "echo a", Path::new("/s"),
        );
        let r2 = simulate_sandbox_build(
            &full_config(), "hash1", &sample_inputs(), "echo b", Path::new("/s"),
        );
        assert_ne!(r1.output_hash, r2.output_hash);
    }

    // ── plan_step_count ────────────────────────────────────────

    #[test]
    fn step_count_matches_steps() {
        let plan = plan_sandbox_build(
            &full_config(),
            "hash123",
            &sample_inputs(),
            "true",
            Path::new("/store"),
        );
        assert_eq!(plan_step_count(&plan), plan.steps.len());
    }

    // ── multi-input overlay ────────────────────────────────────

    #[test]
    fn multi_input_plan_has_bind_steps() {
        let mut inputs = sample_inputs();
        inputs.insert("extra".to_string(), PathBuf::from("/store/def456/content"));

        let plan = plan_sandbox_build(
            &full_config(),
            "hash123",
            &inputs,
            "true",
            Path::new("/store"),
        );
        let bind_steps = plan.steps.iter().filter(|s| s.step == 3).count();
        assert_eq!(bind_steps, 2);
    }
}
