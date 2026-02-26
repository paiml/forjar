//! FJ-003: Template resolution and dependency DAG construction.
//!
//! Resolves `{{params.key}}` and `{{machine.name.field}}` templates.
//! Builds a DAG from explicit depends_on edges and computes topological order
//! using Kahn's algorithm with deterministic (alphabetical) tie-breaking.

use super::types::*;
use provable_contracts_macros::contract;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet, VecDeque};

/// Resolve a secret value from environment variables.
///
/// Looks for `FORJAR_SECRET_<KEY>` (uppercase, hyphens become underscores).
/// Example: `{{secrets.db-password}}` resolves from `FORJAR_SECRET_DB_PASSWORD`.
fn resolve_secret(key: &str) -> Result<String, String> {
    let env_key = format!("FORJAR_SECRET_{}", key.to_uppercase().replace('-', "_"));
    std::env::var(&env_key).map_err(|_| {
        format!(
            "secret '{}' not found (set env var {} or use a secrets file)",
            key, env_key
        )
    })
}

/// DAG representation: (in-degree per node, adjacency list).
type Dag = (HashMap<String, usize>, HashMap<String, Vec<String>>);

/// Resolve all template variables in a string.
pub fn resolve_template(
    template: &str,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<String, String> {
    let mut result = template.to_string();
    let mut start = 0;

    while let Some(open) = result[start..].find("{{") {
        let open = start + open;
        let close = result[open..]
            .find("}}")
            .ok_or_else(|| format!("unclosed template at position {}", open))?;
        let close = open + close + 2;
        let key = result[open + 2..close - 2].trim();

        let value: Cow<str> = if let Some(param_key) = key.strip_prefix("params.") {
            Cow::Owned(
                params
                    .get(param_key)
                    .map(yaml_value_to_string)
                    .ok_or_else(|| format!("unknown param: {}", param_key))?,
            )
        } else if let Some(secret_key) = key.strip_prefix("secrets.") {
            Cow::Owned(resolve_secret(secret_key)?)
        } else if key.starts_with("machine.") {
            let parts: Vec<&str> = key.splitn(3, '.').collect();
            if parts.len() != 3 {
                return Err(format!("invalid machine ref: {}", key));
            }
            let machine = machines
                .get(parts[1])
                .ok_or_else(|| format!("unknown machine: {}", parts[1]))?;
            Cow::Borrowed(match parts[2] {
                "addr" => &machine.addr,
                "hostname" => &machine.hostname,
                "user" => &machine.user,
                "arch" => &machine.arch,
                _ => return Err(format!("unknown machine field: {}", parts[2])),
            })
        } else if let Some(data_key) = key.strip_prefix("data.") {
            Cow::Owned(
                params
                    .get(&format!("__data__{}", data_key))
                    .map(yaml_value_to_string)
                    .ok_or_else(|| format!("unknown data source: {}", data_key))?,
            )
        } else {
            return Err(format!("unknown template variable: {}", key));
        };

        result.replace_range(open..close, &value);
        start = open + value.len();
    }

    Ok(result)
}

/// FJ-223: Resolve all data sources and inject values into config params.
/// Data sources are stored with `__data__` prefix to avoid conflicts.
pub fn resolve_data_sources(config: &mut ForjarConfig) -> Result<(), String> {
    for (key, source) in &config.data {
        let value = match source.source_type {
            DataSourceType::File => std::fs::read_to_string(&source.value)
                .map(|s| s.trim().to_string())
                .or_else(|e| {
                    source
                        .default
                        .clone()
                        .ok_or_else(|| format!("data source '{}' file error: {}", key, e))
                }),
            DataSourceType::Command => {
                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&source.value)
                    .output()
                    .map_err(|e| format!("data source '{}' command error: {}", key, e))?;
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    source.default.clone().ok_or_else(|| {
                        format!(
                            "data source '{}' command failed (exit {})",
                            key,
                            output.status.code().unwrap_or(-1)
                        )
                    })
                }
            }
            DataSourceType::Dns => {
                use std::net::ToSocketAddrs;
                let addr_str = format!("{}:0", source.value);
                match addr_str.to_socket_addrs() {
                    Ok(mut addrs) => {
                        if let Some(addr) = addrs.next() {
                            Ok(addr.ip().to_string())
                        } else {
                            source
                                .default
                                .clone()
                                .ok_or_else(|| format!("data source '{}' DNS: no addresses", key))
                        }
                    }
                    Err(e) => source
                        .default
                        .clone()
                        .ok_or_else(|| format!("data source '{}' DNS error: {}", key, e)),
                }
            }
        }?;

        config.params.insert(
            format!("__data__{}", key),
            serde_yaml_ng::Value::String(value),
        );
    }
    Ok(())
}

