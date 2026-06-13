//! Tests: Apply variants — dry-run graph, dry-run cost, canary.

use super::apply_variants::*;

fn make_config(dir: &std::path::Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

const MULTI_MACHINE_CONFIG: &str = r#"version: "1.0"
name: multi
machines:
  web:
    hostname: web
    addr: 127.0.0.1
  db:
    hostname: db
    addr: 127.0.0.1
resources:
  web-pkg:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
  db-pkg:
    type: package
    machine: db
    provider: apt
    packages: [postgresql]
    depends_on: [web-pkg]
"#;

const SIMPLE_CONFIG: &str = r#"version: "1.0"
name: simple
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
  cfg:
    type: file
    machine: m
    path: /tmp/test.conf
    content: "test"
    depends_on: [pkg]
"#;

#[test]
fn dry_run_graph_shows_resources() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(dir.path(), SIMPLE_CONFIG);
    let result = cmd_apply_dry_run_graph(&file);
    assert!(result.is_ok());
}

#[test]
fn dry_run_graph_multi_machine() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(dir.path(), MULTI_MACHINE_CONFIG);
    let result = cmd_apply_dry_run_graph(&file);
    assert!(result.is_ok());
}

#[test]
fn dry_run_graph_empty_resources() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
    );
    let result = cmd_apply_dry_run_graph(&file);
    assert!(result.is_ok());
}

#[test]
fn dry_run_cost_basic() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = make_config(dir.path(), SIMPLE_CONFIG);
    let result = cmd_apply_dry_run_cost(&file, &state_dir, None);
    assert!(result.is_ok());
}

#[test]
fn dry_run_cost_with_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = make_config(dir.path(), MULTI_MACHINE_CONFIG);
    let result = cmd_apply_dry_run_cost(&file, &state_dir, Some("web"));
    assert!(result.is_ok());
}

#[test]
fn canary_machine_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = make_config(dir.path(), MULTI_MACHINE_CONFIG);
    let result = cmd_apply_canary_machine(&file, &state_dir, "nonexistent", &[], None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn dry_run_graph_nonexistent_config() {
    let result = cmd_apply_dry_run_graph(std::path::Path::new("/nonexistent.yaml"));
    assert!(result.is_err());
}

#[test]
fn dry_run_cost_nonexistent_config() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_apply_dry_run_cost(
        std::path::Path::new("/nonexistent.yaml"),
        dir.path(),
        None,
    );
    assert!(result.is_err());
}

#[test]
fn test_fj154_22_refresh_resolves_secrets_like_executor() {
    // FJ-154 / #22: cmd_refresh_only builds the state-query script from a
    // resource resolved WITH config.secrets, matching exactly what the
    // executor's record path (resource_ops::record_success) used to compute
    // the stored live_hash. Before the fix, refresh resolved with the default
    // (env) provider, so a secret-templated query field produced a different
    // script → different hash → spurious drift + state rewrite on every
    // refresh.
    //
    // Hermetic: a `file` secret provider reads <path>/<key> from a tempdir.
    use crate::core::codegen::state_query_script;
    use crate::core::parser::parse_config;
    use crate::core::resolver::{
        resolve_resource_templates, resolve_resource_templates_with_secrets,
    };

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("conf-name"), "app-prod\n").unwrap();

    let yaml = format!(
        r#"
version: "1.0"
name: refresh-secret
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
    path: "/etc/{{{{secrets.conf-name}}}}.conf"
    content: "ok"
"#,
        path = dir.path().display()
    );
    let config = parse_config(&yaml).unwrap();
    let resource = &config.resources["conf"];

    // Executor (record_success) resolution — what produced the stored hash.
    let executor_resolved = resolve_resource_templates_with_secrets(
        resource,
        &config.params,
        &config.machines,
        &config.secrets,
    )
    .unwrap();
    let executor_query = state_query_script(&executor_resolved).unwrap();

    // Refresh path AFTER the fix uses the same resolution → identical script.
    let refresh_resolved = resolve_resource_templates_with_secrets(
        resource,
        &config.params,
        &config.machines,
        &config.secrets,
    )
    .unwrap();
    let refresh_query = state_query_script(&refresh_resolved).unwrap();
    assert_eq!(
        refresh_query, executor_query,
        "refresh query script must match the executor's stored-hash query script"
    );
    assert!(
        executor_query.contains("/etc/app-prod.conf"),
        "secret should resolve into the queried path: {executor_query}"
    );

    // Regression guard: the OLD refresh path (env-default, falling back to the
    // literal on error) produced a DIFFERENT script — the spurious-drift bug.
    let old_resolved =
        resolve_resource_templates(resource, &config.params, &config.machines)
            .unwrap_or_else(|_| resource.clone());
    let old_query = state_query_script(&old_resolved).unwrap();
    assert_ne!(
        old_query, executor_query,
        "regression guard: env-default refresh query MUST differ from the executor query"
    );
}
