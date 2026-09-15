//! PMAT-560: `exec_start` / `exec_sha256` declaration validation.

use super::*;
use crate::core::types::is_sha256_hex;

/// A declared program must be absolute and a declared digest must be what
/// `sha256sum` prints, because the host compares both as strings.
///
/// A relative `exec_start` can never equal what `systemctl show` reports (it
/// prints the resolved absolute path), and an uppercase or truncated digest
/// can never equal `sha256sum`'s output — either would be a resource that is
/// permanently divergent by construction, which is refused here rather than
/// discovered as a check that never passes. Templates are left for the
/// resolver, as every other format check does.
pub(super) fn validate_service_exec(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    if let Some(path) = resource.exec.exec_start.as_deref() {
        if !path.contains("{{") && !path.starts_with('/') {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (service): exec_start '{path}' must be absolute — \
                     systemctl reports the resolved program path, and a relative one \
                     could never match it"
                ),
            });
        }
    }
    if let Some(digest) = resource.exec.exec_sha256.as_deref() {
        if !digest.contains("{{") && !is_sha256_hex(digest) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (service): exec_sha256 '{digest}' is not 64 lowercase \
                     hex characters — declare exactly what `sha256sum <program> | cut -d' ' -f1` prints"
                ),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{ExecParity, ResourceType};

    fn svc(exec_start: Option<&str>, exec_sha256: Option<&str>) -> Resource {
        Resource {
            resource_type: ResourceType::Service,
            name: Some("svc".into()),
            exec: ExecParity {
                exec_start: exec_start.map(str::to_string),
                exec_sha256: exec_sha256.map(str::to_string),
            },
            ..Default::default()
        }
    }

    fn errors(r: &Resource) -> Vec<String> {
        let mut e = Vec::new();
        validate_service_exec("svc", r, &mut e);
        e.into_iter().map(|x| x.message).collect()
    }

    #[test]
    fn a_relative_program_is_refused() {
        let e = errors(&svc(Some("run.sh"), None));
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(e[0].contains("must be absolute"), "{e:?}");
    }

    #[test]
    fn a_templated_program_is_left_to_the_resolver() {
        assert!(errors(&svc(Some("{{params.eph_dir}}/run.sh"), None)).is_empty());
    }

    #[test]
    fn a_malformed_digest_is_refused() {
        for bad in ["9de9af1a1739b9f8", &"A".repeat(64), &"0".repeat(63), ""] {
            let e = errors(&svc(None, Some(bad)));
            assert_eq!(e.len(), 1, "{bad:?}: {e:?}");
            assert!(e[0].contains("64 lowercase hex"), "{e:?}");
        }
    }

    #[test]
    fn a_well_formed_declaration_passes() {
        assert!(errors(&svc(Some("/opt/x/run.sh"), Some(&"0".repeat(64)))).is_empty());
        assert!(errors(&svc(None, Some("{{params.digest}}"))).is_empty());
        assert!(errors(&svc(None, None)).is_empty());
    }
}
