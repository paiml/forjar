//! FJ-002: YAML parsing and validation.
//!
//! Parses forjar.yaml and validates structural constraints:
//! - Version must be "1.0"
//! - Machine references in resources must exist
//! - depends_on references must exist
//! - Required fields per resource type

use super::recipe;
use super::types::*;
use std::path::Path;

/// Recognized CPU architectures for the `arch` field.
const KNOWN_ARCHITECTURES: &[&str] =
    &["x86_64", "aarch64", "armv7l", "riscv64", "s390x", "ppc64le"];

/// Validation error.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Parse a forjar.yaml file from disk.
pub fn parse_config_file(path: &Path) -> Result<ForjarConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    parse_config(&content)
}

/// Parse a forjar.yaml from a string.
pub fn parse_config(yaml: &str) -> Result<ForjarConfig, String> {
    serde_yaml_ng::from_str(yaml).map_err(|e| format!("YAML parse error: {}", e))
}

/// Validate a parsed config. Returns a list of errors (empty = valid).
pub fn validate_config(config: &ForjarConfig) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if config.version != "1.0" {
        errors.push(ValidationError {
            message: format!("version must be \"1.0\", got \"{}\"", config.version),
        });
    }

    if config.name.is_empty() {
        errors.push(ValidationError {
            message: "name must not be empty".to_string(),
        });
    }

    for (id, resource) in &config.resources {
        validate_resource_refs(config, id, resource, &mut errors);
        validate_resource_type(id, resource, &mut errors);
    }

    for (key, machine) in &config.machines {
        validate_machine(key, machine, &mut errors);
    }

    errors
}

/// Validate machine and dependency references for a single resource.
fn validate_resource_refs(
    config: &ForjarConfig,
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    for machine_name in resource.machine.to_vec() {
        if !config.machines.contains_key(&machine_name) && machine_name != "localhost" {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' references unknown machine '{}'",
                    id, machine_name
                ),
            });
        }
    }

    // FJ-064: Validate arch filter values
    for arch in &resource.arch {
        if !KNOWN_ARCHITECTURES.contains(&arch.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{}' has unknown arch '{}' (expected one of: {})",
                    id,
                    arch,
                    KNOWN_ARCHITECTURES.join(", ")
                ),
            });
        }
    }

    for dep in &resource.depends_on {
        if !config.resources.contains_key(dep) {
            // FJ-203/FJ-204: Dep might reference a resource that will be expanded.
            // After expansion, the original ID disappears — this is valid.
            let dep_resource = config.resources.get(dep);
            let will_expand = dep_resource
                .map(|r| r.count.is_some() || r.for_each.is_some())
                .unwrap_or(false);
            if !will_expand {
                errors.push(ValidationError {
                    message: format!("resource '{}' depends on unknown resource '{}'", id, dep),
                });
            }
        }
        if dep == id {
            errors.push(ValidationError {
                message: format!("resource '{}' depends on itself", id),
            });
        }
    }

    // FJ-224: Validate triggers — must reference existing resources
    for trigger in &resource.triggers {
        if !config.resources.contains_key(trigger) {
            let trigger_resource = config.resources.get(trigger);
            let will_expand = trigger_resource
                .map(|r| r.count.is_some() || r.for_each.is_some())
                .unwrap_or(false);
            if !will_expand {
                errors.push(ValidationError {
                    message: format!(
                        "resource '{}' triggers on unknown resource '{}'",
                        id, trigger
                    ),
                });
            }
        }
        if trigger == id {
            errors.push(ValidationError {
                message: format!("resource '{}' triggers on itself", id),
            });
        }
    }

    // FJ-203/FJ-204: Validate count and for_each
    if resource.count.is_some() && resource.for_each.is_some() {
        errors.push(ValidationError {
            message: format!("resource '{}' cannot have both 'count' and 'for_each'", id),
        });
    }
    if let Some(count) = resource.count {
        if count == 0 {
            errors.push(ValidationError {
                message: format!("resource '{}' has count: 0 (must be >= 1)", id),
            });
        }
    }
    if let Some(ref items) = resource.for_each {
        if items.is_empty() {
            errors.push(ValidationError {
                message: format!("resource '{}' has empty for_each list", id),
            });
        }
    }
}

/// Validate machine configuration (container transport rules, arch).
fn validate_machine(key: &str, machine: &Machine, errors: &mut Vec<ValidationError>) {
    // FJ-064: Validate machine arch
    if !KNOWN_ARCHITECTURES.contains(&machine.arch.as_str()) {
        errors.push(ValidationError {
            message: format!(
                "machine '{}' has unknown arch '{}' (expected one of: {})",
                key,
                machine.arch,
                KNOWN_ARCHITECTURES.join(", ")
            ),
        });
    }

    if machine.is_container_transport() && machine.container.is_none() {
        errors.push(ValidationError {
            message: format!(
                "machine '{}' uses container transport but has no 'container' block",
                key
            ),
        });
    }

    if let Some(ref container) = machine.container {
        if container.runtime != "docker" && container.runtime != "podman" {
            errors.push(ValidationError {
                message: format!(
                    "machine '{}' container runtime must be 'docker' or 'podman', got '{}'",
                    key, container.runtime
                ),
            });
        }
        if container.ephemeral && container.image.is_none() {
            errors.push(ValidationError {
                message: format!("machine '{}' is ephemeral but has no container image", key),
            });
        }
    }
}

