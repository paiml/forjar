//! FJ-2721 (PMAT-199): every templatable field on `Resource` must resolve.
//!
//! # The defect this pins
//!
//! `resolve_resource_templates_with_secrets` is a hand-maintained list of
//! per-field assignments. A field added to `Resource` without a matching line
//! there is silently never resolved — no error, no warning, just the literal
//! `{{params.x}}` handed to whatever consumes it.
//!
//! That is not hypothetical. `task_inputs` — the field the whole v1.11
//! incremental-build release is about — was missing, while its sibling
//! `output_artifacts` was present. Reproduced on the published 1.11.1 binary
//! with `task_inputs: ["{{params.proj}}/src/a.c"]`:
//!
//! ```text
//!   apply           -> Apply complete: 1 converged, 0 unchanged.
//!   edit src/a.c
//!   apply           -> Apply complete: 0 converged, 1 unchanged.   # STALE
//! ```
//!
//! A stale artifact under a green summary: precisely the failure v1.11 was
//! released to eliminate, still live for anyone who templated their inputs.
//! v1.11's tests all used literal paths, so they proved the feature worked in
//! the one case where the missing resolution did not matter.
//!
//! # Why this test reflects instead of listing fields
//!
//! A hand-written list of fields to check has exactly the same failure mode as
//! the hand-written list of fields to resolve: the next field added is missing
//! from both. So this walks the SERIALISED form of `Resource`, discovers which
//! fields accept a string, and asserts each one resolves. A field added later
//! is covered without anybody remembering to come back here.

use super::*;
use crate::core::types::Resource;
use std::collections::HashMap;

const MARK: &str = "{{params.mark}}";
const MARK_VALUE: &str = "RESOLVED";

/// Fields that must NOT be template-resolved, with the reason.
///
/// These are structural: they name things inside the config rather than
/// describing state on a machine. `build_execution_order` runs on a RAW config
/// precisely because `depends_on` and `machine` are never templated; making
/// them templatable would silently invalidate that.
const STRUCTURAL_FIELDS: &[(&str, &str)] = &[
    (
        "depends_on",
        "resource ids — the DAG is built before resolution",
    ),
    (
        "machine",
        "machine names key `config.machines`, resolved separately",
    ),
    ("moved_from", "refers to a former resource id in the lock"),
    (
        "tags",
        "selectors for `-t`, matched on the RAW config in apply_filters before \
         resolution runs; resolving them here would make the filter and the \
         executed graph disagree",
    ),
    (
        "resource_group",
        "selector for `-g`, same pre-resolution filtering as `tags`",
    ),
    (
        "restart_on",
        "resource ids — a restart edge, structural like `depends_on`",
    ),
    (
        "triggers",
        "resource ids — a trigger edge, structural like `depends_on`",
    ),
    (
        "recipe",
        "names a recipe to include; resolved at parse/include time, before \
         params exist",
    ),
];

fn params() -> HashMap<String, serde_yaml_ng::Value> {
    let mut p = HashMap::new();
    p.insert(
        "mark".to_string(),
        serde_yaml_ng::Value::String(MARK_VALUE.to_string()),
    );
    p
}

/// Substitute `MARK` into `value` in whatever shape the field expects.
fn templated(value: &serde_yaml_ng::Value) -> Option<serde_yaml_ng::Value> {
    use serde_yaml_ng::Value;
    match value {
        Value::Null | Value::String(_) => Some(Value::String(MARK.to_string())),
        Value::Sequence(_) => Some(Value::Sequence(vec![Value::String(MARK.to_string())])),
        // Bools and numbers cannot carry a template.
        _ => None,
    }
}

/// Discover every field that accepts a string, by trying it.
///
/// Typed fields (enums, structs, numbers) reject the marker at deserialisation
/// and are skipped — so the set is derived from the type, not from a list
/// someone has to maintain.
fn string_shaped_fields() -> Vec<(String, Resource)> {
    let base = serde_yaml_ng::to_value(Resource::default()).expect("Resource serialises");
    let serde_yaml_ng::Value::Mapping(map) = base else {
        panic!("Resource is a struct");
    };

    let mut found = Vec::new();
    for (key, value) in map.iter() {
        let Some(name) = key.as_str() else { continue };
        if STRUCTURAL_FIELDS.iter().any(|(f, _)| *f == name) {
            continue;
        }
        let Some(marked) = templated(value) else {
            continue;
        };

        let mut candidate = map.clone();
        candidate.insert(key.clone(), marked);
        // Only a field that round-trips as a string is a templatable field.
        if let Ok(r) =
            serde_yaml_ng::from_value::<Resource>(serde_yaml_ng::Value::Mapping(candidate))
        {
            let round_tripped = serde_yaml_ng::to_value(&r).expect("serialises");
            if round_tripped
                .get(name)
                .map(|v| yaml_contains(v, MARK))
                .unwrap_or(false)
            {
                found.push((name.to_string(), r));
            }
        }
    }
    found
}

