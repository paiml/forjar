#![allow(clippy::field_reassign_with_default)]
//! FJ-046/216/2300/005: Changeset, DAG, template resolution, codegen dispatch.
//!
//! Demonstrates:
//! - Minimal changeset with dependency propagation
//! - DAG topological ordering and parallel waves
//! - Template resolution (params, machine refs)
//! - Secret redaction
//! - Codegen dispatch for all resource types
//!
//! Usage: cargo run --example changeset_dag_template_codegen

use forjar::core::codegen::{apply_script, check_script, state_query_script};
use forjar::core::planner::minimal_changeset::{compute_minimal_changeset, verify_minimality};
use forjar::core::resolver::{
    build_execution_order, compute_parallel_waves, redact_secrets, resolve_template,
};
use forjar::core::types::*;
use std::collections::{BTreeMap, HashMap};

fn main() {
    println!("Forjar: Changeset, DAG, Template & Codegen");
    println!("{}", "=".repeat(50));

    // ── FJ-046: Minimal Changeset ──
    println!("\n[FJ-046] Minimal Changeset:");
    let resources = vec![
        ("nginx".into(), "web-01".into(), "hash-new".into()),
        ("certbot".into(), "web-01".into(), "hash-cert".into()),
        ("webapp".into(), "web-01".into(), "hash-app".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("nginx@web-01".into(), "hash-old".into());
    locks.insert("certbot@web-01".into(), "hash-cert".into());
    locks.insert("webapp@web-01".into(), "hash-app".into());
    // webapp depends on nginx
    let deps = vec![("webapp".into(), "nginx".into())];
    let cs = compute_minimal_changeset(&resources, &locks, &deps);
    println!(
        "  Total: {}, Changed: {}, Skipped: {}",
        cs.total_resources, cs.changes_needed, cs.changes_skipped
    );
    for c in &cs.candidates {
        println!("    {} → necessary: {}", c.resource, c.necessary);
    }
    assert_eq!(cs.changes_needed, 2); // nginx changed + webapp depends on it
    assert!(verify_minimality(&cs));

    // ── FJ-216: DAG Ordering ──
    println!("\n[FJ-216] DAG Execution Order:");
    let mut cfg = ForjarConfig::default();
    let nginx = Resource::default();
    let mut certbot = Resource::default();
    certbot.depends_on = vec!["nginx".into()];
    let mut webapp = Resource::default();
    webapp.depends_on = vec!["nginx".into(), "certbot".into()];
    cfg.resources.insert("webapp".into(), webapp);
    cfg.resources.insert("nginx".into(), nginx);
    cfg.resources.insert("certbot".into(), certbot);

    let order = build_execution_order(&cfg).unwrap();
    println!("  Order: {:?}", order);
    assert_eq!(order[0], "nginx");

    let waves = compute_parallel_waves(&cfg).unwrap();
    println!("  Waves: {} total", waves.len());
    for (i, wave) in waves.iter().enumerate() {
        println!("    Wave {}: {:?}", i + 1, wave);
    }
    assert!(waves.len() >= 2);

    // ── FJ-2300: Template Resolution ──
    println!("\n[FJ-2300] Template Resolution:");
    let mut params = HashMap::new();
    params.insert(
        "domain".into(),
        serde_yaml_ng::Value::String("example.com".into()),
    );
    params.insert(
        "port".into(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(443)),
    );
    let mut machines = indexmap::IndexMap::new();
    machines.insert(
        "web".into(),
        Machine {
            hostname: "web-01".into(),
            addr: "10.0.0.1".into(),
            user: "deploy".into(),
            arch: "x86_64".into(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        },
    );

    let tpl = "server {{params.domain}} on {{machine.web.addr}}:{{params.port}}";
    let resolved = resolve_template(tpl, &params, &machines).unwrap();
    println!("  Template: {tpl}");
    println!("  Resolved: {resolved}");
    assert_eq!(resolved, "server example.com on 10.0.0.1:443");

    // Secret redaction
    let secret_output = "Connected with password s3cret123 to db";
    let redacted = redact_secrets(secret_output, &["s3cret123".into()]);
    println!("  Redacted: {redacted}");
    assert!(!redacted.contains("s3cret123"));

    // ── FJ-005: Codegen Dispatch ──
    println!("\n[FJ-005] Codegen Dispatch:");
    let types = [
        ("file", ResourceType::File),
        ("package", ResourceType::Package),
        ("service", ResourceType::Service),
        ("docker", ResourceType::Docker),
    ];
    for (name, rt) in &types {
        let r = Resource {
            resource_type: rt.clone(),
            ..Default::default()
        };
        let check_ok = check_script(&r).is_ok();
        let apply_ok = apply_script(&r).is_ok();
        let query_ok = state_query_script(&r).is_ok();
        println!("  {name:10}: check={check_ok} apply={apply_ok} query={query_ok}");
        assert!(check_ok && apply_ok && query_ok);
    }
    // Recipe should error
    let recipe_r = Resource {
        resource_type: ResourceType::Recipe,
        ..Default::default()
    };
    assert!(check_script(&recipe_r).is_err());
    println!("  recipe    : correctly rejected (expand first)");

    println!("\n{}", "=".repeat(50));
    println!("All changeset/DAG/template/codegen criteria survived.");
}
