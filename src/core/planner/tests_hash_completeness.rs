//! E01 (#403): every identity-bearing field on `Resource` must move the
//! desired-state hash.
//!
//! # The defect this pins
//!
//! `hash_desired_state` was a hand-maintained ALLOWLIST — 14 core fields plus
//! 20 phase-2 fields plus the type — covering 35 of the 109 fields on
//! `Resource`. `determine_present_action` returns `NoOp` iff the recorded lock
//! hash equals that hash, and `should_skip_single` then reports `Unchanged`.
//! So editing a release `tag`, a user's `uid` or `ssh_authorized_keys`, a GPU
//! `driver_version`, a model `checksum` or a task's `working_dir` produced an
//! IDENTICAL hash: `plan` said no change and `apply` said `unchanged` over a
//! machine that still held the old value, forever.
//!
//! Patched piecemeal at least five times (FJ-127, FJ-035, GH-206, #390,
//! FJ-036) — each time for one field. This is the general guard.
//!
//! # Why this reflects instead of listing fields
//!
//! A hand-written list of fields to check has exactly the failure mode of the
//! hand-written list it checks. This walks the SERIALISED form of `Resource`,
//! mutates one key at a time, and asks the hasher. A field added later is
//! covered without anyone remembering to come back here — the same discipline
//! `parser::tests_known_fields_completeness`, `resolver::tests_completeness`
//! and `types::resource_type_all` already apply to their sibling lists.

use super::hashing::{hash_desired_state, NON_IDENTITY_FIELDS};
use crate::core::types::{PlanAction, Resource, ResourceLock, ResourceStatus, ResourceType};
use serde_yaml_ng::{Mapping, Value};

/// Generic replacement values, tried in order.
///
/// Between them these cover every scalar and list shape on `Resource`. A field
/// whose type none of them can represent gets a bespoke entry in
/// [`STRUCTURED_PROBES`]; a field covered by neither fails the guard loudly
/// rather than being silently skipped, because a skipped field is exactly the
/// hole this test exists to close.
fn generic_probes() -> Vec<Value> {
    [
        "forjar-e01-sentinel",
        "7",
        "true",
        "[forjar-e01-sentinel]",
        "[7]",
        "{forjar-e01-sentinel: forjar-e01-sentinel}",
    ]
    .iter()
    .map(|s| serde_yaml_ng::from_str::<Value>(s).expect("probe literal parses"))
    .collect()
}

/// Fields whose type no generic sentinel can represent, as YAML.
const STRUCTURED_PROBES: &[(&str, &str)] = &[
    ("type", "file"),
    ("task_mode", "pipeline"),
    ("quality_gate", "{command: forjar-e01-gate}"),
    ("health_check", "{command: forjar-e01-health}"),
    ("restart_policy", "{max_restarts: 9}"),
    ("stages", "[{name: forjar-e01-stage}]"),
    ("output_equivalence", "{forjar-e01-artifact: external}"),
    (
        "budget_reclaim",
        "[{name: forjar-e01-rule, roots: ['/tmp/forjar-e01']}]",
    ),
    ("lifecycle", "{prevent_destroy: true}"),
];

fn probes_for(field: &str) -> Vec<Value> {
    let mut out: Vec<Value> = STRUCTURED_PROBES
        .iter()
        .filter(|(f, _)| *f == field)
        .map(|(_, yaml)| serde_yaml_ng::from_str::<Value>(yaml).expect("structured probe parses"))
        .collect();
    out.extend(generic_probes());
    out
}

fn as_mapping(resource: &Resource) -> Mapping {
    match serde_yaml_ng::to_value(resource).expect("Resource serialises") {
        Value::Mapping(m) => m,
        _ => panic!("Resource is a struct"),
    }
}

/// Build a `Resource` that differs from `base_map` at exactly `field`.
///
/// Returns `None` when no probe both deserialises AND actually lands — a probe
/// that deserialises into the default (serde ignores unknown keys inside a
/// nested struct) would let the guard pass while testing nothing.
fn mutate_one(base_map: &Mapping, field: &str) -> Option<Resource> {
    let key = Value::String(field.to_string());
    let before = base_map.get(&key).cloned().unwrap_or(Value::Null);
    for probe in probes_for(field) {
        let mut m = base_map.clone();
        m.insert(key.clone(), probe);
        let Ok(candidate) = serde_yaml_ng::from_value::<Resource>(Value::Mapping(m)) else {
            continue;
        };
        let after = as_mapping(&candidate)
            .get(&key)
            .cloned()
            .unwrap_or(Value::Null);
        if after != before {
            return Some(candidate);
        }
    }
    None
}

/// THE GUARD. Every serialised field either moves the hash or is on the
/// denylist — no third option, and no field left unprobed.
#[test]
fn every_identity_field_moves_the_desired_state_hash() {
    let base = Resource::default();
    let base_hash = hash_desired_state(&base);
    let base_map = as_mapping(&base);

    let mut unprobeable = Vec::new();
    let mut unhashed = Vec::new();
    let mut leaked = Vec::new();

    for key in base_map.keys() {
        let field = key.as_str().expect("field names are strings");
        let Some(mutated) = mutate_one(&base_map, field) else {
            unprobeable.push(field.to_string());
            continue;
        };
        let moved = hash_desired_state(&mutated) != base_hash;
        match (moved, NON_IDENTITY_FIELDS.contains(&field)) {
            (false, false) => unhashed.push(field.to_string()),
            (true, true) => leaked.push(field.to_string()),
            _ => {}
        }
    }

    assert!(
        unprobeable.is_empty(),
        "no probe value could set these fields, so the guard below cannot see them \
         ({} of {}):\n  {}\n\nAdd a YAML probe to STRUCTURED_PROBES for each.",
        unprobeable.len(),
        base_map.len(),
        unprobeable.join("\n  ")
    );
    assert!(
        unhashed.is_empty(),
        "these {} of {} fields on `Resource` do NOT change hash_desired_state, so \
         editing one leaves the lock hash identical: `plan` reports no change and \
         `apply` reports `unchanged` over a machine that still holds the old \
         value.\n  {}\n\nEither fold the field into the desired-state hash or add \
         it to NON_IDENTITY_FIELDS with a reason.",
        unhashed.len(),
        base_map.len(),
        unhashed.join("\n  ")
    );
    assert!(
        leaked.is_empty(),
        "these fields are on NON_IDENTITY_FIELDS but still change the hash, so a \
         cosmetic edit forces a fleet-wide re-apply:\n  {}",
        leaked.join("\n  ")
    );
}