/// Validate type-specific required fields for a resource.
fn validate_resource_type(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    match resource.resource_type {
        ResourceType::Package => {
            if resource.packages.is_empty() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (package) has no packages", id),
                });
            }
            if resource.provider.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (package) has no provider", id),
                });
            }
        }
        ResourceType::File => {
            if resource.path.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (file) has no path", id),
                });
            }
            if resource.content.is_some() && resource.source.is_some() {
                errors.push(ValidationError {
                    message: format!(
                        "resource '{}' (file) has both content and source (pick one)",
                        id
                    ),
                });
            }
            if let Some(ref state) = resource.state {
                let valid = ["file", "directory", "symlink", "absent"];
                if !valid.contains(&state.as_str()) {
                    errors.push(ValidationError {
                        message: format!(
                            "resource '{}' (file) has invalid state '{}' (expected: {})",
                            id,
                            state,
                            valid.join(", ")
                        ),
                    });
                }
            }
            if resource.state.as_deref() == Some("symlink") && resource.target.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (file) state=symlink requires a target", id),
                });
            }
        }
        ResourceType::Service => {
            if resource.name.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (service) has no name", id),
                });
            }
            if let Some(ref state) = resource.state {
                let valid = ["running", "stopped", "enabled", "disabled"];
                if !valid.contains(&state.as_str()) {
                    errors.push(ValidationError {
                        message: format!(
                            "resource '{}' (service) has invalid state '{}' (expected: {})",
                            id,
                            state,
                            valid.join(", ")
                        ),
                    });
                }
            }
        }
        ResourceType::Mount => {
            if resource.source.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (mount) has no source", id),
                });
            }
            if resource.path.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (mount) has no path", id),
                });
            }
            if let Some(ref state) = resource.state {
                let valid = ["mounted", "unmounted", "absent"];
                if !valid.contains(&state.as_str()) {
                    errors.push(ValidationError {
                        message: format!(
                            "resource '{}' (mount) has invalid state '{}' (expected: {})",
                            id,
                            state,
                            valid.join(", ")
                        ),
                    });
                }
            }
        }
        ResourceType::User => {
            if resource.name.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (user) has no name", id),
                });
            }
        }
        ResourceType::Docker => {
            if resource.name.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (docker) has no name", id),
                });
            }
            if resource.image.is_none() && resource.state.as_deref() != Some("absent") {
                errors.push(ValidationError {
                    message: format!("resource '{}' (docker) has no image", id),
                });
            }
            if let Some(ref state) = resource.state {
                let valid = ["running", "stopped", "absent"];
                if !valid.contains(&state.as_str()) {
                    errors.push(ValidationError {
                        message: format!(
                            "resource '{}' (docker) has invalid state '{}' (expected: {})",
                            id,
                            state,
                            valid.join(", ")
                        ),
                    });
                }
            }
        }
        ResourceType::Cron => {
            if resource.name.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (cron) has no name", id),
                });
            }
            if resource.schedule.is_none() && resource.state.as_deref() != Some("absent") {
                errors.push(ValidationError {
                    message: format!("resource '{}' (cron) has no schedule", id),
                });
            }
            if let Some(ref sched) = resource.schedule {
                let fields: Vec<&str> = sched.split_whitespace().collect();
                if fields.len() != 5 {
                    errors.push(ValidationError {
                        message: format!(
                            "resource '{}' (cron) schedule '{}' must have exactly 5 fields (min hour dom mon dow)",
                            id, sched
                        ),
                    });
                }
            }
            if resource.command.is_none() && resource.state.as_deref() != Some("absent") {
                errors.push(ValidationError {
                    message: format!("resource '{}' (cron) has no command", id),
                });
            }
            if let Some(ref state) = resource.state {
                let valid = ["present", "absent"];
                if !valid.contains(&state.as_str()) {
                    errors.push(ValidationError {
                        message: format!(
                            "resource '{}' (cron) has invalid state '{}' (expected: present, absent)",
                            id, state
                        ),
                    });
                }
            }
        }
        ResourceType::Network => {
            if resource.port.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (network) has no port", id),
                });
            }
            if let Some(ref proto) = resource.protocol {
                let valid = ["tcp", "udp"];
                if !valid.contains(&proto.as_str()) {
                    errors.push(ValidationError {
                        message: format!(
                            "resource '{}' (network) has invalid protocol '{}' (expected: tcp, udp)",
                            id, proto
                        ),
                    });
                }
            }
            if let Some(ref action) = resource.action {
                let valid = ["allow", "deny", "reject"];
                if !valid.contains(&action.as_str()) {
                    errors.push(ValidationError {
                        message: format!(
                            "resource '{}' (network) has invalid action '{}' (expected: allow, deny, reject)",
                            id, action
                        ),
                    });
                }
            }
        }
        ResourceType::Pepita => {
            if resource.name.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (pepita) has no name", id),
                });
            }
            if let Some(ref state) = resource.state {
                let valid = ["present", "absent"];
                if !valid.contains(&state.as_str()) {
                    errors.push(ValidationError {
                        message: format!(
                            "resource '{}' (pepita) has invalid state '{}' (expected: present, absent)",
                            id, state
                        ),
                    });
                }
            }
            if resource.overlay_merged.is_some()
                && resource.overlay_lower.is_none()
                && resource.overlay_upper.is_none()
            {
                // overlay_merged without explicit dirs uses defaults — valid but warn-worthy
            }
            if let Some(ref cpuset) = resource.cpuset {
                if cpuset.is_empty() {
                    errors.push(ValidationError {
                        message: format!("resource '{}' (pepita) has empty cpuset", id),
                    });
                }
            }
        }
        ResourceType::Model => {
            if resource.name.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (model) has no name", id),
                });
            }
            if let Some(ref state) = resource.state {
                let valid = ["present", "absent"];
                if !valid.contains(&state.as_str()) {
                    errors.push(ValidationError {
                        message: format!(
                            "resource '{}' (model) has invalid state '{}' (expected: present, absent)",
                            id, state
                        ),
                    });
                }
            }
        }
        ResourceType::Gpu => {
            if resource.driver_version.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (gpu) has no driver_version", id),
                });
            }
            if let Some(ref state) = resource.state {
                let valid = ["present", "absent"];
                if !valid.contains(&state.as_str()) {
                    errors.push(ValidationError {
                        message: format!(
                            "resource '{}' (gpu) has invalid state '{}' (expected: present, absent)",
                            id, state
                        ),
                    });
                }
            }
        }
        ResourceType::Recipe => {
            if resource.recipe.is_none() {
                errors.push(ValidationError {
                    message: format!("resource '{}' (recipe) has no recipe name", id),
                });
            }
        }
    }
}

