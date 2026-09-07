//! FJ-2501: Format validation for resource and machine fields.
//!
//! Validates string formats that serde accepts as `String` but have
//! specific constraints: octal mode, port range, absolute paths,
//! Unix names, IP addresses/hostnames.

use super::ValidationError;
use crate::core::types::{ForjarConfig, Resource};

/// Run all format validations on a parsed config.
pub fn validate_formats(config: &ForjarConfig) -> Vec<ValidationError> {
    let mut errors = Vec::with_capacity(config.resources.len());
    for (id, resource) in &config.resources {
        validate_resource_formats(id, resource, &mut errors);
        validate_deny_paths(id, resource, &config.policy.deny_paths, &mut errors);
    }
    for (key, machine) in &config.machines {
        validate_machine_addr(key, &machine.addr, &mut errors);
    }
    errors
}

fn validate_resource_formats(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    validate_mode(id, resource, errors);
    validate_port(id, resource, errors);
    validate_path_absolute(id, resource, errors);
    validate_owner_group(id, resource, errors);
    validate_cron_schedule(id, resource, errors);
}

/// Mode must be octal string: exactly 3 or 4 octal digits, optionally prefixed with 0.
fn validate_mode(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if let Some(ref mode) = resource.mode {
        // Skip template expressions
        if mode.contains("{{") {
            return;
        }
        if !is_valid_mode(mode) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}': invalid mode '{mode}' (expected octal like '0644' or '0755')"
                ),
            });
        }
    }
}

/// Check if a mode string is valid octal: 4 digits where each is 0-7.
/// Accepts "0644", "0755", "1755" (setuid), "0000", etc.
pub(crate) fn is_valid_mode(mode: &str) -> bool {
    mode.len() == 4 && mode.bytes().all(|b| b.is_ascii_digit() && b < b'8')
}

/// Port must be 1-65535.
fn validate_port(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if let Some(ref port_str) = resource.port {
        if port_str.contains("{{") {
            return;
        }
        match port_str.parse::<u32>() {
            Ok(p) if (1..=65535).contains(&p) => {}
            _ => {
                errors.push(ValidationError {
                    message: format!(
                        "resource '{id}': port '{port_str}' out of range (must be 1-65535)"
                    ),
                });
            }
        }
    }
}

/// File path must be absolute (start with /) unless it's a template.
fn validate_path_absolute(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if let Some(ref path) = resource.path {
        if path.contains("{{") {
            return;
        }
        if !path.starts_with('/') {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}': path '{path}' must be absolute (start with '/')"
                ),
            });
        }
    }
}

/// Owner and group must be a Unix name (`^[a-z_][a-z0-9_-]*$`) or a bare
/// numeric id.
///
/// The numeric form is not a loophole, it is the correct thing to write when
/// the account does not exist in the host's passwd database — which is the
/// normal case for a directory bind-mounted into a container. Grafana runs as
/// uid 472, so `owner: 472` on its data directory is what makes the mount
/// usable, and `chown 472 /path` is exactly what forjar already emits for it
/// (src/resources/file.rs:59). Rejecting it made a correct config unwritable.
///
/// This was invisible until #357: `examples/dogfood-renacer.yaml` and
/// `dogfood-sovereign-stack.yaml` have carried `owner: 472` since they were
/// written, and both passed `forjar validate` because the resources come from a
/// recipe and recipes were never validated.
fn validate_owner_group(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    if let Some(ref owner) = resource.owner {
        if !owner.contains("{{") && !is_valid_unix_name(owner) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}': invalid owner '{owner}' (expected Unix username like 'root' or 'www-data')"
                ),
            });
        }
    }
    if let Some(ref group) = resource.group {
        if !group.contains("{{") && !is_valid_unix_name(group) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}': invalid group '{group}' (expected Unix group name)"
                ),
            });
        }
    }
}

/// Valid Unix username/group: starts with [a-z_], followed by [a-z0-9_-].
pub(crate) fn is_valid_unix_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 32 {
        return false;
    }
    // A bare numeric id. `chown 472 /path` is valid and is the only way to
    // express an owner with no passwd entry on the target host.
    if name.bytes().all(|b| b.is_ascii_digit()) {
        return true;
    }
    let first = name.as_bytes()[0];
    if !(first.is_ascii_lowercase() || first == b'_') {
        return false;
    }
    name.bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// FJ-2501: Cron schedule must have 5 fields with valid ranges.
