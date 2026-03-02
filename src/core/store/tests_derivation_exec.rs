//! Tests for derivation lifecycle executor (FJ-1342–FJ-1343).

#[cfg(test)]
mod tests {
    use crate::core::store::derivation::{Derivation, DerivationInput};
    use crate::core::store::derivation_exec::*;
    use crate::core::store::sandbox::{SandboxConfig, SandboxLevel};
    use std::collections::BTreeMap;
    use std::path::Path;

    fn sample_derivation() -> Derivation {
        let mut inputs = BTreeMap::new();
        inputs.insert(
            "base".to_string(),
            DerivationInput::Store {
                store: "blake3:abc123def456".to_string(),
            },
        );

        Derivation {
            inputs,
            script: "cp -r $inputs/base/* $out/".to_string(),
            sandbox: Some(SandboxConfig {
                level: SandboxLevel::Full,
                memory_mb: 2048,
                cpus: 4.0,
                timeout: 600,
                bind_mounts: Vec::new(),
                env: Vec::new(),
            }),
            arch: "x86_64".to_string(),
            out_var: "$out".to_string(),
        }
    }

    fn resource_derivation() -> Derivation {
        let mut inputs = BTreeMap::new();
        inputs.insert(
            "base".to_string(),
            DerivationInput::Resource {
                resource: "ubuntu-base".to_string(),
            },
        );

        Derivation {
            inputs,
            script: "echo hello > $out/greeting".to_string(),
            sandbox: None,
            arch: "x86_64".to_string(),
            out_var: "$out".to_string(),
        }
    }

