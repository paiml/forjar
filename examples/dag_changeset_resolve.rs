//! FJ-046/216/003: DAG ordering, minimal changeset, and template resolution.
//!
//! Demonstrates:
//! - Topological execution order with cycle detection
//! - Parallel wave computation for concurrent execution
//! - Minimal changeset with dependency propagation
//! - Template variable resolution (params + machine refs)
//!
//! Usage: cargo run --example dag_changeset_resolve

use forjar::core::planner::minimal_changeset::{compute_minimal_changeset, verify_minimality};
use forjar::core::resolver::{
    build_execution_order, compute_parallel_waves, resolve_resource_templates, resolve_template,
};
use forjar::core::types::*;
use indexmap::IndexMap;
use std::collections::{BTreeMap, HashMap};

fn main() {
    println!("Forjar: DAG, Changeset & Template Resolution");
    println!("{}", "=".repeat(50));

    // ── FJ-216: DAG Execution Order ──
    println!("\n[FJ-216] DAG Execution Order:");
    let mut resources = IndexMap::new();
    for name in ["nginx-pkg", "nginx-conf", "nginx-svc", "app-deploy"] {
        resources.insert(
            name.to_string(),
            Resource {
                resource_type: ResourceType::File,
                ..Default::default()
            },
        );
    }
    // nginx-conf depends on nginx-pkg, nginx-svc depends on nginx-conf
    // app-deploy depends on nginx-svc
    resources.get_mut("nginx-conf").unwrap().depends_on = vec!["nginx-pkg".into()];
    resources.get_mut("nginx-svc").unwrap().depends_on = vec!["nginx-conf".into()];
    resources.get_mut("app-deploy").unwrap().depends_on = vec!["nginx-svc".into()];

    let config = ForjarConfig {
        name: "web-stack".into(),
        resources,
        ..Default::default()
    };

    let order = build_execution_order(&config).unwrap();
    println!("  Topological order: {}", order.join(" -> "));
    assert_eq!(order[0], "nginx-pkg");
    assert_eq!(order[3], "app-deploy");

    // ── Parallel Waves ──
    println!("\n[FJ-216] Parallel Waves:");
    let waves = compute_parallel_waves(&config).unwrap();
    for (i, wave) in waves.iter().enumerate() {
        println!("  Wave {}: [{}]", i, wave.join(", "));
    }
    assert_eq!(waves.len(), 4, "linear chain = 4 sequential waves");

    // Add independent resource for concurrency demo
    let mut resources2 = config.resources.clone();
    resources2.insert(
        "monitoring".into(),
        Resource {
            resource_type: ResourceType::Package,
            ..Default::default()
        },
    );
    let config2 = ForjarConfig {
        name: "web-stack-v2".into(),
        resources: resources2,
        ..Default::default()
    };
    let waves2 = compute_parallel_waves(&config2).unwrap();
    println!("\n  With independent 'monitoring':");
    for (i, wave) in waves2.iter().enumerate() {
        println!("  Wave {}: [{}]", i, wave.join(", "));
    }
    // monitoring has no deps, runs in wave 0 alongside nginx-pkg
    assert!(waves2[0].contains(&"monitoring".to_string()));

    // ── FJ-046: Minimal Changeset ──
    println!("\n[FJ-046] Minimal Changeset:");
    let resources_list = vec![
        ("nginx-pkg".into(), "web-01".into(), "blake3:pkg-v2".into()),
        (
            "nginx-conf".into(),
            "web-01".into(),
            "blake3:conf-same".into(),
        ),
        (
            "nginx-svc".into(),
            "web-01".into(),
            "blake3:svc-same".into(),
        ),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("nginx-pkg@web-01".into(), "blake3:pkg-v1".into());
    locks.insert("nginx-conf@web-01".into(), "blake3:conf-same".into());
    locks.insert("nginx-svc@web-01".into(), "blake3:svc-same".into());
    let deps = vec![
        ("nginx-conf".into(), "nginx-pkg".into()),
        ("nginx-svc".into(), "nginx-conf".into()),
    ];

    let cs = compute_minimal_changeset(&resources_list, &locks, &deps);
    println!(
        "  Total: {}, Changes: {}, Skipped: {}",
        cs.total_resources, cs.changes_needed, cs.changes_skipped
    );
    for c in &cs.candidates {
        let marker = if c.necessary { "APPLY" } else { "SKIP " };
        println!(
            "  [{marker}] {} (current={}, desired={})",
            c.resource,
            c.current_hash.as_deref().unwrap_or("NEW"),
            c.desired_hash
        );
    }
    assert_eq!(cs.changes_needed, 3, "pkg changed -> conf+svc propagated");
    assert!(verify_minimality(&cs));
    println!("  Provably minimal: {}", cs.is_provably_minimal);

    // ── FJ-003: Template Resolution ──
    println!("\n[FJ-003] Template Resolution:");
    let mut params = HashMap::new();
    params.insert("port".into(), serde_yaml_ng::Value::Number(8080.into()));
    params.insert("env".into(), serde_yaml_ng::Value::String("prod".into()));

    let mut machines = IndexMap::new();
    machines.insert(
        "web-01".into(),
        Machine {
            hostname: "web-01.example.com".into(),
            addr: "10.0.1.10".into(),
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

    let tpl = "server {{machine.web-01.hostname}} port={{params.port}} env={{params.env}}";
    let resolved = resolve_template(tpl, &params, &machines).unwrap();
    println!("  Template: {tpl}");
    println!("  Resolved: {resolved}");
    assert_eq!(resolved, "server web-01.example.com port=8080 env=prod");

    // Resource template resolution
    let resource = Resource {
        resource_type: ResourceType::File,
        path: Some("/etc/{{params.env}}/app.conf".into()),
        content: Some("listen {{params.port}}".into()),
        ..Default::default()
    };
    let resolved_r = resolve_resource_templates(&resource, &params, &machines).unwrap();
    println!(
        "  Resource path: {:?} -> {:?}",
        resource.path, resolved_r.path
    );
    println!(
        "  Resource content: {:?} -> {:?}",
        resource.content, resolved_r.content
    );
    assert_eq!(resolved_r.path.as_deref(), Some("/etc/prod/app.conf"));
    assert_eq!(resolved_r.content.as_deref(), Some("listen 8080"));

    println!("\n{}", "=".repeat(50));
    println!("All DAG/changeset/template criteria survived.");
}