/// Resolve all templates in a resource's string fields.
pub fn resolve_resource_templates(
    resource: &Resource,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<Resource, String> {
    let mut resolved = resource.clone();

    if let Some(ref content) = resolved.content {
        resolved.content = Some(resolve_template(content, params, machines)?);
    }
    if let Some(ref source) = resolved.source {
        resolved.source = Some(resolve_template(source, params, machines)?);
    }
    if let Some(ref path) = resolved.path {
        resolved.path = Some(resolve_template(path, params, machines)?);
    }
    if let Some(ref target) = resolved.target {
        resolved.target = Some(resolve_template(target, params, machines)?);
    }
    if let Some(ref owner) = resolved.owner {
        resolved.owner = Some(resolve_template(owner, params, machines)?);
    }
    if let Some(ref group) = resolved.group {
        resolved.group = Some(resolve_template(group, params, machines)?);
    }
    if let Some(ref mode) = resolved.mode {
        resolved.mode = Some(resolve_template(mode, params, machines)?);
    }
    if let Some(ref name) = resolved.name {
        resolved.name = Some(resolve_template(name, params, machines)?);
    }
    if let Some(ref options) = resolved.options {
        resolved.options = Some(resolve_template(options, params, machines)?);
    }
    if let Some(ref command) = resolved.command {
        resolved.command = Some(resolve_template(command, params, machines)?);
    }
    if let Some(ref schedule) = resolved.schedule {
        resolved.schedule = Some(resolve_template(schedule, params, machines)?);
    }
    if let Some(ref port) = resolved.port {
        resolved.port = Some(resolve_template(port, params, machines)?);
    }
    if let Some(ref protocol) = resolved.protocol {
        resolved.protocol = Some(resolve_template(protocol, params, machines)?);
    }
    if let Some(ref action) = resolved.action {
        resolved.action = Some(resolve_template(action, params, machines)?);
    }
    if let Some(ref from_addr) = resolved.from_addr {
        resolved.from_addr = Some(resolve_template(from_addr, params, machines)?);
    }
    if let Some(ref image) = resolved.image {
        resolved.image = Some(resolve_template(image, params, machines)?);
    }
    if let Some(ref shell) = resolved.shell {
        resolved.shell = Some(resolve_template(shell, params, machines)?);
    }
    if let Some(ref home) = resolved.home {
        resolved.home = Some(resolve_template(home, params, machines)?);
    }
    if let Some(ref restart) = resolved.restart {
        resolved.restart = Some(resolve_template(restart, params, machines)?);
    }
    if let Some(ref version) = resolved.version {
        resolved.version = Some(resolve_template(version, params, machines)?);
    }
    // Resolve list fields
    resolved.ports = resolved
        .ports
        .iter()
        .map(|p| resolve_template(p, params, machines))
        .collect::<Result<Vec<_>, _>>()?;
    resolved.environment = resolved
        .environment
        .iter()
        .map(|e| resolve_template(e, params, machines))
        .collect::<Result<Vec<_>, _>>()?;
    resolved.volumes = resolved
        .volumes
        .iter()
        .map(|v| resolve_template(v, params, machines))
        .collect::<Result<Vec<_>, _>>()?;
    resolved.packages = resolved
        .packages
        .iter()
        .map(|p| resolve_template(p, params, machines))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(resolved)
}

/// Build a topological execution order from resource dependencies.
/// Uses Kahn's algorithm with alphabetical tie-breaking for determinism.
#[contract("dag-ordering-v1", equation = "topological_sort")]
pub fn build_execution_order(config: &ForjarConfig) -> Result<Vec<String>, String> {
    let resource_ids: Vec<String> = config.resources.keys().cloned().collect();
    let (mut in_degree, mut adjacency) = build_dag(config, &resource_ids)?;
    let order = kahn_sort(&resource_ids, &mut in_degree, &mut adjacency);

    if order.len() != resource_ids.len() {
        let remaining: HashSet<_> = resource_ids.iter().collect();
        let ordered: HashSet<_> = order.iter().collect();
        let cycle_members: Vec<_> = remaining.difference(&ordered).collect();
        return Err(format!(
            "dependency cycle detected involving: {}",
            cycle_members
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    Ok(order)
}

/// FJ-216: Compute parallel waves from the DAG.
///
/// Groups resources into waves where all resources in a wave have no
/// inter-dependencies and can execute concurrently. Wave order respects
/// the DAG: all dependencies of a resource are in earlier waves.
///
/// Returns `Vec<Vec<String>>` where each inner Vec is a concurrent wave.
pub fn compute_parallel_waves(config: &ForjarConfig) -> Result<Vec<Vec<String>>, String> {
    let resource_ids: Vec<String> = config.resources.keys().cloned().collect();
    let (mut in_degree, adjacency) = build_dag(config, &resource_ids)?;

    let mut waves = Vec::new();

    loop {
        // Collect all nodes with in-degree 0 (no remaining deps)
        let mut wave: Vec<String> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| id.clone())
            .collect();

        if wave.is_empty() {
            break;
        }

        wave.sort(); // Deterministic order within wave

        // Remove this wave from the graph
        for id in &wave {
            in_degree.remove(id);
            if let Some(neighbors) = adjacency.get(id) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                    }
                }
            }
        }

        waves.push(wave);
    }

    if !in_degree.is_empty() {
        let cycle_members: Vec<_> = in_degree.keys().cloned().collect();
        return Err(format!(
            "dependency cycle detected involving: {}",
            cycle_members.join(", ")
        ));
    }

    Ok(waves)
}

/// Build adjacency list and in-degree map from resource dependencies.
fn build_dag(config: &ForjarConfig, resource_ids: &[String]) -> Result<Dag, String> {
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

    for id in resource_ids {
        in_degree.insert(id.clone(), 0);
        adjacency.insert(id.clone(), Vec::new());
    }

    for (id, resource) in &config.resources {
        for dep in &resource.depends_on {
            if !config.resources.contains_key(dep) {
                return Err(format!("resource '{}' depends on unknown '{}'", id, dep));
            }
            if let Some(adj) = adjacency.get_mut(dep) {
                adj.push(id.clone());
            }
            if let Some(deg) = in_degree.get_mut(id) {
                *deg += 1;
            }
        }
    }

    Ok((in_degree, adjacency))
}

