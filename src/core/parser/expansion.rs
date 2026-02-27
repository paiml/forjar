//! FJ-203/FJ-204: Expand resources with `count:` or `for_each:`.

use super::*;

/// FJ-203/FJ-204: Expand resources with `count:` or `for_each:`.
/// Runs after expand_recipes() and before build_execution_order().
pub fn expand_resources(config: &mut ForjarConfig) {
    // First pass: build a map of original ID -> last expanded ID.
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
