//! Tests for image reference parsing (Refs #210).
//!
//! RED on the pre-fix code: the push target was derived as
//! `name.unwrap_or("app")` split at the FIRST `/`, so `myorg/app` parsed as
//! registry `myorg`, an un-prefixed name became `docker.io/<name>` with no
//! `library/` namespace, and a resource that declared nothing was pushed to
//! `docker.io/app:latest`.

use super::image_ref::{parse_image_ref, ImageRef};

#[test]
fn parses_fully_qualified_reference() {
    let r = parse_image_ref("ghcr.io/foo/bar:1.2.3").unwrap();
    assert_eq!(
        r,
        ImageRef {
            registry: "ghcr.io".into(),
            repository: "foo/bar".into(),
            tag: "1.2.3".into(),
        }
    );
    assert_eq!(r.api_host(), "ghcr.io");
    assert_eq!(r.to_reference(), "ghcr.io/foo/bar:1.2.3");
}

#[test]
fn first_component_without_dot_is_a_namespace_not_a_registry() {
    // RED before the fix: registry was "myorg", name "myapp".
    let r = parse_image_ref("myorg/myapp:2.0").unwrap();
    assert_eq!(r.registry, "docker.io");
    assert_eq!(r.repository, "myorg/myapp");
    assert_eq!(r.tag, "2.0");
}

#[test]
fn bare_name_gets_the_library_namespace_and_latest() {
    let r = parse_image_ref("nginx").unwrap();
    assert_eq!(r.registry, "docker.io");
    assert_eq!(r.repository, "library/nginx");
    assert_eq!(r.tag, "latest");
}

#[test]
fn docker_hub_resolves_to_the_distribution_endpoint() {
    // docker.io is a website; POSTing an upload there answers 301 to the
    // marketing site, which the old push mistook for an upload session.
    for reference in ["docker.io/library/app:1", "index.docker.io/library/app:1"] {
        let r = parse_image_ref(reference).unwrap();
        assert_eq!(r.api_host(), "registry-1.docker.io", "for {reference}");
    }
}

#[test]
fn localhost_and_ported_hosts_are_registries() {
    let r = parse_image_ref("localhost:5000/app:dev").unwrap();
    assert_eq!(r.registry, "localhost:5000");
    assert_eq!(r.repository, "app");
    assert_eq!(r.tag, "dev");

    let r = parse_image_ref("127.0.0.1:5000/team/app").unwrap();
    assert_eq!(r.registry, "127.0.0.1:5000");
    assert_eq!(r.repository, "team/app");
    assert_eq!(r.tag, "latest");
}

#[test]
fn deep_repository_paths_are_preserved() {
    let r = parse_image_ref("registry.example.com/a/b/c/d:v9").unwrap();
    assert_eq!(r.registry, "registry.example.com");
    assert_eq!(r.repository, "a/b/c/d");
    assert_eq!(r.tag, "v9");
}

#[test]
fn a_port_is_not_mistaken_for_a_tag() {
    let r = parse_image_ref("registry.example.com:5000/app").unwrap();
    assert_eq!(r.registry, "registry.example.com:5000");
    assert_eq!(r.repository, "app");
    assert_eq!(r.tag, "latest");
}

#[test]
fn rejects_references_it_cannot_push_honestly() {
    let cases = [
        ("", "empty"),
        ("   ", "empty"),
        ("ghcr.io/foo/bar:", "empty tag"),
        ("ghcr.io/foo/bar@sha256:abc", "digest-pinned"),
        ("https://ghcr.io/foo/bar:1", "URL"),
        ("ghcr.io/foo bar:1", "whitespace"),
        ("ghcr.io/Foo/Bar:1", "uppercase"),
    ];
    for (reference, why) in cases {
        let err = parse_image_ref(reference)
            .expect_err(&format!("{reference:?} must be rejected ({why})"));
        assert!(
            err.contains(why),
            "error for {reference:?} should mention {why}: {err}"
        );
    }
}

#[test]
fn parse_is_total_over_junk_without_panicking() {
    for reference in ["/", "//", ":", "a/", "/a", "a:b:c", "::"] {
        let _ = parse_image_ref(reference);
    }
}
