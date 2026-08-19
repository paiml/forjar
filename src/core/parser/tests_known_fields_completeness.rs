//! FJ-036: every field on `Resource` must appear in `RESOURCE_FIELDS`.
//!
//! # The defect this pins
//!
//! `RESOURCE_FIELDS` is a hand-maintained list of valid YAML keys, consumed by
//! the unknown-field checker. A field added to `Resource` without a matching
//! entry here is accepted by serde and then **rejected by validation** with
//! `unknown field 'x'` — so the feature is undeclarable in YAML while its Rust
//! side is fully implemented and fully tested.
//!
//! That is not hypothetical: the five `budget_*` fields of the `disk_budget`
//! resource shipped with a green `cargo test` (12,831 passing, including 51
//! for the resource itself) and every machine declaration using them failed
//! with "unknown field" until this list was updated. The Rust tests could not
//! see it because none of them go through YAML validation.
//!
//! # Why this reflects instead of listing fields
//!
//! A hand-written list of fields to check has the same failure mode as the
//! hand-written list it is checking. This walks the SERIALISED form of
//! `Resource`, so a field added later is covered without anyone remembering to
//! come back here.

use super::known_fields::RESOURCE_FIELDS;
use crate::core::types::Resource;

#[test]
fn every_resource_field_is_declarable_in_yaml() {
    let base = serde_yaml_ng::to_value(Resource::default()).expect("Resource serialises");
    let serde_yaml_ng::Value::Mapping(map) = base else {
        panic!("Resource is a struct");
    };

    let missing: Vec<String> = map
        .iter()
        .filter_map(|(k, _)| k.as_str())
        .filter(|name| !RESOURCE_FIELDS.contains(name))
        .map(str::to_string)
        .collect();

    assert!(
        missing.is_empty(),
        "these fields exist on `Resource` but are not in `RESOURCE_FIELDS`, so any \
         forjar.yaml using them fails validation with `unknown field` ({} of {}):\n  {}\n\n\
         Add each to RESOURCE_FIELDS in src/core/parser/known_fields.rs.",
        missing.len(),
        map.len(),
        missing.join("\n  ")
    );
}

#[test]
fn resource_fields_has_no_stale_entries() {
    // The other direction: an entry for a field that no longer exists silently
    // accepts a typo'd key forever.
    let base = serde_yaml_ng::to_value(Resource::default()).expect("Resource serialises");
    let serde_yaml_ng::Value::Mapping(map) = base else {
        panic!("Resource is a struct");
    };
    let real: Vec<&str> = map.iter().filter_map(|(k, _)| k.as_str()).collect();

    // Aliases accepted in YAML that do not correspond 1:1 to a struct field.
    const ALIASES: &[&str] = &["type", "moved_from"];

    let stale: Vec<&&str> = RESOURCE_FIELDS
        .iter()
        .filter(|f| !real.contains(f) && !ALIASES.contains(f))
        .collect();

    assert!(
        stale.is_empty(),
        "`RESOURCE_FIELDS` lists keys that are not fields on `Resource`; a typo matching \
         one of these would be silently accepted:\n  {stale:?}"
    );
}
