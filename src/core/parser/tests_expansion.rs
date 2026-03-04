//! for_each/count expansion tests (FJ-203, FJ-204).

use super::*;

// ================================================================
// FJ-204: count: expansion tests
// ================================================================

#[test]
fn test_fj204_count_expands_resources() {
    let yaml = r#"
version: "1.0"
name: test-count
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  data-dir:
    type: file
    machine: m1
    state: directory
    path: "/data/shard-{{index}}"
    count: 3
"#;
    let mut config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.is_empty(), "validation errors: {errors:?}");
    expand_resources(&mut config);

    assert_eq!(config.resources.len(), 3);
    assert!(config.resources.contains_key("data-dir-0"));
    assert!(config.resources.contains_key("data-dir-1"));
    assert!(config.resources.contains_key("data-dir-2"));

    assert_eq!(
        config.resources["data-dir-0"].path.as_deref(),
        Some("/data/shard-0")
    );
    assert_eq!(
        config.resources["data-dir-1"].path.as_deref(),
        Some("/data/shard-1")
    );
    assert_eq!(
        config.resources["data-dir-2"].path.as_deref(),
        Some("/data/shard-2")
    );
}

#[test]
fn test_fj204_count_one_produces_single_resource() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  single:
    type: file
    machine: m1
    path: "/data/node-{{index}}"
    count: 1
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_resources(&mut config);
    assert_eq!(config.resources.len(), 1);
    assert!(config.resources.contains_key("single-0"));
    assert_eq!(
        config.resources["single-0"].path.as_deref(),
        Some("/data/node-0")
    );
}

#[test]
fn test_fj204_count_zero_rejected() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  bad:
    type: file
    machine: m1
    path: "/tmp/bad"
    count: 0
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.message.contains("count: 0")));
}

#[test]
fn test_fj204_count_replaces_in_content() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: "/etc/node-{{index}}.conf"
    content: "node_id={{index}}"
    count: 2
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_resources(&mut config);
    assert_eq!(
        config.resources["cfg-0"].content.as_deref(),
        Some("node_id=0")
    );
    assert_eq!(
        config.resources["cfg-1"].content.as_deref(),
        Some("node_id=1")
    );
}

#[test]
fn test_fj204_count_replaces_in_packages() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: ["tool-{{index}}"]
    count: 2
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_resources(&mut config);
    assert_eq!(config.resources["pkg-0"].packages, vec!["tool-0"]);
    assert_eq!(config.resources["pkg-1"].packages, vec!["tool-1"]);
}

#[test]
fn test_fj204_count_clears_count_field() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  r:
    type: file
    machine: m1
    path: "/tmp/{{index}}"
    count: 2
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_resources(&mut config);
    assert!(config.resources["r-0"].count.is_none());
    assert!(config.resources["r-1"].count.is_none());
}

// ================================================================
// FJ-203: for_each: expansion tests
// ================================================================

#[test]
fn test_fj203_for_each_expands_resources() {
    let yaml = r#"
version: "1.0"
name: test-foreach
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  user-home:
    type: file
    machine: m1
    state: directory
    path: "/home/{{item}}"
    owner: "{{item}}"
    for_each: [alice, bob, charlie]
"#;
    let mut config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.is_empty(), "validation errors: {errors:?}");
    expand_resources(&mut config);

    assert_eq!(config.resources.len(), 3);
    assert!(config.resources.contains_key("user-home-alice"));
    assert!(config.resources.contains_key("user-home-bob"));
    assert!(config.resources.contains_key("user-home-charlie"));

    assert_eq!(
        config.resources["user-home-alice"].path.as_deref(),
        Some("/home/alice")
    );
    assert_eq!(
        config.resources["user-home-alice"].owner.as_deref(),
        Some("alice")
    );
    assert_eq!(
        config.resources["user-home-charlie"].path.as_deref(),
        Some("/home/charlie")
    );
}

#[test]
fn test_fj203_for_each_replaces_in_content() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  vhost:
    type: file
    machine: m1
    path: "/etc/nginx/sites/{{item}}.conf"
    content: "server_name {{item}}.example.com;"
    for_each: [api, web]
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_resources(&mut config);

    assert_eq!(
        config.resources["vhost-api"].path.as_deref(),
        Some("/etc/nginx/sites/api.conf")
    );
    assert_eq!(
        config.resources["vhost-api"].content.as_deref(),
        Some("server_name api.example.com;")
    );
    assert_eq!(
        config.resources["vhost-web"].content.as_deref(),
        Some("server_name web.example.com;")
    );
}