    fn resolved_resources() -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("ubuntu-base".to_string(), "blake3:ubuntuabc123".to_string());
        m
    }

    // ── plan_derivation ────────────────────────────────────────

    #[test]
    fn plan_store_miss_has_all_steps() {
        let plan = plan_derivation(
            &sample_derivation(),
            &BTreeMap::new(),
            &[],
            Path::new("/var/lib/forjar/store"),
        )
        .unwrap();

        assert!(!plan.store_hit);
        assert_eq!(plan.steps.len(), 10);
        assert!(plan.sandbox_plan.is_some());
    }

    #[test]
    fn plan_store_hit_skips_build() {
        let deriv = sample_derivation();
        // Pre-compute the closure hash so we can pretend it's in the store
        let input_hashes =
            crate::core::store::derivation::collect_input_hashes(&deriv, &BTreeMap::new()).unwrap();
        let closure =
            crate::core::store::derivation::derivation_closure_hash(&deriv, &input_hashes);

        let plan =
            plan_derivation(&deriv, &BTreeMap::new(), &[closure], Path::new("/store")).unwrap();

        assert!(plan.store_hit);
        assert!(plan.sandbox_plan.is_none());
        assert_eq!(skipped_steps(&plan), 7); // steps 4-10 skipped
    }

    #[test]
    fn plan_closure_hash_deterministic() {
        let p1 =
            plan_derivation(&sample_derivation(), &BTreeMap::new(), &[], Path::new("/s")).unwrap();
        let p2 =
            plan_derivation(&sample_derivation(), &BTreeMap::new(), &[], Path::new("/s")).unwrap();
        assert_eq!(p1.closure_hash, p2.closure_hash);
    }

    #[test]
    fn plan_with_resource_input() {
        let plan = plan_derivation(
            &resource_derivation(),
            &resolved_resources(),
            &[],
            Path::new("/store"),
        )
        .unwrap();

        assert!(!plan.store_hit);
        assert_eq!(plan.input_paths.len(), 1);
        assert!(plan.input_paths.contains_key("base"));
    }

    #[test]
    fn plan_fails_on_unresolved_resource() {
        let result = plan_derivation(
            &resource_derivation(),
            &BTreeMap::new(), // no resolved resources
            &[],
            Path::new("/store"),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unresolved"));
    }

    #[test]
    fn plan_fails_on_invalid_derivation() {
        let invalid = Derivation {
            inputs: BTreeMap::new(), // empty
            script: String::new(),   // empty
            sandbox: None,
            arch: String::new(),
            out_var: "$out".to_string(),
        };
        let result = plan_derivation(&invalid, &BTreeMap::new(), &[], Path::new("/s"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("validation"));
    }

    // ── simulate_derivation ────────────────────────────────────

    #[test]
    fn simulate_produces_result() {
        let result = simulate_derivation(
            &sample_derivation(),
            &BTreeMap::new(),
            &[],
            Path::new("/var/lib/forjar/store"),
        )
        .unwrap();

        assert!(!result.store_hash.is_empty());
        assert!(!result.store_path.is_empty());
        assert!(!result.closure_hash.is_empty());
        assert_eq!(result.derivation_depth, 1);
    }

    #[test]
    fn simulate_deterministic() {
        let r1 = simulate_derivation(&sample_derivation(), &BTreeMap::new(), &[], Path::new("/s"))
            .unwrap();
        let r2 = simulate_derivation(&sample_derivation(), &BTreeMap::new(), &[], Path::new("/s"))
            .unwrap();
        assert_eq!(r1.store_hash, r2.store_hash);
        assert_eq!(r1.closure_hash, r2.closure_hash);
    }

    #[test]
    fn simulate_store_hit_returns_existing() {
        let deriv = sample_derivation();
        let input_hashes =
            crate::core::store::derivation::collect_input_hashes(&deriv, &BTreeMap::new()).unwrap();
        let closure =
            crate::core::store::derivation::derivation_closure_hash(&deriv, &input_hashes);

        let result = simulate_derivation(
            &deriv,
            &BTreeMap::new(),
            &[closure.clone()],
            Path::new("/store"),
        )
        .unwrap();

        assert_eq!(result.closure_hash, closure);
        assert!(result.store_path.contains("/store/"));
    }

    // ── execute_derivation_dag ─────────────────────────────────

    #[test]
    fn dag_single_derivation() {
        let mut derivations = BTreeMap::new();
        derivations.insert("build".to_string(), sample_derivation());

        let results = execute_derivation_dag(
            &derivations,
            &["build".to_string()],
            &BTreeMap::new(),
            &[],
            Path::new("/store"),
        )
        .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results.contains_key("build"));
    }

    #[test]
    fn dag_chained_derivations() {
        let mut derivations = BTreeMap::new();

        // First derivation: store input
        derivations.insert("base".to_string(), sample_derivation());

        // Second derivation: references first as resource
        let mut chain_inputs = BTreeMap::new();
        chain_inputs.insert(
            "src".to_string(),
            DerivationInput::Resource {
                resource: "base".to_string(),
            },
        );
        derivations.insert(
            "derived".to_string(),
            Derivation {
                inputs: chain_inputs,
                script: "cp -r $inputs/src/* $out/".to_string(),
                sandbox: None,
                arch: "x86_64".to_string(),
                out_var: "$out".to_string(),
            },
        );

        let results = execute_derivation_dag(
            &derivations,
            &["base".to_string(), "derived".to_string()],
            &BTreeMap::new(),
            &[],
            Path::new("/store"),
        )
        .unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.contains_key("base"));
        assert!(results.contains_key("derived"));
    }

    #[test]
    fn dag_missing_derivation_fails() {
        let derivations = BTreeMap::new();
        let result = execute_derivation_dag(
            &derivations,
            &["nonexistent".to_string()],
            &BTreeMap::new(),
            &[],
            Path::new("/store"),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    // ── is_store_hit / skipped_steps ───────────────────────────

    #[test]
    fn is_store_hit_helper() {
        let plan =
            plan_derivation(&sample_derivation(), &BTreeMap::new(), &[], Path::new("/s")).unwrap();
        assert!(!is_store_hit(&plan));
    }

    #[test]
    fn skipped_steps_on_miss_is_zero() {
        let plan =
            plan_derivation(&sample_derivation(), &BTreeMap::new(), &[], Path::new("/s")).unwrap();
        assert_eq!(skipped_steps(&plan), 0);
    }
}
