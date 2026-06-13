//! FJ-154 (#19): Planner secret-resolution idempotency tests.
//!
//! The planner must hash desired state using the SAME `SecretsConfig` the
//! executor uses to store the lock hash; otherwise a secret-bearing resource
//! replans as a spurious Update on every run (idempotency violation).

use super::*;
use std::collections::HashMap;

#[test]
fn test_fj154_19_secret_resource_plans_noop_over_freshly_applied_lock() {
    // FJ-154 / #19: A resource that templates {{secrets.X}} with a non-env
    // provider must plan NoOp over a lock the executor wrote, because the
    // planner now resolves secrets with the SAME SecretsConfig the executor
    // used. Before the fix the planner used the default (env) provider, so
    // its desired hash differed from the executor-stored hash → spurious
    // Update forever (idempotency violation).
    //
    // Hermetic: a `file` secret provider reads <path>/<key> from a tempdir;
    // no network, no real secret manager.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("db-pass"), "s3cr3t-value\n").unwrap();

    let yaml = format!(
        r#"
version: "1.0"
name: secret-noop
secrets:
  provider: file
  path: "{path}"
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  conf:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "password={{{{secrets.db-pass}}}}"
"#,
        path = dir.path().display()
    );
    let config = crate::core::parser::parse_config(&yaml).unwrap();
    assert_eq!(config.secrets.provider.as_deref(), Some("file"));

    // Reproduce exactly what the executor stores: hash of the resource
    // resolved WITH the config's secrets (resource_ops.rs record path).
    let resource = &config.resources["conf"];
    let resolved = super::resolver::resolve_resource_templates_with_secrets(
        resource,
        &config.params,
        &config.machines,
        &config.secrets,
    )
    .unwrap();
    let executor_hash = hash_desired_state(&resolved);
    // Sanity: the secret really was resolved (template is gone, value present).
    assert!(
        resolved.content.as_deref() == Some("password=s3cr3t-value"),
        "secret should resolve from file provider, got {:?}",
        resolved.content
    );

    // Document the divergence the bug exploited: the OLD planner resolved
    // with the DEFAULT (env) provider and, on error (env var unset), fell
    // back to the literal unresolved resource — exactly resolve_or_fallback's
    // old behavior. Either way the hash differs from the file-provider
    // executor hash, so the old planner replanned this resource as Update
    // forever.
    let old_resolved =
        super::resolver::resolve_resource_templates(resource, &config.params, &config.machines)
            .unwrap_or_else(|_| resource.clone());
    assert_ne!(
        hash_desired_state(&old_resolved),
        executor_hash,
        "regression guard: env-default resolution MUST differ from the file-provider \
         executor hash, otherwise this test would pass even with the bug present"
    );

    let mut resources = indexmap::IndexMap::new();
    resources.insert(
        "conf".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: executor_hash,
            details: HashMap::new(),
        },
    );
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "m1".to_string(),
        hostname: "m1".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };
    let mut locks = HashMap::new();
    locks.insert("m1".to_string(), lock);

    let order = vec!["conf".to_string()];
    let p = plan(&config, &order, &locks, None);

    assert_eq!(
        p.to_update, 0,
        "secret-bearing resource must NOT replan as Update (idempotency)"
    );
    assert_eq!(p.unchanged, 1, "secret-bearing resource must be NoOp");
    assert!(p.changes.iter().all(|c| c.action == PlanAction::NoOp));
}