#[test]
fn test_fj203_for_each_empty_rejected() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  bad:
    type: file
    machine: m1
    path: "/tmp/bad"
    for_each: []
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.message.contains("empty for_each")));
}

#[test]
fn test_fj203_for_each_clears_field() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  r:
    type: file
    machine: m1
    path: "/tmp/{{item}}"
    for_each: [x, y]
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_resources(&mut config);
    assert!(config.resources["r-x"].for_each.is_none());
    assert!(config.resources["r-y"].for_each.is_none());
}

#[test]
fn test_fj203_fj204_count_and_for_each_rejected() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  bad:
    type: file
    machine: m1
    path: "/tmp/bad"
    count: 3
    for_each: [a, b]
"#;
    let config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("cannot have both")));
}

#[test]
fn test_fj204_count_preserves_non_expanded() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  base:
    type: file
    machine: m1
    path: "/etc/base.conf"
    content: "base config"
  shards:
    type: file
    machine: m1
    path: "/data/shard-{{index}}"
    count: 2
    depends_on: [base]
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_resources(&mut config);

    assert_eq!(config.resources.len(), 3);
    assert!(config.resources.contains_key("base"));
    assert!(config.resources.contains_key("shards-0"));
    assert!(config.resources.contains_key("shards-1"));
    assert_eq!(config.resources["shards-0"].depends_on, vec!["base"]);
}

#[test]
fn test_fj203_for_each_preserves_order() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  r:
    type: file
    machine: m1
    path: "/tmp/{{item}}"
    for_each: [z, a, m]
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_resources(&mut config);

    let keys: Vec<&String> = config.resources.keys().collect();
    assert_eq!(keys, vec!["r-z", "r-a", "r-m"]);
}

#[test]
fn test_fj203_for_each_replaces_in_name_and_port() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  svc:
    type: service
    machine: m1
    name: "app-{{item}}"
    for_each: [web, api]
  fw:
    type: network
    machine: m1
    port: "{{item}}"
    action: allow
    for_each: ["80", "443"]
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_resources(&mut config);

    assert_eq!(config.resources["svc-web"].name.as_deref(), Some("app-web"));
    assert_eq!(config.resources["svc-api"].name.as_deref(), Some("app-api"));
    assert_eq!(config.resources["fw-80"].port.as_deref(), Some("80"));
    assert_eq!(config.resources["fw-443"].port.as_deref(), Some("443"));
}

#[test]
fn test_fj204_count_replaces_in_source_and_target() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  link:
    type: file
    machine: m1
    state: symlink
    path: "/opt/app-{{index}}"
    source: "/src/app-{{index}}/bin"
    target: "/usr/local/bin/app-{{index}}"
    count: 2
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_resources(&mut config);

    assert_eq!(
        config.resources["link-0"].source.as_deref(),
        Some("/src/app-0/bin")
    );
    assert_eq!(
        config.resources["link-0"].target.as_deref(),
        Some("/usr/local/bin/app-0")
    );
    assert_eq!(
        config.resources["link-1"].source.as_deref(),
        Some("/src/app-1/bin")
    );
}

#[test]
fn test_fj204_dep_rewrite_to_last_expanded() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  shards:
    type: file
    machine: m1
    path: "/data/shard-{{index}}"
    count: 3
  reader:
    type: file
    machine: m1
    path: "/etc/reader.conf"
    depends_on: [shards]
"#;
    let mut config = parse_config(yaml).unwrap();
    let errors = validate_config(&config);
    assert!(errors.is_empty(), "validation errors: {errors:?}");
    expand_resources(&mut config);

    assert_eq!(config.resources["reader"].depends_on, vec!["shards-2"]);
}

#[test]
fn test_fj203_dep_rewrite_for_each() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  homes:
    type: file
    machine: m1
    path: "/home/{{item}}"
    for_each: [alice, bob]
  setup:
    type: file
    machine: m1
    path: "/etc/done"
    depends_on: [homes]
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_resources(&mut config);
    assert_eq!(config.resources["setup"].depends_on, vec!["homes-bob"]);
}