/// Run Kahn's algorithm with alphabetical tie-breaking.
fn kahn_sort(
    _resource_ids: &[String],
    in_degree: &mut HashMap<String, usize>,
    adjacency: &mut HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut zero_degree: Vec<String> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(id, _)| id.clone())
        .collect();
    zero_degree.sort();
    for id in zero_degree {
        queue.push_back(id);
    }

    let mut order = Vec::new();
    while let Some(current) = queue.pop_front() {
        let mut next_ready: Vec<String> = Vec::new();
        if let Some(neighbors) = adjacency.get(&current) {
            for neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        next_ready.push(neighbor.clone());
                    }
                }
            }
        }
        next_ready.sort();
        for id in next_ready {
            queue.push_back(id);
        }
        order.push(current);
    }

    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_fj003_resolve_params() {
        let mut params = HashMap::new();
        params.insert(
            "name".to_string(),
            serde_yaml_ng::Value::String("world".to_string()),
        );
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("hello {{params.name}}", &params, &machines).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_fj003_resolve_machine_addr() {
        let params = HashMap::new();
        let mut machines = indexmap::IndexMap::new();
        machines.insert(
            "lambda".to_string(),
            Machine {
                hostname: "lambda-box".to_string(),
                addr: "192.168.1.1".to_string(),
                user: "noah".to_string(),
                arch: "x86_64".to_string(),
                ssh_key: None,
                roles: vec![],
                transport: None,
                container: None,
                pepita: None,
                cost: 0,
            },
        );
        let result = resolve_template("ssh {{machine.lambda.addr}}", &params, &machines).unwrap();
        assert_eq!(result, "ssh 192.168.1.1");
    }

    #[test]
    fn test_fj003_resolve_unknown_param() {
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("{{params.missing}}", &params, &machines);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown param"));
    }

    #[test]
    fn test_fj062_resolve_secret_missing() {
        // Use a unique key that definitely won't exist in CI/local env
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("{{secrets.zzz-test-nonexistent-9999}}", &params, &machines);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("FORJAR_SECRET_ZZZ_TEST_NONEXISTENT_9999"));
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_fj062_secret_key_normalization() {
        // Verify the env var name construction (hyphens → underscores, uppercase)
        let result = resolve_secret("db-password");
        // We can't set env vars safely, but we can verify the error message
        // contains the correctly-normalized key
        let err = result.unwrap_err();
        assert!(err.contains("FORJAR_SECRET_DB_PASSWORD"));
    }

    #[test]
    fn test_fj062_secret_from_env_via_subprocess() {
        // Run a child process with the env var set to verify resolution works
        let exe = std::env::current_exe().unwrap();
        let output = std::process::Command::new(exe)
            .env("FORJAR_SECRET_TEST_KEY", "secret_value")
            .arg("--test-threads=1")
            .arg("--exact")
            .arg("core::resolver::tests::test_fj062_secret_inner")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "subprocess failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_fj062_secret_inner() {
        // This test is run via subprocess with FORJAR_SECRET_TEST_KEY set
        if std::env::var("FORJAR_SECRET_TEST_KEY").is_err() {
            return; // Skip when not run via subprocess
        }
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("val={{secrets.test-key}}", &params, &machines).unwrap();
        assert_eq!(result, "val=secret_value");
    }

    #[test]
    fn test_fj003_resolve_multiple() {
        let mut params = HashMap::new();
        params.insert(
            "a".to_string(),
            serde_yaml_ng::Value::String("X".to_string()),
        );
        params.insert(
            "b".to_string(),
            serde_yaml_ng::Value::String("Y".to_string()),
        );
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("{{params.a}}-{{params.b}}", &params, &machines).unwrap();
        assert_eq!(result, "X-Y");
    }

    #[test]
    fn test_fj003_topo_linear() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  a:
    type: package
    machine: m1
    provider: apt
    packages: [x]
  b:
    type: file
    machine: m1
    path: /tmp/b
    depends_on: [a]
  c:
    type: service
    machine: m1
    name: svc
    depends_on: [b]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = build_execution_order(&config).unwrap();
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_fj003_topo_parallel() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  alpha:
    type: package
    machine: m1
    provider: apt
    packages: [x]
  beta:
    type: package
    machine: m1
    provider: apt
    packages: [y]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = build_execution_order(&config).unwrap();
        // Alphabetical tie-breaking: alpha before beta
        assert_eq!(order, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_fj003_topo_diamond() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  top:
    type: package
    machine: m1
    provider: apt
    packages: [x]
  left:
    type: file
    machine: m1
    path: /l
    depends_on: [top]
  right:
    type: file
    machine: m1
    path: /r
    depends_on: [top]
  bottom:
    type: service
    machine: m1
    name: svc
    depends_on: [left, right]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = build_execution_order(&config).unwrap();
        assert_eq!(order[0], "top");
        assert_eq!(order[3], "bottom");
        // left and right between, alphabetical
        assert_eq!(order[1], "left");
        assert_eq!(order[2], "right");
    }

    #[test]
    fn test_fj003_topo_cycle() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  a:
    type: package
    machine: m1
    provider: apt
    packages: [x]
    depends_on: [b]
  b:
    type: package
    machine: m1
    provider: apt
    packages: [y]
    depends_on: [a]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let result = build_execution_order(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cycle"));
    }

    #[test]
    fn test_fj003_resolve_all_fields() {
        let mut params = HashMap::new();
        params.insert(
            "dir".to_string(),
            serde_yaml_ng::Value::String("/data".to_string()),
        );
        let mut machines = indexmap::IndexMap::new();
        machines.insert(
            "m1".to_string(),
            Machine {
                hostname: "m1-box".to_string(),
                addr: "10.0.0.1".to_string(),
                user: "deploy".to_string(),
                arch: "aarch64".to_string(),
                ssh_key: None,
                roles: vec![],
                transport: None,
                container: None,
                pepita: None,
                cost: 0,
            },
        );

        let resource = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: Some("{{params.dir}}/config".to_string()),
            content: Some("host={{machine.m1.hostname}}".to_string()),
            source: Some("{{machine.m1.addr}}:/src".to_string()),
            target: Some("{{params.dir}}/link".to_string()),
            owner: Some("{{machine.m1.user}}".to_string()),
            group: None,
            mode: None,
            name: Some("{{machine.m1.hostname}}-svc".to_string()),
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: Some("{{machine.m1.arch}}".to_string()),
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
        };

        let resolved = resolve_resource_templates(&resource, &params, &machines).unwrap();
        assert_eq!(resolved.path.as_deref(), Some("/data/config"));
        assert_eq!(resolved.content.as_deref(), Some("host=m1-box"));
        assert_eq!(resolved.source.as_deref(), Some("10.0.0.1:/src"));
        assert_eq!(resolved.target.as_deref(), Some("/data/link"));
        assert_eq!(resolved.owner.as_deref(), Some("deploy"));
        assert_eq!(resolved.name.as_deref(), Some("m1-box-svc"));
        assert_eq!(resolved.options.as_deref(), Some("aarch64"));
    }

    #[test]
    fn test_fj003_resolve_machine_fields() {
        let params = HashMap::new();
        let mut machines = indexmap::IndexMap::new();
        machines.insert(
            "srv".to_string(),
            Machine {
                hostname: "srv-01".to_string(),
                addr: "192.168.1.1".to_string(),
                user: "admin".to_string(),
                arch: "x86_64".to_string(),
                ssh_key: None,
                roles: vec![],
                transport: None,
                container: None,
                pepita: None,
                cost: 0,
            },
        );

        // Test all machine field branches
        assert_eq!(
            resolve_template("{{machine.srv.hostname}}", &params, &machines).unwrap(),
            "srv-01"
        );
        assert_eq!(
            resolve_template("{{machine.srv.user}}", &params, &machines).unwrap(),
            "admin"
        );
        assert_eq!(
            resolve_template("{{machine.srv.arch}}", &params, &machines).unwrap(),
            "x86_64"
        );

        // Unknown field
        let err = resolve_template("{{machine.srv.bogus}}", &params, &machines);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("unknown machine field"));

        // Invalid machine ref format
        let err = resolve_template("{{machine.srv}}", &params, &machines);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("invalid machine ref"));
    }

    #[test]
    fn test_fj003_resolve_unknown_template_var() {
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();
        let err = resolve_template("{{bogus.var}}", &params, &machines);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("unknown template variable"));
    }

    #[test]
    fn test_fj003_self_dependency_is_cycle() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  self-ref:
    type: package
    machine: m1
    provider: apt
    packages: [x]
    depends_on: [self-ref]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let result = build_execution_order(&config);
        assert!(result.is_err(), "self-dependency must be detected as cycle");
        assert!(result.unwrap_err().contains("cycle"));
    }

    #[test]
    fn test_fj003_transitive_3_level_chain() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  database:
    type: package
    machine: m1
    provider: apt
    packages: [postgresql]
  schema:
    type: file
    machine: m1
    path: /etc/schema.sql
    depends_on: [database]
  app:
    type: service
    machine: m1
    name: app
    depends_on: [schema]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = build_execution_order(&config).unwrap();
        let pos_db = order.iter().position(|x| x == "database").unwrap();
        let pos_schema = order.iter().position(|x| x == "schema").unwrap();
        let pos_app = order.iter().position(|x| x == "app").unwrap();
        assert!(pos_db < pos_schema, "database before schema");
        assert!(pos_schema < pos_app, "schema before app");
    }

    #[test]
    fn test_fj003_empty_depends_on_vs_missing() {
        // Both empty depends_on and missing depends_on should work the same
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  explicit-empty:
    type: package
    machine: m1
    provider: apt
    packages: [x]
    depends_on: []
  implicit-empty:
    type: package
    machine: m1
    provider: apt
    packages: [y]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = build_execution_order(&config).unwrap();
        assert_eq!(order.len(), 2);
        // Both should be independent — alphabetical tie-break
        assert_eq!(order[0], "explicit-empty");
        assert_eq!(order[1], "implicit-empty");
    }

    #[test]
    fn test_fj003_single_resource_no_deps() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  solo:
    type: file
    machine: m1
    path: /tmp/solo
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = build_execution_order(&config).unwrap();
        assert_eq!(order, vec!["solo"]);
    }

    // ── Falsification tests (DAG Ordering Contract) ─────────────

    /// Helper: build a ForjarConfig from a set of resource names and dependency edges.
    fn dag_config(names: &[&str], edges: &[(&str, &str)]) -> ForjarConfig {
        let mut resources = indexmap::IndexMap::new();
        for name in names {
            let mut deps = vec![];
            for (from, to) in edges {
                if to == name {
                    deps.push(from.to_string());
                }
            }
            resources.insert(
                name.to_string(),
                Resource {
                    resource_type: ResourceType::Package,
                    machine: MachineTarget::Single("m1".to_string()),
                    state: None,
                    depends_on: deps,
                    provider: Some("apt".to_string()),
                    packages: vec!["x".to_string()],
                    version: None,
                    path: None,
                    content: None,
                    source: None,
                    target: None,
                    owner: None,
                    group: None,
                    mode: None,
                    name: None,
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
                },
            );
        }
        let mut machines = indexmap::IndexMap::new();
        machines.insert(
            "m1".to_string(),
            Machine {
                hostname: "m1".to_string(),
                addr: "1.1.1.1".to_string(),
                user: "root".to_string(),
                arch: "x86_64".to_string(),
                ssh_key: None,
                roles: vec![],
                transport: None,
                container: None,
                pepita: None,
                cost: 0,
            },
        );
        ForjarConfig {
            version: "1.0".to_string(),
            name: "test".to_string(),
            description: None,
            params: HashMap::new(),
            machines,
            resources,
            policy: Policy::default(),
            outputs: indexmap::IndexMap::new(),
            policies: vec![],
            data: indexmap::IndexMap::new(),
        }
    }

    proptest! {
        /// FALSIFY-DAG-001: Topological ordering — every dependency appears before its dependent.
        #[test]
        fn falsify_dag_001_topo_ordering(
            n in 2..6usize,
            edge_seed in prop::collection::vec((0..5usize, 0..5usize), 0..4),
        ) {
            let names: Vec<String> = (0..n).map(|i| format!("r{}", i)).collect();
            let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            // Build valid DAG edges (only forward edges to avoid cycles)
            let edges: Vec<(&str, &str)> = edge_seed.iter()
                .filter_map(|&(a, b)| {
                    let a = a % n;
                    let b = b % n;
                    if a < b { Some((name_refs[a], name_refs[b])) } else { None }
                })
                .collect();

            let config = dag_config(&name_refs, &edges);
            let order = build_execution_order(&config).unwrap();

            // Verify: for every edge (a -> b), a appears before b
            for (dep, dependent) in &edges {
                let pos_dep = order.iter().position(|x| x == dep).unwrap();
                let pos_dependent = order.iter().position(|x| x == dependent).unwrap();
                prop_assert!(pos_dep < pos_dependent,
                    "dependency {} (pos {}) must appear before {} (pos {})",
                    dep, pos_dep, dependent, pos_dependent);
            }
        }

        /// FALSIFY-DAG-002: Cycle detection returns Err.
        #[test]
        fn falsify_dag_002_cycle_detection(n in 2..5usize) {
            // Create a cycle: r0 -> r1 -> ... -> r(n-1) -> r0
            let names: Vec<String> = (0..n).map(|i| format!("r{}", i)).collect();
            let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            let edges: Vec<(&str, &str)> = (0..n)
                .map(|i| (name_refs[i], name_refs[(i + 1) % n]))
                .collect();

            let config = dag_config(&name_refs, &edges);
            let result = build_execution_order(&config);
            prop_assert!(result.is_err(), "cycle must be detected");
            prop_assert!(result.unwrap_err().contains("cycle"), "error must mention 'cycle'");
        }

    }

    // ── Additional edge case tests ──────────────────────────────

    #[test]
    fn test_fj003_unclosed_template() {
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();
        let err = resolve_template("hello {{params.name", &params, &machines);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("unclosed template"));
    }

    #[test]
    fn test_fj003_no_template_passthrough() {
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("plain string no templates", &params, &machines).unwrap();
        assert_eq!(result, "plain string no templates");
    }

    #[test]
    fn test_fj003_empty_string_passthrough() {
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("", &params, &machines).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_fj003_mixed_template_types() {
        // Mix params and machine refs in one string
        let mut params = HashMap::new();
        params.insert(
            "port".to_string(),
            serde_yaml_ng::Value::String("8080".to_string()),
        );
        let mut machines = indexmap::IndexMap::new();
        machines.insert(
            "web".to_string(),
            Machine {
                hostname: "web-01".to_string(),
                addr: "10.0.0.1".to_string(),
                user: "deploy".to_string(),
                arch: "x86_64".to_string(),
                ssh_key: None,
                roles: vec![],
                transport: None,
                container: None,
                pepita: None,
                cost: 0,
            },
        );
        let result = resolve_template(
            "http://{{machine.web.addr}}:{{params.port}}",
            &params,
            &machines,
        )
        .unwrap();
        assert_eq!(result, "http://10.0.0.1:8080");
    }

    #[test]
    fn test_fj003_unknown_machine_name() {
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();
        let err = resolve_template("{{machine.ghost.addr}}", &params, &machines);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("unknown machine"));
    }

    #[test]
    fn test_fj003_numeric_param_value() {
        let mut params = HashMap::new();
        params.insert(
            "count".to_string(),
            serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(42)),
        );
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("total={{params.count}}", &params, &machines).unwrap();
        assert_eq!(result, "total=42");
    }

    #[test]
    fn test_fj003_boolean_param_value() {
        let mut params = HashMap::new();
        params.insert("flag".to_string(), serde_yaml_ng::Value::Bool(true));
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("debug={{params.flag}}", &params, &machines).unwrap();
        assert_eq!(result, "debug=true");
    }

    #[test]
    fn test_fj003_resolve_resource_templates_group_and_mode() {
        let mut params = HashMap::new();
        params.insert(
            "grp".to_string(),
            serde_yaml_ng::Value::String("www-data".to_string()),
        );
        params.insert(
            "perm".to_string(),
            serde_yaml_ng::Value::String("0644".to_string()),
        );
        let machines = indexmap::IndexMap::new();

        let resource = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: Some("{{params.grp}}".to_string()),
            mode: Some("{{params.perm}}".to_string()),
            name: None,
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
        };

        let resolved = resolve_resource_templates(&resource, &params, &machines).unwrap();
        assert_eq!(resolved.group.as_deref(), Some("www-data"));
        assert_eq!(resolved.mode.as_deref(), Some("0644"));
    }

    #[test]
    fn test_fj003_wide_fan_out_dag() {
        // One node depended on by many
        let config = dag_config(
            &["root", "a", "b", "c", "d"],
            &[("root", "a"), ("root", "b"), ("root", "c"), ("root", "d")],
        );
        let order = build_execution_order(&config).unwrap();
        assert_eq!(order[0], "root");
        // a, b, c, d in alphabetical order after root
        assert_eq!(order[1], "a");
        assert_eq!(order[2], "b");
        assert_eq!(order[3], "c");
        assert_eq!(order[4], "d");
    }

    #[test]
    fn test_fj003_wide_fan_in_dag() {
        // Many nodes converging to one
        let config = dag_config(
            &["a", "b", "c", "leaf"],
            &[("a", "leaf"), ("b", "leaf"), ("c", "leaf")],
        );
        let order = build_execution_order(&config).unwrap();
        // a, b, c independent — alphabetical
        assert_eq!(order[0], "a");
        assert_eq!(order[1], "b");
        assert_eq!(order[2], "c");
        assert_eq!(order[3], "leaf");
    }

    #[test]
    fn test_fj003_dag_unknown_dependency() {
        let config = dag_config(&["a"], &[("nonexistent", "a")]);
        let result = build_execution_order(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown"));
    }

    #[test]
    fn test_fj003_template_with_whitespace() {
        // Templates with spaces around the key should still resolve
        let mut params = HashMap::new();
        params.insert(
            "name".to_string(),
            serde_yaml_ng::Value::String("val".to_string()),
        );
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("{{ params.name }}", &params, &machines).unwrap();
        assert_eq!(result, "val");
    }

    #[test]
    fn test_fj003_consecutive_templates() {
        let mut params = HashMap::new();
        params.insert(
            "a".to_string(),
            serde_yaml_ng::Value::String("X".to_string()),
        );
        params.insert(
            "b".to_string(),
            serde_yaml_ng::Value::String("Y".to_string()),
        );
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("{{params.a}}{{params.b}}", &params, &machines).unwrap();
        assert_eq!(result, "XY");
    }

    proptest! {
        /// FALSIFY-DAG-003: Deterministic output — same graph always produces same order.
        #[test]
        fn falsify_dag_003_determinism(
            n in 2..6usize,
            edge_seed in prop::collection::vec((0..5usize, 0..5usize), 0..4),
        ) {
            let names: Vec<String> = (0..n).map(|i| format!("r{}", i)).collect();
            let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            let edges: Vec<(&str, &str)> = edge_seed.iter()
                .filter_map(|&(a, b)| {
                    let a = a % n;
                    let b = b % n;
                    if a < b { Some((name_refs[a], name_refs[b])) } else { None }
                })
                .collect();

            let config = dag_config(&name_refs, &edges);
            let order1 = build_execution_order(&config).unwrap();
            let order2 = build_execution_order(&config).unwrap();
            prop_assert_eq!(order1, order2, "build_execution_order must be deterministic");
        }
    }

    // ── Template resolution for Phase 2 fields (FJ-126) ─────────────

    #[test]
    fn test_resolve_port_template() {
        let params = HashMap::from([(
            "port".to_string(),
            serde_yaml_ng::Value::String("8080".to_string()),
        )]);
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.port = Some("{{params.port}}".to_string());
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.port.as_deref(), Some("8080"));
    }

    #[test]
    fn test_resolve_command_template() {
        let params = HashMap::from([(
            "host".to_string(),
            serde_yaml_ng::Value::String("localhost".to_string()),
        )]);
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.command = Some("curl http://{{params.host}}/health".to_string());
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(
            resolved.command.as_deref(),
            Some("curl http://localhost/health")
        );
    }

    #[test]
    fn test_resolve_image_template() {
        let params = HashMap::from([(
            "tag".to_string(),
            serde_yaml_ng::Value::String("v2.1".to_string()),
        )]);
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.image = Some("myapp:{{params.tag}}".to_string());
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.image.as_deref(), Some("myapp:v2.1"));
    }

    #[test]
    fn test_resolve_ports_list_template() {
        let params = HashMap::from([(
            "port".to_string(),
            serde_yaml_ng::Value::String("9090".to_string()),
        )]);
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.ports = vec!["{{params.port}}:8080".to_string(), "443:443".to_string()];
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.ports, vec!["9090:8080", "443:443"]);
    }

    #[test]
    fn test_resolve_environment_list_template() {
        let params = HashMap::from([(
            "env".to_string(),
            serde_yaml_ng::Value::String("prod".to_string()),
        )]);
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.environment = vec!["APP_ENV={{params.env}}".to_string()];
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.environment, vec!["APP_ENV=prod"]);
    }

    #[test]
    fn test_resolve_volumes_list_template() {
        let params = HashMap::from([(
            "data".to_string(),
            serde_yaml_ng::Value::String("/data/app".to_string()),
        )]);
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.volumes = vec!["{{params.data}}:/app/data:ro".to_string()];
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.volumes, vec!["/data/app:/app/data:ro"]);
    }

    #[test]
    fn test_resolve_packages_list_template() {
        let params = HashMap::from([(
            "pkg".to_string(),
            serde_yaml_ng::Value::String("htop".to_string()),
        )]);
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.packages = vec!["curl".to_string(), "{{params.pkg}}".to_string()];
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.packages, vec!["curl", "htop"]);
    }

    fn make_base_resource() -> Resource {
        Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
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
        }
    }

    // ── FJ-131: resolver field coverage tests ─────────────────

    fn test_params() -> HashMap<String, serde_yaml_ng::Value> {
        HashMap::from([(
            "val".to_string(),
            serde_yaml_ng::Value::String("resolved".to_string()),
        )])
    }

    #[test]
    fn test_fj131_resolve_source_field() {
        let params = test_params();
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.source = Some("/data/{{params.val}}/file.txt".to_string());
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.source.as_deref(), Some("/data/resolved/file.txt"));
    }

    #[test]
    fn test_fj131_resolve_target_field() {
        let params = test_params();
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.target = Some("/opt/{{params.val}}/bin".to_string());
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.target.as_deref(), Some("/opt/resolved/bin"));
    }

    #[test]
    fn test_fj131_resolve_options_field() {
        let params = test_params();
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.options = Some("rw,{{params.val}}".to_string());
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.options.as_deref(), Some("rw,resolved"));
    }

    #[test]
    fn test_fj131_resolve_protocol_field() {
        let params = HashMap::from([(
            "proto".to_string(),
            serde_yaml_ng::Value::String("tcp".to_string()),
        )]);
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.protocol = Some("{{params.proto}}".to_string());
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.protocol.as_deref(), Some("tcp"));
    }

    #[test]
    fn test_fj131_resolve_action_field() {
        let params = HashMap::from([(
            "act".to_string(),
            serde_yaml_ng::Value::String("allow".to_string()),
        )]);
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.action = Some("{{params.act}}".to_string());
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.action.as_deref(), Some("allow"));
    }

    #[test]
    fn test_fj131_resolve_from_addr_field() {
        let params = HashMap::from([(
            "cidr".to_string(),
            serde_yaml_ng::Value::String("10.0.0.0/24".to_string()),
        )]);
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.from_addr = Some("{{params.cidr}}".to_string());
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.from_addr.as_deref(), Some("10.0.0.0/24"));
    }

    #[test]
    fn test_fj131_resolve_shell_field() {
        let params = test_params();
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.shell = Some("/bin/{{params.val}}".to_string());
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.shell.as_deref(), Some("/bin/resolved"));
    }

    #[test]
    fn test_fj131_resolve_home_field() {
        let params = HashMap::from([(
            "user".to_string(),
            serde_yaml_ng::Value::String("deploy".to_string()),
        )]);
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.home = Some("/home/{{params.user}}".to_string());
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.home.as_deref(), Some("/home/deploy"));
    }

    #[test]
    fn test_fj131_resolve_restart_field() {
        let params = HashMap::from([(
            "policy".to_string(),
            serde_yaml_ng::Value::String("unless-stopped".to_string()),
        )]);
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.restart = Some("{{params.policy}}".to_string());
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.restart.as_deref(), Some("unless-stopped"));
    }

    #[test]
    fn test_fj131_resolve_version_field() {
        let params = HashMap::from([(
            "ver".to_string(),
            serde_yaml_ng::Value::String("2.1.0".to_string()),
        )]);
        let machines = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.version = Some("{{params.ver}}".to_string());
        let resolved = resolve_resource_templates(&r, &params, &machines).unwrap();
        assert_eq!(resolved.version.as_deref(), Some("2.1.0"));
    }

    #[test]
    fn test_fj131_resolve_machine_hostname_field() {
        let params = HashMap::new();
        let mut machines = indexmap::IndexMap::new();
        machines.insert(
            "db".to_string(),
            Machine {
                hostname: "db-primary".to_string(),
                addr: "10.0.0.5".to_string(),
                user: "postgres".to_string(),
                arch: "aarch64".to_string(),
                ssh_key: None,
                roles: vec![],
                transport: None,
                container: None,
                pepita: None,
                cost: 0,
            },
        );
        let result = resolve_template("host={{machine.db.hostname}}", &params, &machines).unwrap();
        assert_eq!(result, "host=db-primary");
    }

    #[test]
    fn test_fj131_resolve_machine_user_field() {
        let params = HashMap::new();
        let mut machines = indexmap::IndexMap::new();
        machines.insert(
            "db".to_string(),
            Machine {
                hostname: "db".to_string(),
                addr: "10.0.0.5".to_string(),
                user: "postgres".to_string(),
                arch: "x86_64".to_string(),
                ssh_key: None,
                roles: vec![],
                transport: None,
                container: None,
                pepita: None,
                cost: 0,
            },
        );
        let result = resolve_template("user={{machine.db.user}}", &params, &machines).unwrap();
        assert_eq!(result, "user=postgres");
    }

    #[test]
    fn test_fj131_resolve_machine_arch_field() {
        let params = HashMap::new();
        let mut machines = indexmap::IndexMap::new();
        machines.insert(
            "arm".to_string(),
            Machine {
                hostname: "arm".to_string(),
                addr: "10.0.0.6".to_string(),
                user: "root".to_string(),
                arch: "aarch64".to_string(),
                ssh_key: None,
                roles: vec![],
                transport: None,
                container: None,
                pepita: None,
                cost: 0,
            },
        );
        let result = resolve_template("arch={{machine.arm.arch}}", &params, &machines).unwrap();
        assert_eq!(result, "arch=aarch64");
    }

    #[test]
    fn test_fj131_resolve_machine_invalid_field() {
        let params = HashMap::new();
        let mut machines = indexmap::IndexMap::new();
        machines.insert(
            "m".to_string(),
            Machine {
                hostname: "m".to_string(),
                addr: "1.1.1.1".to_string(),
                user: "root".to_string(),
                arch: "x86_64".to_string(),
                ssh_key: None,
                roles: vec![],
                transport: None,
                container: None,
                pepita: None,
                cost: 0,
            },
        );
        let result = resolve_template("{{machine.m.cost}}", &params, &machines);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown machine field"));
    }

    #[test]
    fn test_fj131_resolve_machine_ref_too_few_parts() {
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("{{machine.only}}", &params, &machines);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid machine ref"));
    }

    #[test]
    fn test_fj131_resolve_unknown_template_type() {
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("{{foobar.baz}}", &params, &machines);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown template variable"));
    }

    #[test]
    fn test_fj132_resolve_secret_missing() {
        // Use a key that won't exist in any CI/local env
        let result = resolve_secret("zzz-nonexistent-key-12345");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("FORJAR_SECRET_ZZZ_NONEXISTENT_KEY_12345"));
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_fj132_resolve_secret_env_key_format() {
        // Verify the env key derivation: hyphens → underscores, uppercase
        let result = resolve_secret("my-db-pass");
        // Will fail because env var doesn't exist, but error message shows the derived key
        let err = result.unwrap_err();
        assert!(
            err.contains("FORJAR_SECRET_MY_DB_PASS"),
            "should derive FORJAR_SECRET_MY_DB_PASS from 'my-db-pass'"
        );
    }

    #[test]
    fn test_fj132_resolve_resource_templates_list_fields() {
        let mut params = HashMap::new();
        params.insert(
            "port".to_string(),
            serde_yaml_ng::Value::String("8080".to_string()),
        );
        let machines = indexmap::IndexMap::new();
        let mut resource = make_base_resource();
        resource.resource_type = ResourceType::Docker;
        resource.ports = vec!["{{params.port}}:{{params.port}}".to_string()];
        resource.environment = vec!["PORT={{params.port}}".to_string()];
        let resolved = resolve_resource_templates(&resource, &params, &machines).unwrap();
        assert_eq!(resolved.ports, vec!["8080:8080"]);
        assert_eq!(resolved.environment, vec!["PORT=8080"]);
    }

    #[test]
    fn test_fj132_build_dag_unknown_dependency() {
        let mut resources = indexmap::IndexMap::new();
        let mut r = make_base_resource();
        r.depends_on = vec!["nonexistent".to_string()];
        resources.insert("my-file".to_string(), r);
        let config = ForjarConfig {
            version: "1.0".to_string(),
            name: "test".to_string(),
            description: None,
            params: HashMap::new(),
            machines: indexmap::IndexMap::new(),
            resources,
            policy: Policy::default(),
            outputs: indexmap::IndexMap::new(),
            policies: vec![],
            data: indexmap::IndexMap::new(),
        };
        let result = build_execution_order(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown"));
    }

    #[test]
    fn test_fj132_kahn_sort_diamond_dependency() {
        // A -> B, A -> C, B -> D, C -> D (diamond shape)
        let mut resources = indexmap::IndexMap::new();
        let mut r_a = make_base_resource();
        r_a.resource_type = ResourceType::Package;
        resources.insert("a".to_string(), r_a);
        let mut r_b = make_base_resource();
        r_b.resource_type = ResourceType::Package;
        r_b.depends_on = vec!["a".to_string()];
        resources.insert("b".to_string(), r_b);
        let mut r_c = make_base_resource();
        r_c.resource_type = ResourceType::Package;
        r_c.depends_on = vec!["a".to_string()];
        resources.insert("c".to_string(), r_c);
        let mut r_d = make_base_resource();
        r_d.resource_type = ResourceType::Package;
        r_d.depends_on = vec!["b".to_string(), "c".to_string()];
        resources.insert("d".to_string(), r_d);
        let config = ForjarConfig {
            version: "1.0".to_string(),
            name: "test".to_string(),
            description: None,
            params: HashMap::new(),
            machines: indexmap::IndexMap::new(),
            resources,
            policy: Policy::default(),
            outputs: indexmap::IndexMap::new(),
            policies: vec![],
            data: indexmap::IndexMap::new(),
        };
        let order = build_execution_order(&config).unwrap();
        assert_eq!(order, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_fj132_build_execution_order_empty() {
        let config = ForjarConfig {
            version: "1.0".to_string(),
            name: "test".to_string(),
            description: None,
            params: HashMap::new(),
            machines: indexmap::IndexMap::new(),
            resources: indexmap::IndexMap::new(),
            policy: Policy::default(),
            outputs: indexmap::IndexMap::new(),
            policies: vec![],
            data: indexmap::IndexMap::new(),
        };
        let order = build_execution_order(&config).unwrap();
        assert!(order.is_empty());
    }

    #[test]
    fn test_fj132_unclosed_template() {
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("hello {{params.name", &params, &machines);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unclosed template"));
    }

    #[test]
    fn test_fj132_resolve_template_secret_missing_error() {
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("token={{secrets.zzz-missing-99}}", &params, &machines);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("FORJAR_SECRET_ZZZ_MISSING_99"));
    }

    #[test]
    fn test_resolve_template_nested_braces() {
        let mut params = HashMap::new();
        params.insert(
            "x".to_string(),
            serde_yaml_ng::Value::String("value".to_string()),
        );
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("{{params.x}}_suffix", &params, &machines).unwrap();
        assert_eq!(result, "value_suffix");
    }

    #[test]
    fn test_resolve_template_multiple() {
        let mut params = HashMap::new();
        params.insert(
            "a".to_string(),
            serde_yaml_ng::Value::String("hello".to_string()),
        );
        params.insert(
            "b".to_string(),
            serde_yaml_ng::Value::String("world".to_string()),
        );
        let machines = indexmap::IndexMap::new();
        let result = resolve_template("{{params.a}}-{{params.b}}", &params, &machines).unwrap();
        assert_eq!(result, "hello-world");
    }

    #[test]
    fn test_build_execution_order_empty() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources: {}
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = build_execution_order(&config).unwrap();
        assert!(order.is_empty());
    }

    #[test]
    fn test_build_execution_order_no_deps() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  alpha:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  beta:
    type: file
    machine: m1
    path: /tmp/beta.txt
    content: "beta content"
  gamma:
    type: service
    machine: m1
    name: gamma-svc
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let order = build_execution_order(&config).unwrap();
        assert_eq!(order.len(), 3);
        // Alphabetical tie-breaking with no deps
        assert_eq!(order, vec!["alpha", "beta", "gamma"]);
    }

    // ================================================================
    // FJ-216: parallel wave computation tests
    // ================================================================

    #[test]
    fn test_fj216_waves_no_deps() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  a:
    type: file
    machine: m1
    path: /a
  b:
    type: file
    machine: m1
    path: /b
  c:
    type: file
    machine: m1
    path: /c
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let waves = compute_parallel_waves(&config).unwrap();
        assert_eq!(waves.len(), 1, "all independent = single wave");
        assert_eq!(waves[0].len(), 3);
    }

    #[test]
    fn test_fj216_waves_linear_chain() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  a:
    type: file
    machine: m1
    path: /a
  b:
    type: file
    machine: m1
    path: /b
    depends_on: [a]
  c:
    type: file
    machine: m1
    path: /c
    depends_on: [b]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let waves = compute_parallel_waves(&config).unwrap();
        assert_eq!(waves.len(), 3, "linear chain = 3 waves");
        assert_eq!(waves[0], vec!["a"]);
        assert_eq!(waves[1], vec!["b"]);
        assert_eq!(waves[2], vec!["c"]);
    }

    #[test]
    fn test_fj216_waves_diamond() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  root:
    type: file
    machine: m1
    path: /root
  left:
    type: file
    machine: m1
    path: /left
    depends_on: [root]
  right:
    type: file
    machine: m1
    path: /right
    depends_on: [root]
  join:
    type: file
    machine: m1
    path: /join
    depends_on: [left, right]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let waves = compute_parallel_waves(&config).unwrap();
        assert_eq!(waves.len(), 3, "diamond = 3 waves");
        assert_eq!(waves[0], vec!["root"]);
        assert_eq!(waves[1], vec!["left", "right"]); // concurrent
        assert_eq!(waves[2], vec!["join"]);
    }

    #[test]
    fn test_fj216_waves_mixed() {
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
    path: /base
  pkg-a:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
    depends_on: [base]
  pkg-b:
    type: package
    machine: m1
    provider: apt
    packages: [git]
    depends_on: [base]
  independent:
    type: file
    machine: m1
    path: /ind
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let waves = compute_parallel_waves(&config).unwrap();
        assert_eq!(waves.len(), 2);
        // Wave 0: base + independent (both have in-degree 0)
        assert!(waves[0].contains(&"base".to_string()));
        assert!(waves[0].contains(&"independent".to_string()));
        // Wave 1: pkg-a + pkg-b (both depend only on base)
        assert!(waves[1].contains(&"pkg-a".to_string()));
        assert!(waves[1].contains(&"pkg-b".to_string()));
    }

    // ================================================================
    // FJ-223: Data source tests
    // ================================================================

    #[test]
    fn test_fj223_data_source_file() {
        let dir = tempfile::tempdir().unwrap();
        let data_file = dir.path().join("version.txt");
        std::fs::write(&data_file, "1.2.3\n").unwrap();

        let yaml = format!(
            r#"
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
    path: /etc/version
    content: "v={{{{data.app_version}}}}"
data:
  app_version:
    type: file
    value: "{}"
"#,
            data_file.display()
        );
        let mut config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        resolve_data_sources(&mut config).unwrap();

        // Should have injected __data__app_version
        let val = config.params.get("__data__app_version").unwrap();
        assert_eq!(yaml_value_to_string(val), "1.2.3");
    }

    #[test]
    fn test_fj223_data_source_command() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  hostname:
    type: command
    value: "echo test-host"
