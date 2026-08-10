//! GH-206: a `source:` file's CONTENT is part of the desired state.
//!
//! `hash_desired_state` hashes resource FIELD STRINGS. For `content:` that is
//! right - the content is the field. For `source:` the field is a PATH, so
//! editing the referenced file left the hash identical, `plan` reported NoOp,
//! and `apply` printed "unchanged" over stale content on the machine. For a
//! declarative tool that is the worst available failure mode: silently not
//! converging while reporting success.
//!
//! Observed live in paiml/infra PMAT-204 - an edited reconciler script reported
//! "converged" three times while the box kept executing the previous copy.

use super::hashing::hash_desired_state;
use crate::core::types::{MachineTarget, Resource, ResourceType};
use std::io::Write;

/// A `type: file` resource pointing at `path` via `source:`.
fn source_resource(path: &str) -> Resource {
    Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("m1".to_string()),
        path: Some("/tmp/deployed-target".to_string()),
        source: Some(path.to_string()),
        mode: Some("0644".to_string()),
        ..Default::default()
    }
}

fn write(path: &std::path::Path, body: &str) {
    let mut f = std::fs::File::create(path).expect("create");
    f.write_all(body.as_bytes()).expect("write");
    f.flush().expect("flush");
}

#[test]
fn source_content_change_changes_desired_hash() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("payload.txt");
    let src_str = src.to_str().expect("utf8");

    write(&src, "VERSION-ONE\n");
    let h1 = hash_desired_state(&source_resource(src_str));

    write(&src, "VERSION-TWO\n");
    let h2 = hash_desired_state(&source_resource(src_str));

    assert_ne!(
        h1, h2,
        "editing a source: file must change the desired-state hash; \
         otherwise plan reports NoOp and apply deploys stale content while \
         printing 'unchanged' (GH-206)"
    );
}

#[test]
fn identical_source_content_hashes_identically() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("payload.txt");
    let src_str = src.to_str().expect("utf8");

    write(&src, "STABLE\n");
    let h1 = hash_desired_state(&source_resource(src_str));
    let h2 = hash_desired_state(&source_resource(src_str));

    assert_eq!(
        h1, h2,
        "hashing must stay deterministic - an unchanged source must not force \
         a spurious re-apply on every run"
    );
}

#[test]
fn two_sources_with_same_content_but_different_paths_differ() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    write(&a, "SAME\n");
    write(&b, "SAME\n");

    assert_ne!(
        hash_desired_state(&source_resource(a.to_str().unwrap())),
        hash_desired_state(&source_resource(b.to_str().unwrap())),
        "the path is still part of identity: two resources reading different \
         files must not collide just because the bytes match today"
    );
}

#[test]
fn missing_source_is_distinguishable_from_present_source() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("payload.txt");
    let src_str = src.to_str().expect("utf8");

    let h_missing = hash_desired_state(&source_resource(src_str));
    write(&src, "NOW-EXISTS\n");
    let h_present = hash_desired_state(&source_resource(src_str));

    assert_ne!(
        h_missing, h_present,
        "a source file appearing must change the hash, so a resource that was \
         unappliable becomes appliable rather than staying 'unchanged'"
    );
}

#[test]
fn resource_without_source_is_unaffected() {
    // Hash identity for source-less resources must NOT shift: field order is
    // hash identity, and changing it would invalidate every recorded hash on
    // every machine in the fleet. Only resources that declare `source:` may
    // gain a component.
    let pkg = Resource {
        resource_type: ResourceType::Package,
        machine: MachineTarget::Single("m1".to_string()),
        provider: Some("apt".to_string()),
        packages: vec!["curl".to_string()],
        ..Default::default()
    };
    let a = hash_desired_state(&pkg);
    let b = hash_desired_state(&pkg);
    assert_eq!(a, b, "determinism");

    // An inline-content file resource must also be untouched by the source path.
    let inline = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("m1".to_string()),
        path: Some("/tmp/x".to_string()),
        content: Some("hello".to_string()),
        ..Default::default()
    };
    assert_eq!(
        hash_desired_state(&inline),
        hash_desired_state(&inline),
        "inline content resources keep their existing hash behaviour"
    );
}
