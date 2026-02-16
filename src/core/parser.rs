//! FJ-002: YAML parsing and validation.
//!
//! Parses forjar.yaml and validates structural constraints:
//! - Version must be "1.0"
//! - Machine references in resources must exist
//! - depends_on references must exist
//! - Required fields per resource type

use super::types::*;
use std::path::Path;

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
    serde_yaml::from_str(yaml).map_err(|e| format!("YAML parse error: {}", e))
}

/// Validate a parsed config. Returns a list of errors (empty = valid).
pub fn validate_config(config: &ForjarConfig) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Version check
    if config.version != "1.0" {
        errors.push(ValidationError {
            message: format!("version must be \"1.0\", got \"{}\"", config.version),
        });
    }

    // Name check
    if config.name.is_empty() {
        errors.push(ValidationError {
            message: "name must not be empty".to_string(),
        });
    }

    // Validate each resource
    for (id, resource) in &config.resources {
        // Machine references
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

        // depends_on references
        for dep in &resource.depends_on {
            if !config.resources.contains_key(dep) {
                errors.push(ValidationError {
                    message: format!(
                        "resource '{}' depends on unknown resource '{}'",
                        id, dep
                    ),
                });
            }
            if dep == id {
                errors.push(ValidationError {
                    message: format!("resource '{}' depends on itself", id),
                });
            }
        }

        // Type-specific validation
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
            }
            ResourceType::Service => {
                if resource.name.is_none() {
                    errors.push(ValidationError {
                        message: format!("resource '{}' (service) has no name", id),
                    });
                }
            }
            ResourceType::Mount => {
                if resource.source.is_none() && resource.path.is_none() {
                    errors.push(ValidationError {
                        message: format!(
                            "resource '{}' (mount) needs source and target path",
                            id
                        ),
                    });
                }
            }
            _ => {} // Phase 2+ types — no validation yet
        }
    }

    errors
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
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors.iter().map(|e| &e.message).collect::<Vec<_>>());
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
        assert!(errors.iter().any(|e| e.message.contains("unknown resource")));
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
        assert!(errors.iter().any(|e| e.message.contains("depends on itself")));
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
    fn test_fj002_parse_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("forjar.yaml");
        std::fs::write(&path, r#"
version: "1.0"
name: file-test
machines: {}
resources: {}
"#).unwrap();
        let config = parse_config_file(&path).unwrap();
        assert_eq!(config.name, "file-test");
    }

    #[test]
    fn test_fj002_parse_invalid_yaml() {
        let result = parse_config("not: [valid: yaml: {{");
        assert!(result.is_err());
    }
}