fn yaml_contains(v: &serde_yaml_ng::Value, needle: &str) -> bool {
    match v {
        serde_yaml_ng::Value::String(s) => s.contains(needle),
        serde_yaml_ng::Value::Sequence(xs) => xs.iter().any(|x| yaml_contains(x, needle)),
        serde_yaml_ng::Value::Mapping(m) => m.values().any(|x| yaml_contains(x, needle)),
        _ => false,
    }
}

#[test]
fn every_string_field_on_resource_is_template_resolved() {
    let machines = indexmap::IndexMap::new();
    let fields = string_shaped_fields();

    assert!(
        fields.len() > 20,
        "reflection found only {} templatable fields — the discovery is broken, \
         not the resolver",
        fields.len()
    );

    let mut unresolved = Vec::new();
    for (name, resource) in &fields {
        let resolved = resolve_resource_templates(resource, &params(), &machines)
            .unwrap_or_else(|e| panic!("{name}: resolve failed: {e}"));
        let out = serde_yaml_ng::to_value(&resolved).expect("serialises");
        if let Some(v) = out.get(name.as_str()) {
            if yaml_contains(v, MARK) {
                unresolved.push(name.clone());
            }
        }
    }

    assert!(
        unresolved.is_empty(),
        "these fields accept a template but the resolver never expands it, so \
         the literal `{{{{params.x}}}}` reaches whatever consumes them \
         ({} of {} fields):\n  {}\n\n\
         Add each to `resolve_resource_templates_with_secrets`, or add it to \
         STRUCTURAL_FIELDS with a reason if it must stay literal.",
        unresolved.len(),
        fields.len(),
        unresolved.join("\n  ")
    );
}

#[test]
fn task_inputs_resolves() {
    // The specific regression: the field v1.11's incremental build is about.
    // Kept as a named test so a failure says what broke, not just "a field".
    let r = Resource {
        task_inputs: vec!["{{params.mark}}/src/a.c".to_string()],
        output_artifacts: vec!["{{params.mark}}/build/a.o".to_string()],
        ..Default::default()
    };

    let resolved =
        resolve_resource_templates(&r, &params(), &indexmap::IndexMap::new()).expect("resolves");

    assert_eq!(
        resolved.task_inputs,
        vec![format!("{MARK_VALUE}/src/a.c")],
        "a templated task_input that stays literal makes the staleness probe \
         hash a path that does not exist, so `forjar apply` reports `unchanged` \
         over a stale artifact"
    );
    assert_eq!(
        resolved.output_artifacts,
        vec![format!("{MARK_VALUE}/build/a.o")],
        "output_artifacts was already resolved; it must stay that way"
    );
}

#[test]
fn pipeline_stage_fields_resolve() {
    // `stages` is a Vec<PipelineStage>, not a string field, so the reflection
    // above cannot reach into it. Its command/inputs/outputs are spliced into
    // executed shell by `pipeline_script`.
    let r = Resource {
        stages: vec![crate::core::types::PipelineStage {
            name: "s1".to_string(),
            command: Some("cc -c {{params.mark}}/a.c".to_string()),
            inputs: vec!["{{params.mark}}/a.c".to_string()],
            outputs: vec!["{{params.mark}}/a.o".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let resolved =
        resolve_resource_templates(&r, &params(), &indexmap::IndexMap::new()).expect("resolves");
    let s = &resolved.stages[0];

    assert!(
        !s.command.as_deref().unwrap_or("").contains(MARK),
        "a pipeline stage's command is executed verbatim: {:?}",
        s.command
    );
    assert!(!s.inputs[0].contains(MARK), "stage inputs: {:?}", s.inputs);
    assert!(
        !s.outputs[0].contains(MARK),
        "stage outputs: {:?}",
        s.outputs
    );
}

#[test]
fn structural_fields_are_deliberately_left_literal() {
    // The exclusions must be a decision, not an oversight. If one of these ever
    // starts resolving, `build_execution_order` on a raw config silently stops
    // agreeing with the executed graph.
    let r = Resource {
        depends_on: vec!["{{params.mark}}".to_string()],
        ..Default::default()
    };

    let resolved =
        resolve_resource_templates(&r, &params(), &indexmap::IndexMap::new()).expect("resolves");
    assert_eq!(
        resolved.depends_on,
        vec![MARK.to_string()],
        "depends_on must stay literal — the DAG is built before resolution"
    );
}
