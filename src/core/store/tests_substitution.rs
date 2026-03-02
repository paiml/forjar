//! Tests for substitution protocol executor (FJ-1322).

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
mod tests {
    use crate::core::store::cache::*;
    use crate::core::store::sandbox::{SandboxConfig, SandboxLevel};
    use crate::core::store::substitution::*;
    use std::collections::BTreeMap;
    use std::path::Path;

    fn simple_cache_config() -> CacheConfig {
        CacheConfig {
            sources: vec![
                CacheSource::Ssh {
                    host: "cache.internal".to_string(),
                    user: "forjar".to_string(),
                    path: "/var/forjar/cache".to_string(),
                    port: None,
                },
                CacheSource::Local {
                    path: "/var/forjar/store".to_string(),
                },
            ],
            auto_push: true,
            max_size_mb: 0,
        }
    }

    fn full_sandbox() -> SandboxConfig {
        SandboxConfig {
            level: SandboxLevel::Full,
            memory_mb: 2048,
            cpus: 4.0,
            timeout: 600,
            bind_mounts: Vec::new(),
            env: Vec::new(),
        }
    }

    fn make_inventory(name: &str, hashes: &[&str]) -> CacheInventory {
        let entries = hashes
            .iter()
            .map(|h| {
                (
                    h.to_string(),
                    CacheEntry {
                        store_hash: h.to_string(),
                        size_bytes: 1024,
                        created_at: "2026-01-01T00:00:00Z".to_string(),
                        provider: "apt".to_string(),
                        arch: "x86_64".to_string(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        CacheInventory {
            source_name: name.to_string(),
            entries,
        }
    }

    fn ctx<'a>(
        closure_hash: &'a str,
        input_hashes: &'a [String],
        local_entries: &'a [String],
        cache_config: &'a CacheConfig,
        cache_inventories: &'a [CacheInventory],
        sandbox: Option<&'a SandboxConfig>,
        store_dir: &'a Path,
    ) -> SubstitutionContext<'a> {
        SubstitutionContext {
            closure_hash,
            input_hashes,
            local_entries,
            cache_config,
            cache_inventories,
            sandbox,
            store_dir,
        }
    }

    // ── Local hit ──────────────────────────────────────────────

    #[test]
    fn local_hit_returns_immediately() {
        let cc = simple_cache_config();
        let local = vec!["blake3:abc123".to_string()];
        let c = ctx(
            "blake3:abc123",
            &[],
            &local,
            &cc,
            &[],
            None,
            Path::new("/var/forjar/store"),
        );
        let plan = plan_substitution(&c);
        assert!(matches!(plan.outcome, SubstitutionOutcome::LocalHit { .. }));
        assert_eq!(step_count(&plan), 2); // compute + check local
    }

    #[test]
    fn local_hit_store_path_correct() {
        let cc = simple_cache_config();
        let local = vec!["blake3:abc123".to_string()];
        let c = ctx(
            "blake3:abc123",
            &[],
            &local,
            &cc,
            &[],
            None,
            Path::new("/var/forjar/store"),
        );
        let plan = plan_substitution(&c);
        if let SubstitutionOutcome::LocalHit { store_path } = &plan.outcome {
            assert!(store_path.contains("abc123"));
        } else {
            panic!("expected local hit");
        }
    }

    // ── Cache hit ──────────────────────────────────────────────

    #[test]
    fn cache_hit_from_ssh() {
        let cc = simple_cache_config();
        let inv = make_inventory("cache.internal", &["blake3:def456"]);
        let invs = [inv];
        let c = ctx(
            "blake3:def456",
            &[],
            &[],
            &cc,
            &invs,
            None,
            Path::new("/store"),
        );
        let plan = plan_substitution(&c);
        assert!(matches!(plan.outcome, SubstitutionOutcome::CacheHit { .. }));
        assert!(requires_pull(&plan));
        assert!(!requires_build(&plan));
    }

    #[test]
    fn cache_hit_has_pull_step() {
        let cc = simple_cache_config();
        let inv = make_inventory("cache.internal", &["blake3:def456"]);
        let invs = [inv];
        let c = ctx(
            "blake3:def456",
            &[],
            &[],
            &cc,
            &invs,
            None,
            Path::new("/store"),
        );
        let plan = plan_substitution(&c);
        let has_pull = plan
            .steps
            .iter()
            .any(|s| matches!(s, SubstitutionStep::PullFromCache { .. }));
        assert!(has_pull);
    }

    // ── Cache miss ─────────────────────────────────────────────

    #[test]
    fn cache_miss_requires_build() {
        let cc = simple_cache_config();
        let c = ctx(
            "blake3:missing",
            &[],
            &[],
            &cc,
            &[],
            None,
            Path::new("/store"),
        );
        let plan = plan_substitution(&c);
        assert!(requires_build(&plan));
        assert!(!requires_pull(&plan));
    }

    #[test]
    fn cache_miss_has_build_step() {
        let cc = simple_cache_config();
        let sb = full_sandbox();
        let c = ctx(
            "blake3:missing",
            &[],
            &[],
            &cc,
            &[],
            Some(&sb),
            Path::new("/store"),
        );
        let plan = plan_substitution(&c);
        let has_build = plan
            .steps
            .iter()
            .any(|s| matches!(s, SubstitutionStep::BuildFromScratch { .. }));
        assert!(has_build);
    }

    #[test]
    fn cache_miss_auto_push() {
        let cc = simple_cache_config();
        assert!(cc.auto_push);
        let c = ctx(
            "blake3:missing",
            &[],
            &[],
            &cc,
            &[],
            None,
            Path::new("/store"),
        );
        let plan = plan_substitution(&c);
        let has_push = plan
            .steps
            .iter()
            .any(|s| matches!(s, SubstitutionStep::PushToCache { .. }));
        assert!(has_push);
    }

    #[test]
    fn cache_miss_no_auto_push() {
        let cc = CacheConfig {
            auto_push: false,
            ..simple_cache_config()
        };
        let c = ctx(
            "blake3:missing",
            &[],
            &[],
            &cc,
            &[],
            None,
            Path::new("/store"),
        );
        let plan = plan_substitution(&c);
        let has_push = plan
            .steps
            .iter()
            .any(|s| matches!(s, SubstitutionStep::PushToCache { .. }));
        assert!(!has_push);
    }

    // ── step_count / requires_build / requires_pull ────────────

    #[test]
    fn local_hit_does_not_require_build_or_pull() {
        let cc = simple_cache_config();
        let local = vec!["blake3:abc".to_string()];
        let c = ctx(
            "blake3:abc",
            &[],
            &local,
            &cc,
            &[],
            None,
            Path::new("/store"),
        );
        let plan = plan_substitution(&c);
        assert!(!requires_build(&plan));
        assert!(!requires_pull(&plan));
    }

    #[test]
    fn step_count_cache_miss_with_push() {
        let cc = simple_cache_config();
        let c = ctx("blake3:miss", &[], &[], &cc, &[], None, Path::new("/store"));
        let plan = plan_substitution(&c);
        // compute + local check + ssh check + build + store + push = 6
        assert!(step_count(&plan) >= 5);
    }
}
