//! IMG-001: `type: docker` `state: pulled` digest validation.

use super::*;

fn docker_pulled_yaml(image: &str) -> String {
    format!(
        r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  golden:
    type: docker
    machine: m1
    name: golden
    image: {image}
    state: pulled
"#
    )
}

#[test]
fn test_docker_pulled_accepts_sha256_digest() {
    let img = "ghcr.io/paiml/build@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let config = parse_config(&docker_pulled_yaml(img)).unwrap();
    let errors = validate_config(&config);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn test_docker_pulled_rejects_tag_without_digest() {
    let config = parse_config(&docker_pulled_yaml("ghcr.io/paiml/build:latest")).unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.iter().any(|e| e.message.contains("@sha256")),
        "{errors:?}"
    );
}

#[test]
fn test_docker_pulled_rejects_short_or_malformed_digest() {
    for img in ["img@sha256:abc", "img@sha256:ZZZZ", "img@md5:0123"] {
        let config = parse_config(&docker_pulled_yaml(img)).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.iter().any(|e| e.message.contains("@sha256")),
            "{img}: {errors:?}"
        );
    }
}

#[test]
fn test_docker_running_still_allows_tags() {
    let yaml = docker_pulled_yaml("nginx:latest").replace("state: pulled", "state: running");
    let config = parse_config(&yaml).unwrap();
    assert!(validate_config(&config).is_empty());
}
