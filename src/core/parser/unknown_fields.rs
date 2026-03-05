//! FJ-2500: Unknown field detection with "did you mean?" suggestions.
//!
//! Two-pass parsing: after serde deserializes into typed structs, we parse the
//! raw YAML as `Value` and walk keys against known field sets. Unknown fields
//! produce warnings with Levenshtein-based suggestions.

use super::ValidationError;

/// An unknown field detected in the YAML.
#[derive(Debug, Clone)]
pub struct UnknownField {
    /// Dot-separated path (e.g., "resources.pkg.packges")
    pub path: String,
    /// The unknown key
    pub key: String,
    /// Closest known field if Levenshtein distance <= 2
    pub suggestion: Option<String>,
}

impl std::fmt::Display for UnknownField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref suggestion) = self.suggestion {
            write!(
                f,
                "unknown field '{}' at '{}' — did you mean '{suggestion}'?",
                self.key, self.path
            )
        } else {
            write!(f, "unknown field '{}' at '{}'", self.key, self.path)
        }
    }
}

// Known YAML keys for each struct (using serde rename where applicable).
const CONFIG_FIELDS: &[&str] = &[
    "version",
    "name",
    "description",
    "params",
    "machines",
    "resources",
    "policy",
    "outputs",
    "policies",
    "data",
    "includes",
    "checks",
    "moved",
];

const RESOURCE_FIELDS: &[&str] = &[
    "type",
    "machine",
    "state",
    "depends_on",
    "provider",
    "packages",
    "version",
    "path",
    "content",
    "source",
    "target",
    "owner",
    "group",
    "mode",
    "name",
    "enabled",
    "restart_on",
    "triggers",
    "fstype",
    "options",
    "uid",
    "shell",
    "home",
    "groups",
    "ssh_authorized_keys",
    "system_user",
    "schedule",
    "command",
    "image",
    "ports",
    "environment",
    "volumes",
    "restart",
    "protocol",
    "port",
    "action",
    "from",
    "recipe",
    "inputs",
    "arch",
    "tags",
    "resource_group",
    "when",
    "count",
    "for_each",
    "chroot_dir",
    "namespace_uid",
    "namespace_gid",
    "seccomp",
    "netns",
    "cpuset",
    "memory_limit",
    "overlay_lower",
    "overlay_upper",
    "overlay_work",
    "overlay_merged",
    "format",
    "quantization",
    "checksum",
    "cache_dir",
    "gpu_backend",
    "driver_version",
    "cuda_version",
    "rocm_version",
    "devices",
    "persistence_mode",
    "compute_mode",
    "gpu_memory_limit_mb",
    "task_mode",
    "task_inputs",
    "output_artifacts",
    "completion_check",
    "timeout",
    "working_dir",
    "stages",
    "cache",
    "gpu_device",
    "restart_delay",
    "pre_apply",
    "post_apply",
    "lifecycle",
    "sudo",
    "store",
    "script",
];

const MACHINE_FIELDS: &[&str] = &[
    "hostname",
    "addr",
    "user",
    "arch",
    "ssh_key",
    "roles",
    "transport",
    "container",
    "pepita",
    "cost",
];

const POLICY_FIELDS: &[&str] = &[
    "failure",
    "parallel_machines",
    "tripwire",
    "lock_file",
    "parallel_resources",
    "pre_apply",
    "post_apply",
    "serial",
    "max_fail_percentage",
    "ssh_retries",
    "convergence_budget",
    "snapshot_generations",
    "security_gate",
    "notify",
];

const NOTIFY_FIELDS: &[&str] = &["on_success", "on_failure", "on_drift"];

const CONTAINER_FIELDS: &[&str] = &[
    "runtime",
    "image",
    "name",
    "ephemeral",
    "privileged",
    "init",
    "gpus",
    "devices",
    "group_add",
    "env",
    "volumes",
];

const PEPITA_FIELDS: &[&str] = &[
    "rootfs",
    "memory_mb",
    "cpus",
    "network",
    "filesystem",
    "ephemeral",
];

const DATASOURCE_FIELDS: &[&str] = &[
    "type",
    "value",
    "default",
    "state_dir",
    "config",
    "outputs",
    "max_staleness",
];

const POLICY_RULE_FIELDS: &[&str] = &[
    "type",
    "message",
    "resource_type",
    "tag",
    "field",
    "condition_field",
    "condition_value",
];