/// Parse, validate, and expand recipes in a config file.
/// This is the main entry point for loading a config for plan/apply.
pub fn parse_and_validate(path: &Path) -> Result<ForjarConfig, String> {
    let mut config = parse_config_file(path)?;
    let errors = validate_config(&config);
    if !errors.is_empty() {
        return Err(format!(
            "validation errors:\n{}",
            errors
                .iter()
                .map(|e| format!("  - {}", e))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    expand_recipes(&mut config, path.parent())?;
    expand_resources(&mut config);
    Ok(config)
}

/// Expand recipe resources into their constituent resources.
/// Recipe resources (type: recipe) are replaced with the expanded resources
/// from the referenced recipe file.
pub fn expand_recipes(config: &mut ForjarConfig, config_dir: Option<&Path>) -> Result<(), String> {
    let base_dir = config_dir.unwrap_or_else(|| Path::new("."));
    let mut expanded = indexmap::IndexMap::new();

    for (id, resource) in &config.resources {
        if resource.resource_type != ResourceType::Recipe {
            expanded.insert(id.clone(), resource.clone());
            continue;
        }

        let recipe_name = resource
            .recipe
            .as_deref()
            .ok_or_else(|| format!("recipe resource '{}' has no recipe name", id))?;

        // Look for recipe file relative to config directory
        let recipe_path = base_dir
            .join("recipes")
            .join(format!("{}.yaml", recipe_name));
        if !recipe_path.exists() {
            return Err(format!(
                "recipe '{}' not found at {}",
                recipe_name,
                recipe_path.display()
            ));
        }

        let recipe_file = recipe::load_recipe(&recipe_path)?;
        let expanded_resources = recipe::expand_recipe(
            id,
            &recipe_file,
            &resource.machine,
            &resource.inputs,
            &resource.depends_on,
        )?;

        for (res_id, res) in expanded_resources {
            expanded.insert(res_id, res);
        }
    }

    config.resources = expanded;
    Ok(())
}

// ========================================================================
// FJ-220: Policy evaluation
// ========================================================================

use crate::core::types::{PolicyRuleType, PolicyViolation};

/// Check if a resource has a given field set (non-None, non-empty).
fn resource_has_field(resource: &Resource, field: &str) -> bool {
    match field {
        "owner" => resource.owner.is_some(),
        "group" => resource.group.is_some(),
        "mode" => resource.mode.is_some(),
        "tags" => !resource.tags.is_empty(),
        "path" => resource.path.is_some(),
        "content" => resource.content.is_some(),
        "source" => resource.source.is_some(),
        "name" => resource.name.is_some(),
        "provider" => resource.provider.is_some(),
        "packages" => !resource.packages.is_empty(),
        "depends_on" => !resource.depends_on.is_empty(),
        "shell" => resource.shell.is_some(),
        "home" => resource.home.is_some(),
        "schedule" => resource.schedule.is_some(),
        "command" => resource.command.is_some(),
        "image" => resource.image.is_some(),
        "state" => resource.state.is_some(),
        "when" => resource.when.is_some(),
        _ => false,
    }
}

/// Get a string representation of a resource field for condition checks.
fn resource_field_value(resource: &Resource, field: &str) -> Option<String> {
    match field {
        "owner" => resource.owner.clone(),
        "group" => resource.group.clone(),
        "mode" => resource.mode.clone(),
        "path" => resource.path.clone(),
        "content" => resource.content.clone(),
        "source" => resource.source.clone(),
        "name" => resource.name.clone(),
        "provider" => resource.provider.clone(),
        "state" => resource.state.clone(),
        "type" => Some(format!("{:?}", resource.resource_type).to_lowercase()),
        "shell" => resource.shell.clone(),
        "home" => resource.home.clone(),
        "schedule" => resource.schedule.clone(),
        "command" => resource.command.clone(),
        "image" => resource.image.clone(),
        _ => None,
    }
}

/// Evaluate all policy rules against all resources. Returns violations.
pub fn evaluate_policies(config: &ForjarConfig) -> Vec<PolicyViolation> {
    let mut violations = Vec::new();

    for rule in &config.policies {
        for (id, resource) in &config.resources {
            // Filter by resource_type if specified
            if let Some(ref rt) = rule.resource_type {
                let actual = format!("{:?}", resource.resource_type).to_lowercase();
                if actual != *rt {
                    continue;
                }
            }

            // Filter by tag if specified
            if let Some(ref tag) = rule.tag {
                if !resource.tags.contains(tag) {
                    continue;
                }
            }

            let violated = match rule.rule_type {
                PolicyRuleType::Require => {
                    // Resource must have the field set
                    if let Some(ref field) = rule.field {
                        !resource_has_field(resource, field)
                    } else {
                        false
                    }
                }
                PolicyRuleType::Deny | PolicyRuleType::Warn => {
                    // Check if condition field matches condition value
                    if let (Some(ref field), Some(ref value)) =
                        (&rule.condition_field, &rule.condition_value)
                    {
                        resource_field_value(resource, field).as_deref() == Some(value.as_str())
                    } else {
                        false
                    }
                }
            };

            if violated {
                violations.push(PolicyViolation {
                    rule_message: rule.message.clone(),
                    resource_id: id.clone(),
                    severity: rule.rule_type.clone(),
                });
            }
        }
    }

    violations
}

/// FJ-203/FJ-204: Expand resources with `count:` or `for_each:`.
/// Runs after expand_recipes() and before build_execution_order().
pub fn expand_resources(config: &mut ForjarConfig) {
    // First pass: build a map of original ID → last expanded ID.
    let mut last_expanded: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for (id, resource) in &config.resources {
        if let Some(count) = resource.count {
            if count > 0 {
                last_expanded.insert(id.clone(), format!("{}-{}", id, count - 1));
            }
        } else if let Some(ref items) = resource.for_each {
            if let Some(last) = items.last() {
                last_expanded.insert(id.clone(), format!("{}-{}", id, last));
            }
        }
    }

    // Second pass: expand and rewrite deps.
    let mut expanded = indexmap::IndexMap::new();

    for (id, resource) in &config.resources {
        if let Some(count) = resource.count {
            for i in 0..count {
                let suffix = i.to_string();
                let new_id = format!("{}-{}", id, suffix);
                let mut cloned = resource.clone();
                cloned.count = None;
                replace_template_in_resource(&mut cloned, "{{index}}", &suffix);
                cloned.depends_on = rewrite_deps(&cloned.depends_on, &last_expanded);
                expanded.insert(new_id, cloned);
            }
        } else if let Some(ref items) = resource.for_each {
            let items = items.clone();
            for item in &items {
                let new_id = format!("{}-{}", id, item);
                let mut cloned = resource.clone();
                cloned.for_each = None;
                replace_template_in_resource(&mut cloned, "{{item}}", item);
                cloned.depends_on = rewrite_deps(&cloned.depends_on, &last_expanded);
                expanded.insert(new_id, cloned);
            }
        } else {
            let mut cloned = resource.clone();
            cloned.depends_on = rewrite_deps(&cloned.depends_on, &last_expanded);
            expanded.insert(id.clone(), cloned);
        }
    }

    config.resources = expanded;
}

/// Replace a template placeholder in all string fields of a resource.
fn replace_template_in_resource(resource: &mut Resource, placeholder: &str, value: &str) {
    // Path
    if let Some(ref mut path) = resource.path {
        *path = path.replace(placeholder, value);
    }
    // Content
    if let Some(ref mut content) = resource.content {
        *content = content.replace(placeholder, value);
    }
    // Name (service, pepita)
    if let Some(ref mut name) = resource.name {
        *name = name.replace(placeholder, value);
    }
    // Owner
    if let Some(ref mut owner) = resource.owner {
        *owner = owner.replace(placeholder, value);
    }
    // Source
    if let Some(ref mut source) = resource.source {
        *source = source.replace(placeholder, value);
    }
    // Target (symlink)
    if let Some(ref mut target) = resource.target {
        *target = target.replace(placeholder, value);
    }
    // Port (network)
    if let Some(ref mut port) = resource.port {
        *port = port.replace(placeholder, value);
    }
    // Packages
    resource.packages = resource
        .packages
        .iter()
        .map(|p| p.replace(placeholder, value))
        .collect();
}

/// Rewrite dependency references: if a dep points to an expanded resource
/// (one with count/for_each), replace it with the last expanded copy.
/// This ensures `depends_on: [shards]` becomes `depends_on: [shards-2]`.
fn rewrite_deps(
    deps: &[String],
    last_expanded: &std::collections::HashMap<String, String>,
) -> Vec<String> {
    deps.iter()
        .map(|dep| {
            last_expanded
                .get(dep)
                .cloned()
                .unwrap_or_else(|| dep.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj002_parse_valid() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#;
        let config = parse_config(yaml).unwrap();
        assert_eq!(config.name, "test");
        let errors = validate_config(&config);
        assert!(
            errors.is_empty(),
            "unexpected errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fj002_bad_version() {
        let yaml = r#"
version: "2.0"
name: test
machines: {}
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.message.contains("version")));
    }

    #[test]
    fn test_fj002_unknown_machine() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources:
  pkg:
    type: package
    machine: nonexistent
    provider: apt
    packages: [curl]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.message.contains("unknown machine")));
    }

    #[test]
    fn test_fj002_unknown_dependency() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
    depends_on: [ghost]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("unknown resource")));
    }

    #[test]
    fn test_fj002_self_dependency() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
    depends_on: [pkg]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("depends on itself")));
    }

    #[test]
    fn test_fj002_package_no_packages() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: []
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.message.contains("no packages")));
    }

    #[test]
    fn test_fj002_file_no_path() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  f:
    type: file
    machine: m1
    content: "hello"
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.message.contains("no path")));
    }

    #[test]
    fn test_fj035_file_content_and_source_exclusive() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  f:
    type: file
    machine: m1
    path: /etc/config
    content: "inline content"
    source: /local/path/config.txt
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("both content and source")));
    }

    #[test]
    fn test_fj002_parse_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("forjar.yaml");
        std::fs::write(
            &path,
            r#"
version: "1.0"
name: file-test
machines: {}
resources: {}
"#,
        )
        .unwrap();
        let config = parse_config_file(&path).unwrap();
        assert_eq!(config.name, "file-test");
    }

    #[test]
    fn test_fj002_parse_invalid_yaml() {
        let result = parse_config("not: [valid: yaml: {{");
        assert!(result.is_err());
    }

    #[test]
    fn test_fj002_empty_name() {
        let yaml = r#"
version: "1.0"
name: ""
machines: {}
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("name must not be empty")));
    }

    #[test]
    fn test_fj002_service_no_name() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  svc:
    type: service
    machine: m1
    state: running
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.message.contains("no name")));
    }

    #[test]
    fn test_fj002_package_no_provider() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    packages: [curl]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.message.contains("no provider")));
    }

    #[test]
    fn test_fj002_mount_no_source_or_path() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  mnt:
    type: mount
    machine: m1
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.message.contains("has no source")));
        assert!(errors.iter().any(|e| e.message.contains("has no path")));
    }

    #[test]
    fn test_fj002_validation_error_display() {
        let err = ValidationError {
            message: "test error".to_string(),
        };
        assert_eq!(format!("{}", err), "test error");
    }

    #[test]
    fn test_fj002_container_transport_requires_container_block() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("no 'container' block")));
    }

    #[test]
    fn test_fj002_container_ephemeral_requires_image() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      ephemeral: true
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("ephemeral but has no container image")));
    }

    #[test]
    fn test_fj002_container_invalid_runtime() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: lxc
      image: ubuntu:22.04
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("must be 'docker' or 'podman'")));
    }

    #[test]
    fn test_fj002_container_valid_config() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      ephemeral: true
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.is_empty(),
            "unexpected errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fj002_container_podman_valid() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: podman
      image: ubuntu:22.04
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.is_empty(),
            "unexpected errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fj002_parse_config_file_missing() {
        let result = parse_config_file(std::path::Path::new("/nonexistent/forjar.yaml"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("failed to read"));
    }

    /// BH-MUT-0001: Kill mutation of `machine_name != "localhost"`.
    #[test]
    fn test_fj002_user_no_name() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  u:
    type: user
    machine: m1
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("(user) has no name")));
    }

    #[test]
    fn test_fj002_docker_no_name() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  d:
    type: docker
    machine: m1
    image: nginx:latest
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("(docker) has no name")));
    }

    #[test]
    fn test_fj002_docker_no_image() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  d:
    type: docker
    machine: m1
    name: web
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("(docker) has no image")));
    }

    #[test]
    fn test_fj002_cron_no_schedule() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  c:
    type: cron
    machine: m1
    name: job
    command: /bin/true
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("(cron) has no schedule")));
    }

    #[test]
    fn test_fj002_cron_no_command() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  c:
    type: cron
    machine: m1
    name: job
    schedule: "0 * * * *"
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("(cron) has no command")));
    }

    #[test]
    fn test_fj002_network_no_port() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  fw:
    type: network
    machine: m1
    action: allow
    protocol: tcp
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("(network) has no port")));
    }

    #[test]
    fn test_fj002_user_valid() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  deploy-user:
    type: user
    machine: m1
    name: deploy
    shell: /bin/bash
    groups: [docker, sudo]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.is_empty(),
            "unexpected errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fj002_docker_valid() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  web:
    type: docker
    machine: m1
    name: web
    image: nginx:latest
    ports: ["8080:80"]
    environment: ["ENV=prod"]
    restart: unless-stopped
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.is_empty(),
            "unexpected errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    /// localhost should be accepted even when not in machines map.
    #[test]
    fn test_fj002_localhost_accepted_without_definition() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources:
  pkg:
    type: package
    machine: localhost
    provider: apt
    packages: [curl]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        // No "unknown machine" error for localhost
        assert!(
            !errors.iter().any(|e| e.message.contains("unknown machine")),
            "localhost should be accepted without explicit definition"
        );
    }

    // ── Recipe expansion integration tests ────────────────────

    #[test]
    fn test_expand_recipes_replaces_recipe_resources() {
        // Write a recipe file to a temp dir
        let dir = tempfile::tempdir().unwrap();
        let recipes_dir = dir.path().join("recipes");
        std::fs::create_dir_all(&recipes_dir).unwrap();
        std::fs::write(
            recipes_dir.join("test-recipe.yaml"),
            r#"
recipe:
  name: test-recipe
  inputs:
    greeting:
      type: string
      default: hello
resources:
  config-file:
    type: file
    path: /etc/test.conf
    content: "{{inputs.greeting}} world"
"#,
        )
        .unwrap();

        // Build a config with a recipe resource
        let yaml = r#"
version: "1.0"
name: recipe-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  setup:
    type: recipe
    machine: m1
    recipe: test-recipe
    inputs:
      greeting: hi
"#;
        let mut config = parse_config(yaml).unwrap();
        expand_recipes(&mut config, Some(dir.path())).unwrap();

        // Recipe resource should be replaced by expanded resources
        assert!(!config.resources.contains_key("setup"));
        assert!(config.resources.contains_key("setup/config-file"));

        let file_res = &config.resources["setup/config-file"];
        assert_eq!(file_res.resource_type, ResourceType::File);
        assert_eq!(file_res.content.as_deref(), Some("hi world"));
        assert_eq!(file_res.machine.to_vec(), vec!["m1"]);
    }

    #[test]
    fn test_expand_recipes_missing_recipe_file() {
        let dir = tempfile::tempdir().unwrap();
        let recipes_dir = dir.path().join("recipes");
        std::fs::create_dir_all(&recipes_dir).unwrap();

        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  setup:
    type: recipe
    machine: m1
    recipe: nonexistent
"#;
        let mut config = parse_config(yaml).unwrap();
        let result = expand_recipes(&mut config, Some(dir.path()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_expand_recipes_preserves_non_recipe_resources() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#;
        let mut config = parse_config(yaml).unwrap();
        expand_recipes(&mut config, Some(dir.path())).unwrap();

        // Non-recipe resources pass through unchanged
        assert!(config.resources.contains_key("pkg"));
        assert_eq!(config.resources.len(), 1);
    }

    #[test]
    fn test_expand_recipes_external_deps_propagated() {
        let dir = tempfile::tempdir().unwrap();
        let recipes_dir = dir.path().join("recipes");
        std::fs::create_dir_all(&recipes_dir).unwrap();
        std::fs::write(
            recipes_dir.join("dep-test.yaml"),
            r#"
recipe:
  name: dep-test
resources:
  first:
    type: package
    provider: apt
    packages: [nginx]
  second:
    type: file
    path: /etc/test
    content: test
    depends_on: [first]
"#,
        )
        .unwrap();

        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  base:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  my-recipe:
    type: recipe
    machine: m1
    recipe: dep-test
    depends_on:
      - base
"#;
        let mut config = parse_config(yaml).unwrap();
        expand_recipes(&mut config, Some(dir.path())).unwrap();

        assert_eq!(config.resources.len(), 3); // base + 2 expanded
        let first = &config.resources["my-recipe/first"];
        assert!(first.depends_on.contains(&"base".to_string()));

        let second = &config.resources["my-recipe/second"];
        assert!(second.depends_on.contains(&"my-recipe/first".to_string()));
        assert!(!second.depends_on.contains(&"base".to_string()));
    }

    // ── FJ-064: Cross-architecture tests ───────────────────────────

    #[test]
    fn test_fj064_resource_arch_filter_parsed() {
        let yaml = r#"
version: "1.0"
name: arch-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  gpu-driver:
    type: package
    machine: m1
    provider: apt
    packages: [nvidia-driver]
    arch: [x86_64]
"#;
        let config = parse_config(yaml).unwrap();
        assert_eq!(config.resources["gpu-driver"].arch, vec!["x86_64"]);
    }

    #[test]
    fn test_fj064_resource_arch_multi() {
        let yaml = r#"
version: "1.0"
name: arch-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  common:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
    arch: [x86_64, aarch64]
"#;
        let config = parse_config(yaml).unwrap();
        assert_eq!(config.resources["common"].arch, vec!["x86_64", "aarch64"]);
    }

    #[test]
    fn test_fj064_resource_arch_empty_default() {
        let yaml = r#"
version: "1.0"
name: arch-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#;
        let config = parse_config(yaml).unwrap();
        assert!(config.resources["pkg"].arch.is_empty());
    }

    #[test]
    fn test_fj064_invalid_resource_arch() {
        let yaml = r#"
version: "1.0"
name: arch-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
    arch: [sparc]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.message.contains("sparc")));
    }

    #[test]
    fn test_fj064_invalid_machine_arch() {
        let yaml = r#"
version: "1.0"
name: arch-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
    arch: mips
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.message.contains("mips")));
    }

    #[test]
    fn test_fj064_valid_machine_arch_aarch64() {
        let yaml = r#"
version: "1.0"
name: arch-test
machines:
  edge:
    hostname: jetson
    addr: 10.0.0.1
    arch: aarch64
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.is_empty(),
            "aarch64 should be valid, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_file_invalid_state() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  f:
    type: file
    machine: m1
    path: /tmp/x
    state: bogus
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("invalid state 'bogus'")));
    }

    #[test]
    fn test_file_symlink_requires_target() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  link:
    type: file
    machine: m1
    path: /usr/local/bin/tool
    state: symlink
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("symlink requires a target")));
    }

    #[test]
    fn test_file_valid_states() {
        for state in &["file", "directory", "symlink", "absent"] {
            let yaml = format!(
                r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  f:
    type: file
    machine: m1
    path: /tmp/x
    state: {}
    target: /tmp/y
"#,
                state
            );
            let config = parse_config(&yaml).unwrap();
            let errors = validate_config(&config);
            let state_errors: Vec<_> = errors
                .iter()
                .filter(|e| e.message.contains("invalid state"))
                .collect();
            assert!(state_errors.is_empty(), "state '{}' should be valid", state);
        }
    }

    #[test]
    fn test_service_invalid_state() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  svc:
    type: service
    machine: m1
    name: nginx
    state: restarting
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("invalid state 'restarting'")));
    }

    #[test]
    fn test_service_valid_states() {
        for state in &["running", "stopped", "enabled", "disabled"] {
            let yaml = format!(
                r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  svc:
    type: service
    machine: m1
    name: nginx
    state: {}
"#,
                state
            );
            let config = parse_config(&yaml).unwrap();
            let errors = validate_config(&config);
            let state_errors: Vec<_> = errors
                .iter()
                .filter(|e| e.message.contains("invalid state"))
                .collect();
            assert!(state_errors.is_empty(), "state '{}' should be valid", state);
        }
    }

    #[test]
    fn test_mount_invalid_state() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  mnt:
    type: mount
    machine: m1
    source: /dev/sda1
    path: /mnt/data
    state: attached
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("invalid state 'attached'")));
    }

    #[test]
    fn test_mount_missing_source_only() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  mnt:
    type: mount
    machine: m1
    path: /mnt/data
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.message.contains("has no source")));
        assert!(!errors.iter().any(|e| e.message.contains("has no path")));
    }

    #[test]
    fn test_network_invalid_protocol() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  fw:
    type: network
    machine: m1
    port: "22"
    protocol: sctp
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("invalid protocol 'sctp'")));
    }

    #[test]
    fn test_network_invalid_action() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  fw:
    type: network
    machine: m1
    port: "80"
    action: block
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("invalid action 'block'")));
    }

    #[test]
    fn test_docker_invalid_state() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  db:
    type: docker
    machine: m1
    name: postgres
    image: postgres:16
    state: paused
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("invalid state 'paused'")));
    }

    #[test]
    fn test_cron_schedule_must_have_5_fields() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  job:
    type: cron
    machine: m1
    name: bad-job
    schedule: "0 2 * *"
    command: /usr/bin/backup
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("must have exactly 5 fields")));
    }

    #[test]
    fn test_cron_valid_schedule() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  job:
    type: cron
    machine: m1
    name: good-job
    schedule: "0 2 * * *"
    command: /usr/bin/backup
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            !errors.iter().any(|e| e.message.contains("5 fields")),
            "valid 5-field schedule should pass"
        );
    }

    #[test]
    fn test_cron_invalid_state() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  job:
    type: cron
    machine: m1
    name: bad
    schedule: "* * * * *"
    command: echo hi
    state: disabled
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("invalid state 'disabled'")));
    }

    #[test]
    fn test_cron_absent_skips_schedule_and_command() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  old-job:
    type: cron
    machine: m1
    name: old-job
    state: absent
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        // state=absent should not require schedule or command
        assert!(
            !errors.iter().any(|e| e.message.contains("no schedule")),
            "absent cron should not require schedule"
        );
        assert!(
            !errors.iter().any(|e| e.message.contains("no command")),
            "absent cron should not require command"
        );
    }

    #[test]
    fn test_cron_schedule_too_many_fields() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  job:
    type: cron
    machine: m1
    name: bad-job
    schedule: "0 2 * * * *"
    command: echo hi
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("must have exactly 5 fields")));
    }

    // ── Edge-case / compound-validation tests ──────────────────────────

    #[test]
    fn test_fj002_deep_dependency_cycle_5_nodes() {
        // A→B→C→D→E→A: validate_resource_refs won't catch multi-hop cycles
        // (cycle detection happens in DAG sort), but self-dep and missing-dep
        // should still work. Here we verify no false-positive validation errors
        // when all refs are valid (cycle is a planning-time error, not parse-time).
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  a:
    type: file
    machine: m1
    path: /a
    depends_on: [b]
  b:
    type: file
    machine: m1
    path: /b
    depends_on: [c]
  c:
    type: file
    machine: m1
    path: /c
    depends_on: [d]
  d:
    type: file
    machine: m1
    path: /d
    depends_on: [e]
  e:
    type: file
    machine: m1
    path: /e
    depends_on: [a]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        // Parser validates refs, not cycles. All refs exist → no errors.
        assert!(
            errors.is_empty(),
            "cycle detection is planning-time, not parse-time: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fj002_diamond_dependency_valid() {
        // Diamond: A depends on B and C; B and C both depend on D.
        // This is valid and should produce no errors.
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  a:
    type: file
    machine: m1
    path: /a
    depends_on: [b, c]
  b:
    type: file
    machine: m1
    path: /b
    depends_on: [d]
  c:
    type: file
    machine: m1
    path: /c
    depends_on: [d]
  d:
    type: file
    machine: m1
    path: /d
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors.is_empty(), "diamond pattern is valid: {:?}", errors);
    }

    #[test]
    fn test_fj002_multiple_validation_errors_same_config() {
        // A config with many errors at once — all should be collected.
        let yaml = r#"
version: "2.0"
name: ""
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  bad-pkg:
    type: package
    machine: m1
  bad-file:
    type: file
    machine: m1
  bad-svc:
    type: service
    machine: m1
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        let msgs: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
        // version error
        assert!(msgs.iter().any(|m| m.contains("version must be")));
        // name error
        assert!(msgs.iter().any(|m| m.contains("name must not be empty")));
        // package missing packages + provider
        assert!(msgs.iter().any(|m| m.contains("no packages")));
        assert!(msgs.iter().any(|m| m.contains("no provider")));
        // file missing path
        assert!(msgs.iter().any(|m| m.contains("no path")));
        // service missing name
        assert!(msgs.iter().any(|m| m.contains("(service) has no name")));
        // At least 6 errors total
        assert!(
            errors.len() >= 6,
            "expected >= 6 errors, got {}",
            errors.len()
        );
    }

    #[test]
    fn test_fj002_docker_absent_skips_image_requirement() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  old-container:
    type: docker
    machine: m1
    name: old-container
    state: absent
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            !errors.iter().any(|e| e.message.contains("no image")),
            "docker state=absent should not require image"
        );
    }

    #[test]
    fn test_fj002_docker_running_requires_image() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  no-image:
    type: docker
    machine: m1
    name: no-image
    state: running
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.iter().any(|e| e.message.contains("no image")),
            "docker state=running must require image"
        );
    }

    #[test]
    fn test_fj002_mount_both_missing_gives_two_errors() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  bad-mount:
    type: mount
    machine: m1
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        let mount_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.message.contains("bad-mount"))
            .collect();
        assert!(
            mount_errors.iter().any(|e| e.message.contains("no source")),
            "should report missing source"
        );
        assert!(
            mount_errors.iter().any(|e| e.message.contains("no path")),
            "should report missing path"
        );
        assert!(
            mount_errors.len() >= 2,
            "mount with both missing should produce >=2 errors"
        );
    }

    #[test]
    fn test_fj002_network_reject_is_valid_action() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  fw-rule:
    type: network
    machine: m1
    port: 443
    protocol: tcp
    action: reject
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            !errors.iter().any(|e| e.message.contains("invalid action")),
            "'reject' should be a valid network action"
        );
    }

    #[test]
    fn test_fj002_network_invalid_protocol() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  fw:
    type: network
    machine: m1
    port: 80
    protocol: icmp
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("invalid protocol")));
    }

    #[test]
    fn test_fj002_recipe_missing_recipe_name() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  my-recipe:
    type: recipe
    machine: m1
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.iter().any(|e| e.message.contains("no recipe name")),
            "recipe without recipe field should error"
        );
    }

    #[test]
    fn test_fj002_unknown_arch_in_resource() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    packages: [vim]
    provider: apt
    arch: [mips64]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.iter().any(|e| e.message.contains("unknown arch")),
            "mips64 should be an unknown arch"
        );
    }

    #[test]
    fn test_fj002_unknown_arch_in_machine() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
    arch: sparc64
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.iter().any(|e| e.message.contains("unknown arch")),
            "sparc64 should be an unknown machine arch"
        );
    }

    #[test]
    fn test_fj002_container_transport_missing_block() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("no 'container' block")),
            "container transport without container block should error"
        );
    }

    #[test]
    fn test_fj002_container_runtime_containerd_rejected() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: containerd
      image: ubuntu:22.04
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("must be 'docker' or 'podman'")));
    }

    #[test]
    fn test_fj002_container_ephemeral_no_image() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      ephemeral: true
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("ephemeral but has no container image")),
            "ephemeral container without image should error"
        );
    }

    #[test]
    fn test_fj002_self_dependency_detected() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  loopy:
    type: file
    machine: m1
    path: /etc/loopy
    depends_on: [loopy]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("depends on itself")),
            "self-dependency should be caught"
        );
    }

    #[test]
    fn test_fj002_depends_on_unknown_resource() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  web:
    type: file
    machine: m1
    path: /etc/nginx.conf
    depends_on: [ghost-resource]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("unknown resource 'ghost-resource'")));
    }

    #[test]
    fn test_fj002_file_both_content_and_source() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  both:
    type: file
    machine: m1
    path: /etc/both
    content: "hello"
    source: ./local.txt
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(errors
            .iter()
            .any(|e| e.message.contains("both content and source")));
    }

    #[test]
    fn test_fj002_file_symlink_without_target() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  link:
    type: file
    machine: m1
    path: /usr/local/bin/myapp
    state: symlink
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("symlink requires a target")),
            "symlink without target should error"
        );
    }

    #[test]
    fn test_fj002_all_valid_arch_values_accepted() {
        for arch in &["x86_64", "aarch64", "armv7l", "riscv64", "s390x", "ppc64le"] {
            let yaml = format!(
                r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
    arch: {}
resources: {{}}
"#,
                arch
            );
            let config = parse_config(&yaml).unwrap();
            let errors = validate_config(&config);
            assert!(
                errors.is_empty(),
                "arch '{}' should be valid but got errors: {:?}",
                arch,
                errors.iter().map(|e| &e.message).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_fj002_localhost_machine_ref_always_valid() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  local-file:
    type: file
    machine: localhost
    path: /tmp/local
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            !errors.iter().any(|e| e.message.contains("unknown machine")),
            "'localhost' should be accepted without being in machines map"
        );
    }

    #[test]
    fn test_fj002_service_invalid_state() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  svc:
    type: service
    machine: m1
    name: nginx
    state: restarted
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.iter().any(|e| e.message.contains("invalid state")),
            "'restarted' is not a valid service state"
        );
    }

    #[test]
    fn test_fj002_mount_invalid_state() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  mnt:
    type: mount
    machine: m1
    source: /dev/sda1
    path: /mnt/data
    state: enabled
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.iter().any(|e| e.message.contains("invalid state")),
            "'enabled' is not a valid mount state"
        );
    }

    #[test]
    fn test_fj002_cron_invalid_state() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  job:
    type: cron
    machine: m1
    name: bad-job
    schedule: "0 2 * * *"
    command: echo hi
    state: running
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.iter().any(|e| e.message.contains("invalid state")),
            "'running' is not a valid cron state"
        );
    }

    #[test]
    fn test_fj002_docker_invalid_state() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  c:
    type: docker
    machine: m1
    name: c
    image: nginx
    state: paused
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.iter().any(|e| e.message.contains("invalid state")),
            "'paused' is not a valid docker state"
        );
    }

    // ── FJ-131: Parser edge case tests ────────────────────────────

    #[test]
    fn test_fj131_parse_and_validate_valid_config() {
        // parse_and_validate happy path with a valid config file
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: valid-config
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/test.txt
    content: "hello"
"#,
        )
        .unwrap();

        let config = parse_and_validate(&config_path).unwrap();
        assert_eq!(config.name, "valid-config");
        assert!(config.resources.contains_key("test-file"));
    }

    #[test]
    fn test_fj131_parse_and_validate_error_formatting() {
        // parse_and_validate with multiple validation errors — verify error format
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: bad-config
machines: {}
resources:
  bad-pkg:
    type: package
    machine: unknown-machine
  bad-file:
    type: file
    machine: another-unknown
"#,
        )
        .unwrap();

        let err = parse_and_validate(&config_path).unwrap_err();
        assert!(
            err.starts_with("validation errors:\n"),
            "error should start with 'validation errors:'"
        );
        assert!(
            err.contains("  - "),
            "each error should be indented with '  - '"
        );
        // Should have multiple errors
        let bullet_count = err.matches("  - ").count();
        assert!(
            bullet_count >= 2,
            "expected multiple errors, got {} bullets",
            bullet_count
        );
    }

    #[test]
    fn test_fj131_parse_and_validate_nonexistent_file() {
        let result = parse_and_validate(Path::new("/tmp/nonexistent-forjar-config.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_fj131_package_both_missing_provider_and_packages() {
        // Package with BOTH missing provider AND empty packages — should produce 2 errors
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  bad-pkg:
    type: package
    machine: m
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        let pkg_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.message.contains("bad-pkg"))
            .collect();
        assert!(
            pkg_errors.len() >= 2,
            "should have at least 2 errors for missing packages AND provider, got {}",
            pkg_errors.len()
        );
        assert!(pkg_errors.iter().any(|e| e.message.contains("no packages")));
        assert!(pkg_errors.iter().any(|e| e.message.contains("no provider")));
    }

    #[test]
    fn test_fj131_file_invalid_state_error_lists_valid_options() {
        // Verify the error message includes the list of valid states
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  bad-file:
    type: file
    machine: m
    path: /tmp/test
    state: "executable"
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        let file_err = errors
            .iter()
            .find(|e| e.message.contains("invalid state"))
            .expect("should have invalid state error");
        // Error message should list all valid states
        assert!(file_err
            .message
            .contains("file, directory, symlink, absent"));
    }

    #[test]
    fn test_fj131_container_valid_config_no_errors() {
        // Container machine with valid config should produce zero errors
        let yaml = r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: podman
      image: ubuntu:22.04
      ephemeral: false
      name: my-container
resources:
  f:
    type: file
    machine: test-box
    path: /tmp/test
    content: "hello"
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.is_empty(),
            "valid container config should have no errors: {:?}",
            errors
        );
    }

    #[test]
    fn test_fj131_service_invalid_state_lists_valid_options() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  svc:
    type: service
    machine: m
    name: nginx
    state: "paused"
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        let svc_err = errors
            .iter()
            .find(|e| e.message.contains("invalid state"))
            .expect("should have invalid state error");
        assert!(svc_err
            .message
            .contains("running, stopped, enabled, disabled"));
    }

    #[test]
    fn test_fj131_mount_invalid_state_lists_valid_options() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  mnt:
    type: mount
    machine: m
    source: /dev/sda1
    path: /mnt/data
    state: "bound"
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        let mnt_err = errors
            .iter()
            .find(|e| e.message.contains("invalid state"))
            .expect("should have invalid state error");
        assert!(mnt_err.message.contains("mounted, unmounted, absent"));
    }

    #[test]
    fn test_fj131_cron_both_missing_schedule_and_command() {
        // Cron with both missing schedule AND command (not absent) — should produce 3 errors
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  bad-cron:
    type: cron
    machine: m
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        let cron_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.message.contains("bad-cron"))
            .collect();
        // Should have: no name, no schedule, no command = 3 errors
        assert!(
            cron_errors.len() >= 3,
            "expected at least 3 cron errors, got {}",
            cron_errors.len()
        );
    }

    #[test]
    fn test_fj131_network_both_invalid_protocol_and_action() {
        // Network with invalid protocol AND invalid action — should produce 2 errors
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  bad-net:
    type: network
    machine: m
    port: "22"
    protocol: icmp
    action: forward
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        let net_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.message.contains("bad-net"))
            .collect();
        assert!(
            net_errors.len() >= 2,
            "expected at least 2 network errors, got {}",
            net_errors.len()
        );
        assert!(net_errors.iter().any(|e| e.message.contains("protocol")));
        assert!(net_errors.iter().any(|e| e.message.contains("action")));
    }

    #[test]
    fn test_fj131_parse_and_validate_with_recipe_expansion() {
        // parse_and_validate should expand recipes
        let dir = tempfile::tempdir().unwrap();

        // Write recipe file at recipes/web.yaml (relative to config dir)
        let recipes_dir = dir.path().join("recipes");
        std::fs::create_dir_all(&recipes_dir).unwrap();
        std::fs::write(
            recipes_dir.join("web.yaml"),
            r#"
recipe:
  name: web-recipe
resources:
  web-file:
    type: file
    path: /tmp/web.txt
    content: "web"
"#,
        )
        .unwrap();

        // Write config referencing the recipe by name (not path)
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: recipe-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  web:
    type: recipe
    machine: local
    recipe: web
"#,
        )
        .unwrap();

        let config = parse_and_validate(&config_path).unwrap();
        // Recipe should be expanded — "web" replaced with "web/web-file"
        assert!(
            !config.resources.contains_key("web"),
            "recipe resource should be replaced"
        );
        assert!(
            config.resources.keys().any(|k| k.contains("web-file")),
            "expanded resource should be present"
        );
    }

    #[test]
    fn test_fj132_validate_container_no_block() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  box:
    hostname: box
    addr: container
    transport: container
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("no 'container' block")),
            "container transport without container block should error"
        );
    }

    #[test]
    fn test_fj132_validate_container_bad_runtime() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  box:
    hostname: box
    addr: container
    transport: container
    container:
      runtime: lxc
      image: ubuntu:22.04
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("docker' or 'podman'")),
            "invalid runtime should error"
        );
    }

    #[test]
    fn test_fj132_validate_ephemeral_no_image() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  box:
    hostname: box
    addr: container
    transport: container
    container:
      runtime: docker
      ephemeral: true
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("ephemeral but has no container image")),
            "ephemeral without image should error"
        );
    }

    #[test]
    fn test_fj132_validate_file_both_content_and_source() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  bad-file:
    type: file
    machine: m
    path: /etc/test.conf
    content: "inline"
    source: /builds/app.conf
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("both content and source")),
            "file with both content and source should error"
        );
    }

    #[test]
    fn test_fj132_validate_symlink_no_target() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  bad-link:
    type: file
    machine: m
    path: /usr/local/bin/myapp
    state: symlink
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("symlink requires a target")),
            "symlink without target should error"
        );
    }

    #[test]
    fn test_fj132_validate_unknown_arch() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
    arch: mips64