"#;
        let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        resolve_data_sources(&mut config).unwrap();

        let val = config.params.get("__data__hostname").unwrap();
        assert_eq!(yaml_value_to_string(val), "test-host");
    }

    #[test]
    fn test_fj223_data_source_file_with_default() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  missing:
    type: file
    value: /nonexistent/file
    default: "fallback"
"#;
        let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        resolve_data_sources(&mut config).unwrap();

        let val = config.params.get("__data__missing").unwrap();
        assert_eq!(yaml_value_to_string(val), "fallback");
    }

    #[test]
    fn test_fj223_data_source_file_no_default_fails() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  missing:
    type: file
    value: /nonexistent/file
"#;
        let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let result = resolve_data_sources(&mut config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("file error"));
    }

    #[test]
    fn test_fj223_data_template_resolution() {
        let dir = tempfile::tempdir().unwrap();
        let data_file = dir.path().join("env.txt");
        std::fs::write(&data_file, "production").unwrap();

        let yaml = format!(
            r#"
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
    path: /etc/env.conf
    content: "env={{{{data.env}}}}"
data:
  env:
    type: file
    value: "{}"
"#,
            data_file.display()
        );
        let mut config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        resolve_data_sources(&mut config).unwrap();

        // Now resolve the template
        let resolved =
            resolve_template("env={{data.env}}", &config.params, &config.machines).unwrap();
        assert_eq!(resolved, "env=production");
    }

    #[test]
    fn test_fj223_data_source_command_with_default() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  fail:
    type: command
    value: "exit 1"
    default: "fallback"
"#;
        let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        resolve_data_sources(&mut config).unwrap();

        let val = config.params.get("__data__fail").unwrap();
        assert_eq!(yaml_value_to_string(val), "fallback");
    }

    #[test]
    fn test_fj223_data_source_dns() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
data:
  loopback:
    type: dns
    value: localhost
"#;
        let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        resolve_data_sources(&mut config).unwrap();

        let val = config.params.get("__data__loopback").unwrap();
        let ip = yaml_value_to_string(val);
        assert!(ip == "127.0.0.1" || ip == "::1", "got: {}", ip);
    }

    #[test]
    fn test_fj223_no_data_sources() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
"#;
        let mut config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        resolve_data_sources(&mut config).unwrap();
        // No __data__ keys added
        assert!(!config.params.keys().any(|k| k.starts_with("__data__")));
    }
}
