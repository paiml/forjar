//! FJ-2501: Format validation for resource and machine fields.
//!
//! Validates string formats that serde accepts as `String` but have
//! specific constraints: octal mode, port range, absolute paths,
//! Unix names, IP addresses/hostnames.

use super::ValidationError;
use crate::core::types::{ForjarConfig, Resource};

/// Run all format validations on a parsed config.
pub fn validate_formats(config: &ForjarConfig) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for (id, resource) in &config.resources {
        validate_resource_formats(id, resource, &mut errors);
    }
    for (key, machine) in &config.machines {
        validate_machine_addr(key, &machine.addr, &mut errors);
    }
    errors
}

fn validate_resource_formats(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    validate_mode(id, resource, errors);
    validate_port(id, resource, errors);
    validate_path_absolute(id, resource, errors);
    validate_owner_group(id, resource, errors);
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
fn is_valid_mode(mode: &str) -> bool {
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
fn validate_path_absolute(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
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

/// Owner and group must be valid Unix names: ^[a-z_][a-z0-9_-]*$
fn validate_owner_group(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
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
fn is_valid_unix_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 32 {
        return false;
    }
    let first = name.as_bytes()[0];
    if !(first.is_ascii_lowercase() || first == b'_') {
        return false;
    }
    name.bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// Machine addr must look like an IP or hostname (not empty, no spaces).
fn validate_machine_addr(
    key: &str,
    addr: &str,
    errors: &mut Vec<ValidationError>,
) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_modes() {
        assert!(is_valid_mode("0644"));
        assert!(is_valid_mode("0755"));
        assert!(is_valid_mode("0600"));
        assert!(is_valid_mode("0777"));
        assert!(is_valid_mode("0000"));
        assert!(is_valid_mode("1755")); // setuid
    }

    #[test]
    fn invalid_modes() {
        assert!(!is_valid_mode("644")); // no leading zero, 3 chars but without 0 prefix
        assert!(!is_valid_mode("0888")); // 8 not valid octal
        assert!(!is_valid_mode("abcd"));
        assert!(!is_valid_mode(""));
        assert!(!is_valid_mode("07777")); // too long
    }

    #[test]
    fn valid_unix_names() {
        assert!(is_valid_unix_name("root"));
        assert!(is_valid_unix_name("www-data"));
        assert!(is_valid_unix_name("_apt"));
        assert!(is_valid_unix_name("nobody"));
        assert!(is_valid_unix_name("user123"));
    }

    #[test]
    fn invalid_unix_names() {
        assert!(!is_valid_unix_name(""));
        assert!(!is_valid_unix_name("123user")); // starts with digit
        assert!(!is_valid_unix_name("Root")); // uppercase
        assert!(!is_valid_unix_name("user.name")); // dot
        assert!(!is_valid_unix_name("a".repeat(33).as_str())); // too long
    }

    #[test]
    fn format_validation_on_config() {
        let yaml = r#"
version: "1.0"
name: format-test
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /etc/nginx.conf
    mode: "0644"
    owner: www-data
    group: www-data
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let errors = validate_formats(&config);
        assert!(errors.is_empty(), "expected no errors: {errors:?}");
    }

    #[test]
    fn format_bad_mode_detected() {
        let yaml = r#"
version: "1.0"
name: bad-mode
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /etc/test
    mode: "0999"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let errors = validate_formats(&config);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("invalid mode"));
    }

    #[test]
    fn format_bad_owner_detected() {
        let yaml = r#"
version: "1.0"
name: bad-owner
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /etc/test
    owner: "Bad User"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let errors = validate_formats(&config);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("invalid owner"));
    }

    #[test]
    fn format_relative_path_detected() {
        let yaml = r#"
version: "1.0"
name: rel-path
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: relative/path.txt
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let errors = validate_formats(&config);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("must be absolute"));
    }

    #[test]
    fn format_bad_machine_addr() {
        let yaml = r#"
version: "1.0"
name: bad-addr
machines:
  m:
    hostname: m
    addr: "has spaces"
resources: {}
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let errors = validate_formats(&config);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("invalid addr"));
    }

    #[test]
    fn format_template_expressions_skipped() {
        let yaml = r#"
version: "1.0"
name: template
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: "{{params.config_path}}"
    mode: "{{params.file_mode}}"
    owner: "{{params.owner}}"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let errors = validate_formats(&config);
        assert!(errors.is_empty(), "templates should be skipped: {errors:?}");
    }

    #[test]
    fn format_port_out_of_range() {
        let yaml = r#"
version: "1.0"
name: port-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  fw:
    type: network
    machine: m
    port: 99999
    protocol: tcp
    action: allow
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let errors = validate_formats(&config);
        assert!(errors.iter().any(|e| e.message.contains("port")));
    }
}