const OUTPUT_FIELDS: &[&str] = &["value", "description"];

const CHECK_FIELDS: &[&str] = &["machine", "command", "expect_exit", "description"];

const MOVED_FIELDS: &[&str] = &["from", "to"];

const LIFECYCLE_FIELDS: &[&str] = &["prevent_destroy", "create_before_destroy", "ignore_drift"];

// -- Recipe known fields --

const RECIPE_FILE_FIELDS: &[&str] = &["recipe", "resources"];

const RECIPE_META_FIELDS: &[&str] = &["name", "version", "description", "inputs", "requires"];

const RECIPE_INPUT_FIELDS: &[&str] = &["type", "description", "default", "min", "max", "choices"];

const RECIPE_REQUIREMENT_FIELDS: &[&str] = &["recipe"];

/// Detect unknown fields in raw YAML by comparing against known field sets.
pub fn detect_unknown_fields(yaml: &str) -> Result<Vec<UnknownField>, String> {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(yaml).map_err(|e| format!("YAML parse error: {e}"))?;

    let mut unknowns = Vec::new();

    let mapping = match value.as_mapping() {
        Some(m) => m,
        None => return Ok(unknowns), // Not a mapping at top level — serde will catch this
    };

    for (key, val) in mapping {
        let key_str = yaml_key_str(key);
        if !CONFIG_FIELDS.contains(&key_str.as_str()) {
            unknowns.push(make_unknown(&key_str, &key_str, CONFIG_FIELDS));
            continue;
        }
        // Recurse into known nested structures
        match key_str.as_str() {
            "machines" => check_named_map(val, "machines", MACHINE_FIELDS, &mut unknowns),
            "resources" => check_named_map(val, "resources", RESOURCE_FIELDS, &mut unknowns),
            "policy" => check_mapping(val, "policy", POLICY_FIELDS, &mut unknowns),
            "data" => check_named_map(val, "data", DATASOURCE_FIELDS, &mut unknowns),
            "outputs" => check_named_map(val, "outputs", OUTPUT_FIELDS, &mut unknowns),
            "checks" => check_named_map(val, "checks", CHECK_FIELDS, &mut unknowns),
            "policies" => check_list(val, "policies", POLICY_RULE_FIELDS, &mut unknowns),
            "moved" => check_list(val, "moved", MOVED_FIELDS, &mut unknowns),
            _ => {}
        }
    }
    Ok(unknowns)
}

/// Detect unknown fields in a recipe YAML file.
pub fn detect_unknown_recipe_fields(yaml: &str) -> Result<Vec<UnknownField>, String> {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(yaml).map_err(|e| format!("YAML parse error: {e}"))?;

    let mut unknowns = Vec::new();

    let mapping = match value.as_mapping() {
        Some(m) => m,
        None => return Ok(unknowns),
    };

    for (key, val) in mapping {
        let key_str = yaml_key_str(key);
        if !RECIPE_FILE_FIELDS.contains(&key_str.as_str()) {
            unknowns.push(make_unknown(&key_str, &key_str, RECIPE_FILE_FIELDS));
            continue;
        }
        match key_str.as_str() {
            "recipe" => check_recipe_meta(val, "recipe", &mut unknowns),
            "resources" => check_named_map(val, "resources", RESOURCE_FIELDS, &mut unknowns),
            _ => {}
        }
    }
    Ok(unknowns)
}

/// Check recipe metadata fields including nested inputs and requires.
fn check_recipe_meta(val: &serde_yaml_ng::Value, path: &str, unknowns: &mut Vec<UnknownField>) {
    let mapping = match val.as_mapping() {
        Some(m) => m,
        None => return,
    };
    for (key, child) in mapping {
        let key_str = yaml_key_str(key);
        let full = format!("{path}.{key_str}");
        if !RECIPE_META_FIELDS.contains(&key_str.as_str()) {
            unknowns.push(make_unknown(&full, &key_str, RECIPE_META_FIELDS));
            continue;
        }
        match key_str.as_str() {
            "inputs" => check_named_map(child, &full, RECIPE_INPUT_FIELDS, unknowns),
            "requires" => check_list(child, &full, RECIPE_REQUIREMENT_FIELDS, unknowns),
            _ => {}
        }
    }
}