/// The denylist must name real fields — a stale entry silently excludes
/// nothing while reading as if it excluded something.
#[test]
fn non_identity_fields_are_all_real_resource_fields() {
    let real = as_mapping(&Resource::default());
    let stale: Vec<&&str> = NON_IDENTITY_FIELDS
        .iter()
        .filter(|f| !real.contains_key(Value::String((**f).to_string())))
        .collect();
    assert!(
        stale.is_empty(),
        "NON_IDENTITY_FIELDS names keys that are not fields on `Resource`: {stale:?}"
    );
}

/// The reflection must actually find fields; a probe that silently stops
/// working turns the guard above into a no-op.
#[test]
fn the_reflection_sees_the_fields_e01_measured() {
    let base = Resource::default();
    let base_hash = hash_desired_state(&base);
    let base_map = as_mapping(&base);
    for field in [
        "tag",
        "binary",
        "uid",
        "checksum",
        "driver_version",
        "working_dir",
    ] {
        let mutated = mutate_one(&base_map, field)
            .unwrap_or_else(|| panic!("`{field}` could not be probed at all"));
        assert_ne!(
            hash_desired_state(&mutated),
            base_hash,
            "`{field}` is one of the fields E01 measured; if it does not move the \
             hash the guard above is reporting on a probe that stopped working"
        );
    }
}

// ── The measured case: the 11-field pair from the audit ──────────────────

/// One edit: the field's name (for the failure message), the resource type it
/// belongs to, and the mutation itself.
type EditCase = (&'static str, ResourceType, fn(&mut Resource));

fn converged_lock(
    resource_type: ResourceType,
    hash: String,
) -> std::collections::HashMap<String, crate::core::types::StateLock> {
    let mut resources = indexmap::IndexMap::new();
    resources.insert(
        "r".to_string(),
        ResourceLock {
            resource_type,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash,
            observed: None,
            details: std::collections::HashMap::new(),
        },
    );
    let mut locks = std::collections::HashMap::new();
    locks.insert(
        "m1".to_string(),
        crate::core::types::StateLock {
            schema: "1.0".to_string(),
            machine: "m1".to_string(),
            hostname: "m1".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        },
    );
    locks
}

/// Each of the eleven fields the audit measured, one at a time: converge with
/// the old value, edit only that field, and the planner must decide `Update`.
///
/// This asserts the planner's DECISION, not a hash string — `NoOp` here is
/// what `forjar apply` prints as `unchanged` and what makes it skip the
/// resource entirely (`executor::resource_ops::should_skip_single`).
#[test]
fn e01_editing_one_identity_field_replans_as_update() {
    let cases: &[EditCase] = &[
        ("tag", ResourceType::GithubRelease, |r| {
            r.tag = Some("v2.0.0".into());
        }),
        ("binary", ResourceType::GithubRelease, |r| {
            r.binary = Some("other-binary".into());
        }),
        ("asset_pattern", ResourceType::GithubRelease, |r| {
            r.asset_pattern = Some("*aarch64*".into());
        }),
        ("uid", ResourceType::User, |r| r.uid = Some(4242)),
        ("ssh_authorized_keys", ResourceType::User, |r| {
            r.ssh_authorized_keys = vec!["ssh-ed25519 AAAA-attacker".into()];
        }),
        ("driver_version", ResourceType::Gpu, |r| {
            r.driver_version = Some("570".into());
        }),
        ("cuda_version", ResourceType::Gpu, |r| {
            r.cuda_version = Some("12.9".into());
        }),
        ("checksum", ResourceType::Model, |r| {
            r.checksum = Some("blake3:deadbeef".into());
        }),
        ("quantization", ResourceType::Model, |r| {
            r.quantization = Some("q8_0".into());
        }),
        ("working_dir", ResourceType::Task, |r| {
            r.working_dir = Some("/srv/other".into());
        }),
        ("timeout", ResourceType::Task, |r| r.timeout = Some(9_999)),
        ("sudo", ResourceType::Task, |r| r.sudo = true),
    ];

    let mut silent = Vec::new();
    for (field, rtype, edit) in cases {
        let old = Resource {
            resource_type: rtype.clone(),
            machine: crate::core::types::MachineTarget::Single("m1".into()),
            name: Some("r".into()),
            ..Resource::default()
        };
        let mut new = old.clone();
        edit(&mut new);

        let locks = converged_lock(rtype.clone(), hash_desired_state(&old));
        let action =
            super::determine_action("r", &new, "m1", &locks, &std::collections::HashMap::new());
        if action != PlanAction::Update {
            silent.push(format!("{field} ({rtype}) -> {action:?}"));
        }
    }

    assert!(
        silent.is_empty(),
        "editing these fields after convergence did not replan as Update, so \
         `forjar apply` reports `unchanged` and never writes the new value:\n  {}",
        silent.join("\n  ")
    );
}
