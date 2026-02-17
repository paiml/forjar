//! FJ-003: Template resolution and dependency DAG construction.
//!
//! Resolves `{{params.key}}` and `{{machine.name.field}}` templates.
//! Builds a DAG from explicit depends_on edges and computes topological order
//! using Kahn's algorithm with deterministic (alphabetical) tie-breaking.

use super::types::*;
use std::collections::{HashMap, HashSet, VecDeque};

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

        let value = if let Some(param_key) = key.strip_prefix("params.") {
            params
                .get(param_key)
                .map(yaml_value_to_string)
                .ok_or_else(|| format!("unknown param: {}", param_key))?
        } else if key.starts_with("machine.") {
            let parts: Vec<&str> = key.splitn(3, '.').collect();
            if parts.len() != 3 {
                return Err(format!("invalid machine ref: {}", key));
            }
            let machine = machines
                .get(parts[1])
                .ok_or_else(|| format!("unknown machine: {}", parts[1]))?;
            match parts[2] {
                "addr" => machine.addr.clone(),
                "hostname" => machine.hostname.clone(),
                "user" => machine.user.clone(),
                "arch" => machine.arch.clone(),
                _ => return Err(format!("unknown machine field: {}", parts[2])),
            }
        } else {
            return Err(format!("unknown template variable: {}", key));
        };

        result.replace_range(open..close, &value);
        start = open + value.len();
    }

    Ok(result)
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

    Ok(resolved)
}

/// Build a topological execution order from resource dependencies.
/// Uses Kahn's algorithm with alphabetical tie-breaking for determinism.
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
            },
        );

        let resource = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m1".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
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
            fs_type: None,
            options: Some("{{machine.m1.arch}}".to_string()),
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
}