/// Convert unknown fields into validation errors.
pub fn unknown_fields_to_errors(unknowns: &[UnknownField]) -> Vec<ValidationError> {
    unknowns
        .iter()
        .map(|u| ValidationError {
            message: u.to_string(),
        })
        .collect()
}

// -- Internal helpers --

/// Check a mapping where each value is a named entry (e.g., machines.web, resources.pkg).
fn check_named_map(
    val: &serde_yaml_ng::Value,
    parent: &str,
    known: &[&str],
    unknowns: &mut Vec<UnknownField>,
) {
    let mapping = match val.as_mapping() {
        Some(m) => m,
        None => return,
    };
    for (name_key, entry_val) in mapping {
        let name = yaml_key_str(name_key);
        let path = format!("{parent}.{name}");
        check_mapping(entry_val, &path, known, unknowns);
    }
}

/// Check a single mapping's keys against known fields.
fn check_mapping(
    val: &serde_yaml_ng::Value,
    path: &str,
    known: &[&str],
    unknowns: &mut Vec<UnknownField>,
) {
    let mapping = match val.as_mapping() {
        Some(m) => m,
        None => return,
    };
    for (key, child) in mapping {
        let key_str = yaml_key_str(key);
        let full = format!("{path}.{key_str}");
        if !known.contains(&key_str.as_str()) {
            unknowns.push(make_unknown(&full, &key_str, known));
        } else {
            // Recurse into known nested structures
            check_nested(&key_str, child, path, unknowns);
        }
    }
}

/// Check a sequence of mappings (e.g., policies: [list of PolicyRule]).
fn check_list(
    val: &serde_yaml_ng::Value,
    parent: &str,
    known: &[&str],
    unknowns: &mut Vec<UnknownField>,
) {
    let seq = match val.as_sequence() {
        Some(s) => s,
        None => return,
    };
    for (i, entry) in seq.iter().enumerate() {
        let path = format!("{parent}[{i}]");
        check_mapping(entry, &path, known, unknowns);
    }
}

/// Recurse into known nested structures within a resource or machine.
fn check_nested(
    key: &str,
    val: &serde_yaml_ng::Value,
    parent: &str,
    unknowns: &mut Vec<UnknownField>,
) {
    let path = format!("{parent}.{key}");
    match key {
        "container" => check_mapping(val, &path, CONTAINER_FIELDS, unknowns),
        "pepita" => check_mapping(val, &path, PEPITA_FIELDS, unknowns),
        "lifecycle" => check_mapping(val, &path, LIFECYCLE_FIELDS, unknowns),
        "notify" => check_mapping(val, &path, NOTIFY_FIELDS, unknowns),
        _ => {}
    }
}

/// Extract a string from a YAML key Value.
fn yaml_key_str(key: &serde_yaml_ng::Value) -> String {
    match key {
        serde_yaml_ng::Value::String(s) => s.clone(),
        other => format!("{other:?}"),
    }
}

/// Build an UnknownField with optional Levenshtein suggestion.
fn make_unknown(path: &str, key: &str, known: &[&str]) -> UnknownField {
    let suggestion = closest_match(key, known);
    UnknownField {
        path: path.to_string(),
        key: key.to_string(),
        suggestion,
    }
}

/// Find the closest known field within Levenshtein distance <= 2.
fn closest_match(input: &str, candidates: &[&str]) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    for &candidate in candidates {
        let dist = levenshtein(input, candidate);
        if dist <= 2 && dist > 0 && (best.is_none() || dist < best.unwrap().0) {
            best = Some((dist, candidate));
        }
    }
    best.map(|(_, s)| s.to_string())
}

/// Levenshtein edit distance (O(nm), no allocations beyond a single row).
fn levenshtein(a: &str, b: &str) -> usize {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let n = a_bytes.len();
    let m = b_bytes.len();

    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }

    let mut row: Vec<usize> = (0..=m).collect();

    for i in 1..=n {
        let mut prev = row[0];
        row[0] = i;
        for j in 1..=m {
            let cost = if a_bytes[i - 1] == b_bytes[j - 1] {
                0
            } else {
                1
            };
            let new_val = (row[j] + 1).min(row[j - 1] + 1).min(prev + cost);
            prev = row[j];
            row[j] = new_val;
        }
    }
    row[m]
}