/// Accepts: number, *, */N, N-N, and comma-separated lists thereof.
fn validate_cron_schedule(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    let schedule = match resource.schedule {
        Some(ref s) => s,
        None => return,
    };
    if schedule.contains("{{") {
        return;
    }
    // Special keywords
    if matches!(
        schedule.as_str(),
        "@yearly" | "@annually" | "@monthly" | "@weekly" | "@daily" | "@midnight" | "@hourly"
    ) {
        return;
    }
    let fields: Vec<&str> = schedule.split_whitespace().collect();
    if fields.len() != 5 {
        errors.push(ValidationError {
            message: format!(
                "resource '{id}': cron schedule '{schedule}' must have exactly 5 fields"
            ),
        });
        return;
    }
    let ranges: [(u32, u32); 5] = [(0, 59), (0, 23), (1, 31), (1, 12), (0, 7)];
    let names = ["minute", "hour", "day-of-month", "month", "day-of-week"];
    for (i, field) in fields.iter().enumerate() {
        if let Err(msg) = validate_cron_field(field, ranges[i].0, ranges[i].1) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}': cron {name} field '{field}': {msg}",
                    name = names[i]
                ),
            });
        }
    }
}

/// `*/N` — the step must parse and land in `1..=max`.
fn validate_cron_step(step: &str, max: u32) -> Result<(), String> {
    let n: u32 = step.parse().map_err(|_| format!("invalid step '{step}'"))?;
    if n == 0 || n > max {
        return Err(format!("step {n} out of range (1-{max})"));
    }
    Ok(())
}

/// `lo-hi` — both ends must parse, sit inside `min..=max`, and be in order.
fn validate_cron_range(part: &str, min: u32, max: u32) -> Result<(), String> {
    let (lo, hi) = part
        .split_once('-')
        .ok_or_else(|| format!("invalid range '{part}'"))?;
    let lo: u32 = lo.parse().map_err(|_| format!("invalid number '{lo}'"))?;
    let hi: u32 = hi.parse().map_err(|_| format!("invalid number '{hi}'"))?;
    if lo < min || hi > max || lo > hi {
        return Err(format!("range {lo}-{hi} out of bounds ({min}-{max})"));
    }
    Ok(())
}

/// One comma-separated part of a cron field: `*`, a `*/N` step, an `lo-hi`
/// range, or a plain value.
fn validate_cron_part(part: &str, min: u32, max: u32) -> Result<(), String> {
    if part == "*" {
        return Ok(());
    }
    if let Some(step) = part.strip_prefix("*/") {
        return validate_cron_step(step, max);
    }
    if part.contains('-') {
        return validate_cron_range(part, min, max);
    }
    let n: u32 = part
        .parse()
        .map_err(|_| format!("invalid value '{part}'"))?;
    if n < min || n > max {
        return Err(format!("value {n} out of range ({min}-{max})"));
    }
    Ok(())
}

/// Validate a single cron field against min..=max range. Reports the first
/// offending part, left to right.
pub(crate) fn validate_cron_field(field: &str, min: u32, max: u32) -> Result<(), String> {
    field
        .split(',')
        .try_for_each(|part| validate_cron_part(part, min, max))
}

/// FJ-2300: Check resource path against `policy.deny_paths` glob patterns.
fn validate_deny_paths(
    id: &str,
    resource: &Resource,
    deny_paths: &[String],
    errors: &mut Vec<ValidationError>,
) {
    if deny_paths.is_empty() {
        return;
    }
    let path = match resource.path.as_deref() {
        Some(p) if !p.contains("{{") => p,
        _ => return,
    };
    for pattern in deny_paths {
        if path_matches_glob(path, pattern) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}': path '{path}' is denied by policy.deny_paths pattern '{pattern}'"
                ),
            });
        }
    }
}

/// Simple glob matching: supports `*` (any segment) and `**` (any depth).
pub(crate) fn path_matches_glob(path: &str, pattern: &str) -> bool {
    if pattern.contains("**") {
        let prefix = pattern.split("**").next().unwrap_or("");
        path.starts_with(prefix)
    } else if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            path.starts_with(parts[0]) && path.ends_with(parts[1])
        } else {
            path == pattern
        }
    } else {
        path == pattern
    }
}

/// Machine addr must look like an IP or hostname (not empty, no spaces).
fn validate_machine_addr(key: &str, addr: &str, errors: &mut Vec<ValidationError>) {
    // Skip special transport markers
    if addr == "container" || addr == "pepita" || addr == "localhost" || addr == "127.0.0.1" {
        return;
    }
    if addr.contains("{{") {
        return;
    }
    if addr.is_empty() || addr.contains(' ') {
        errors.push(ValidationError {
            message: format!(
                "machine '{key}': invalid addr '{addr}' (must be an IP address or hostname)"
            ),
        });
    }
}