resources: {}
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.iter().any(|e| e.message.contains("unknown arch")),
            "unknown architecture should error"
        );
    }

    #[test]
    fn test_fj132_validate_service_invalid_state() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  svc:
    type: service
    machine: m
    name: nginx
    state: restarted
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("invalid state 'restarted'")),
            "invalid service state should error"
        );
    }

    #[test]
    fn test_fj132_parse_config_invalid_yaml() {
        let result = parse_config("{{{{bad yaml");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("YAML parse error"));
    }

    // ── FJ-036 tests ────────────────────────────────────────────

    #[test]
    fn test_fj036_parse_minimal_config() {
        let yaml = r#"
version: "1.0"
name: minimal
machines:
  m1:
    hostname: box
    addr: 10.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#;
        let config = parse_config(yaml).unwrap();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.name, "minimal");
        assert_eq!(config.machines.len(), 1);
        assert!(config.machines.contains_key("m1"));
        assert_eq!(config.resources.len(), 1);
        assert!(config.resources.contains_key("pkg"));
        let errors = validate_config(&config);
        assert!(
            errors.is_empty(),
            "minimal valid config should have no errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fj036_parse_multiple_machines() {
        let yaml = r#"
version: "1.0"
name: multi-machine
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
  db:
    hostname: db-01
    addr: 10.0.0.2
  cache:
    hostname: cache-01
    addr: 10.0.0.3
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
  cache-pkg:
    type: package
    machine: cache
    provider: apt
    packages: [redis-server]
"#;
        let config = parse_config(yaml).unwrap();
        assert_eq!(config.machines.len(), 3);
        assert!(config.machines.contains_key("web"));
        assert!(config.machines.contains_key("db"));
        assert!(config.machines.contains_key("cache"));
        assert_eq!(config.machines["web"].hostname, "web-01");
        assert_eq!(config.machines["db"].hostname, "db-01");
        assert_eq!(config.machines["cache"].hostname, "cache-01");
        let errors = validate_config(&config);
        assert!(
            errors.is_empty(),
            "multi-machine config should validate: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fj036_validate_duplicate_depends() {
        let yaml = r#"
version: "1.0"
name: self-dep
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  circular:
    type: file
    machine: m1
    path: /etc/circular.conf
    content: "loop"
    depends_on: [circular]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("depends on itself")),
            "resource depending on itself should produce error, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_fj036_parse_with_all_resource_types() {
        let yaml = r#"
version: "1.0"
name: all-types
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  conf:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "key=value"
  svc:
    type: service
    machine: m1
    name: nginx
    state: running
  mnt:
    type: mount
    machine: m1
    source: /dev/sda1
    path: /mnt/data
  deploy-user:
    type: user
    machine: m1
    name: deploy
  web-container:
    type: docker
    machine: m1
    name: web
    image: nginx:latest
  backup-job:
    type: cron
    machine: m1
    name: backup
    schedule: "0 2 * * *"
    command: /usr/bin/backup
  firewall:
    type: network
    machine: m1
    port: "443"
    protocol: tcp
    action: allow
  sandbox:
    type: pepita
    machine: m1
    name: sandbox
    state: present
"#;
        let config = parse_config(yaml).unwrap();
        assert_eq!(config.resources.len(), 9);
        let errors = validate_config(&config);
        assert!(
            errors.is_empty(),
            "config with all 9 resource types should validate: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
        // Verify each resource type parsed correctly
        assert_eq!(config.resources["pkg"].resource_type, ResourceType::Package);
        assert_eq!(config.resources["conf"].resource_type, ResourceType::File);
        assert_eq!(config.resources["svc"].resource_type, ResourceType::Service);
        assert_eq!(config.resources["mnt"].resource_type, ResourceType::Mount);
        assert_eq!(
            config.resources["deploy-user"].resource_type,
            ResourceType::User
        );
        assert_eq!(
            config.resources["web-container"].resource_type,
            ResourceType::Docker
        );
        assert_eq!(
            config.resources["backup-job"].resource_type,
            ResourceType::Cron
        );
        assert_eq!(
            config.resources["firewall"].resource_type,
            ResourceType::Network
        );
        assert_eq!(
            config.resources["sandbox"].resource_type,
            ResourceType::Pepita
        );
    }

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
        assert!(errors.is_empty(), "validation errors: {:?}", errors);
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
        assert!(errors.is_empty(), "validation errors: {:?}", errors);
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

        // base is preserved, shards expanded to 2
        assert_eq!(config.resources.len(), 3);
        assert!(config.resources.contains_key("base"));
        assert!(config.resources.contains_key("shards-0"));
        assert!(config.resources.contains_key("shards-1"));
        // depends_on preserved
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
        assert!(errors.is_empty(), "validation errors: {:?}", errors);
        expand_resources(&mut config);

        // reader's dep should be rewritten to shards-2 (last expanded copy)
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
        // setup's dep should be rewritten to homes-bob (last item)
        assert_eq!(config.resources["setup"].depends_on, vec!["homes-bob"]);
    }

    // ================================================================
    // FJ-220: Policy evaluation tests
    // ================================================================

    #[test]
    fn test_fj220_policy_require_field_pass() {
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
    path: /etc/app.conf
    owner: noah
    mode: "0644"
policies:
  - type: require
    message: "files must have owner"
    resource_type: file
    field: owner
"#;
        let config = parse_config(yaml).unwrap();
        let violations = evaluate_policies(&config);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_fj220_policy_require_field_fail() {
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
    path: /etc/app.conf
policies:
  - type: require
    message: "files must have owner"
    resource_type: file
    field: owner
"#;
        let config = parse_config(yaml).unwrap();
        let violations = evaluate_policies(&config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].resource_id, "cfg");
        assert_eq!(violations[0].severity, PolicyRuleType::Require);
    }

    #[test]
    fn test_fj220_policy_deny_condition() {
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
    path: /etc/app.conf
    owner: root
policies:
  - type: deny
    message: "files must not be owned by root"
    resource_type: file
    condition_field: owner
    condition_value: root
"#;
        let config = parse_config(yaml).unwrap();
        let violations = evaluate_policies(&config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].severity, PolicyRuleType::Deny);
    }

    #[test]
    fn test_fj220_policy_warn_only() {
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
    path: /etc/app.conf
    owner: root
policies:
  - type: warn
    message: "files should not be owned by root"
    resource_type: file
    condition_field: owner
    condition_value: root
"#;
        let config = parse_config(yaml).unwrap();
        let violations = evaluate_policies(&config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].severity, PolicyRuleType::Warn);
    }

    #[test]
    fn test_fj220_policy_type_filter() {
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
    packages: [curl]
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
policies:
  - type: require
    message: "files must have owner"
    resource_type: file
    field: owner
"#;
        let config = parse_config(yaml).unwrap();
        let violations = evaluate_policies(&config);
        // Only file resource should be checked, not package
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].resource_id, "cfg");
    }

    #[test]
    fn test_fj220_policy_tag_filter() {
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
    path: /etc/app.conf
    tags: [critical]
  log:
    type: file
    machine: m1
    path: /var/log/app.log
policies:
  - type: require
    message: "critical files must have owner"
    tag: critical
    field: owner
"#;
        let config = parse_config(yaml).unwrap();
        let violations = evaluate_policies(&config);
        // Only cfg (tagged critical) should trigger, not log
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].resource_id, "cfg");
    }

    #[test]
    fn test_fj220_policy_multiple_rules() {
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
    path: /etc/app.conf
    owner: root
policies:
  - type: require
    message: "files must have mode"
    resource_type: file
    field: mode
  - type: deny
    message: "no root owner"
    resource_type: file
    condition_field: owner
    condition_value: root
"#;
        let config = parse_config(yaml).unwrap();
        let violations = evaluate_policies(&config);
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn test_fj220_no_policies() {
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
    path: /etc/app.conf
"#;
        let config = parse_config(yaml).unwrap();
        let violations = evaluate_policies(&config);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_fj220_require_tags() {
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
    path: /etc/app.conf
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
    tags: [infra]
policies:
  - type: require
    message: "all resources must have tags"
    field: tags
"#;
        let config = parse_config(yaml).unwrap();
        let violations = evaluate_policies(&config);
        // cfg has no tags, pkg has tags
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].resource_id, "cfg");
    }

    #[test]
    fn test_fj224_triggers_valid() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  config:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "hello"
  app:
    type: service
    machine: m1
    name: app
    depends_on: [config]
    triggers: [config]
"#;
        let config = parse_config(yaml).unwrap();
        assert!(config.resources["app"]
            .triggers
            .contains(&"config".to_string()));
    }

    #[test]
    fn test_fj224_triggers_unknown_resource() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  app:
    type: service
    machine: m1
    name: app
    triggers: [ghost-resource]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("triggers on unknown resource")),
            "errors: {:?}",
            errors
        );
    }

    #[test]
    fn test_fj224_triggers_self_reference() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/app
    content: "x"
    triggers: [app]
"#;
        let config = parse_config(yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("triggers on itself")),
            "errors: {:?}",
            errors
        );
    }

    #[test]
    fn test_fj224_empty_triggers() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/app
    content: "hello"
"#;
        let config = parse_config(yaml).unwrap();
        assert!(config.resources["app"].triggers.is_empty());
    }
}
