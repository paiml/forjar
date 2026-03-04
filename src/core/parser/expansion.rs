//! FJ-203/FJ-204: Expand resources with `count:` or `for_each:`.

use super::*;

/// Build the map of original resource ID → last expanded ID.
fn build_last_expanded_map(
    resources: &indexmap::IndexMap<String, Resource>,
) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for (id, resource) in resources {
        if let Some(count) = resource.count {
            if count > 0 {
                map.insert(id.clone(), format!("{}-{}", id, count - 1));
            }
        } else if let Some(ref items) = resource.for_each {
            if let Some(last) = items.last() {
                map.insert(id.clone(), format!("{id}-{last}"));
            }
        }
    }
    map
}

/// Expand a single resource with `count:` into N copies.
fn expand_count(
    id: &str,
    resource: &Resource,
    count: u32,
    last_expanded: &std::collections::HashMap<String, String>,
    out: &mut indexmap::IndexMap<String, Resource>,
) {
    for i in 0..count {
        let suffix = i.to_string();
        let mut cloned = resource.clone();
        cloned.count = None;
        replace_template_in_resource(&mut cloned, "{{index}}", &suffix);
        cloned.depends_on = rewrite_deps(&cloned.depends_on, last_expanded);
        out.insert(format!("{id}-{suffix}"), cloned);
    }
}

/// Expand a single resource with `for_each:` into one copy per item.
fn expand_for_each(
    id: &str,
    resource: &Resource,
    items: &[String],
    last_expanded: &std::collections::HashMap<String, String>,
    out: &mut indexmap::IndexMap<String, Resource>,
) {
    for item in items {
        let mut cloned = resource.clone();
        cloned.for_each = None;
        replace_template_in_resource(&mut cloned, "{{item}}", item);
        cloned.depends_on = rewrite_deps(&cloned.depends_on, last_expanded);
        out.insert(format!("{id}-{item}"), cloned);
    }
}

/// FJ-203/FJ-204: Expand resources with `count:` or `for_each:`.
/// Runs after expand_recipes() and before build_execution_order().
pub fn expand_resources(config: &mut ForjarConfig) {
    let last_expanded = build_last_expanded_map(&config.resources);
    let mut expanded = indexmap::IndexMap::new();

    for (id, resource) in &config.resources {
        if let Some(count) = resource.count {
            expand_count(id, resource, count, &last_expanded, &mut expanded);
        } else if let Some(ref items) = resource.for_each {
            expand_for_each(id, resource, &items.clone(), &last_expanded, &mut expanded);
        } else {
            let mut cloned = resource.clone();
            cloned.depends_on = rewrite_deps(&cloned.depends_on, &last_expanded);
            expanded.insert(id.clone(), cloned);
        }
    }

    config.resources = expanded;
}

/// Replace a placeholder in an optional string field.
fn replace_in_opt(field: &mut Option<String>, placeholder: &str, value: &str) {
    if let Some(ref mut s) = field {
        *s = s.replace(placeholder, value);
    }
}

/// Replace a placeholder in a list of strings.
fn replace_in_list(list: &mut Vec<String>, placeholder: &str, value: &str) {
    *list = list.iter().map(|s| s.replace(placeholder, value)).collect();
}

/// Replace a template placeholder in all string fields of a resource.
fn replace_template_in_resource(resource: &mut Resource, placeholder: &str, value: &str) {
    replace_in_opt(&mut resource.path, placeholder, value);
    replace_in_opt(&mut resource.content, placeholder, value);
    replace_in_opt(&mut resource.name, placeholder, value);
    replace_in_opt(&mut resource.owner, placeholder, value);
    replace_in_opt(&mut resource.source, placeholder, value);
    replace_in_opt(&mut resource.target, placeholder, value);
    replace_in_opt(&mut resource.port, placeholder, value);
    replace_in_opt(&mut resource.command, placeholder, value);
    replace_in_opt(&mut resource.working_dir, placeholder, value);
    replace_in_opt(&mut resource.completion_check, placeholder, value);
    replace_in_opt(&mut resource.pre_apply, placeholder, value);
    replace_in_opt(&mut resource.post_apply, placeholder, value);
    replace_in_opt(&mut resource.schedule, placeholder, value);
    replace_in_opt(&mut resource.script, placeholder, value);
    replace_in_list(&mut resource.packages, placeholder, value);
    replace_in_list(&mut resource.output_artifacts, placeholder, value);
    replace_in_list(&mut resource.depends_on, placeholder, value);
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
