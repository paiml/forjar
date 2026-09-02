//! FJ-004: Desired-state hashing for the planner.
//!
//! # What is hashed, and why it is a denylist
//!
//! EVERYTHING on the resolved `Resource` except [`NON_IDENTITY_FIELDS`].
//!
//! It used to be an ALLOWLIST — 14 core fields, 20 phase-2 fields, the type —
//! covering 35 of the 122 serialised fields. `determine_present_action`
//! returns `NoOp` iff the recorded lock hash equals this hash, and
//! `executor::resource_ops::should_skip_single` then reports `unchanged`. So a
//! changed `uid`, `ssh_authorized_keys`, release `tag`, `driver_version`,
//! model `checksum`, `working_dir`, `timeout` or `sudo` hashed IDENTICALLY:
//! `plan` reported no change and `apply` printed `unchanged` over a machine
//! that still held the old value, permanently. Measured in the CRUX audit
//! (E01, #403): two six-resource configs differing in eleven identity fields
//! produced byte-identical `state.lock.yaml` files while `codegen` emitted
//! visibly different apply scripts.
//!
//! An allowlist fails silently — a field added later is simply not converged.
//! A denylist fails loudly, and `tests_hash_completeness` reflects over the
//! serialised `Resource` to make sure it keeps failing loudly.
//!
//! # Hash identity is versioned, not ordered
//!
//! The old collector's field ORDER was hash identity, so inserting a component
//! invalidated every recorded hash on every machine. The canonical form below
//! sorts mapping keys instead, so adding a `Resource` field changes only the
//! hashes of resources that actually set it. [`HASH_GENERATION`] gates the one
//! unavoidable break: it changes when the CANONICAL FORM itself changes, and
//! every resource then replans as `Update` exactly once. That is the correct
//! outcome, not a regression — forjar cannot know whether the 74 previously
//! unhashed fields drifted while nothing was watching them, so it re-converges
//! rather than assuming they did not.

use crate::core::types::*;
use crate::tripwire::hasher;
use serde_yaml_ng::{Mapping, Value};

/// Hash-identity generation for the canonical form.
///
/// Bump ONLY when the canonicalisation itself changes shape. Every recorded
/// lock hash on every machine stops matching, so the whole fleet re-converges
/// once — see the module header.
const HASH_GENERATION: &str = "forjar-desired-state-v2";

/// Fields that describe HOW, WHERE or WHETHER a resource is applied — never
/// WHAT it converges to. Excluded from the desired-state hash BY NAME.
///
/// Everything else on `Resource` is hashed. That polarity is the whole point:
/// the previous allowlist covered 35 of 109 fields, so a changed `uid`, `tag`,
/// `checksum` or `timeout` produced an identical hash and `apply` reported
/// `unchanged` over a machine that still held the old value (#403 / audit E01).
/// A denylist fails the other way — a field added later is hashed until
/// somebody decides it is not identity — and `tests_hash_completeness`
/// enforces exactly that.
pub(crate) const NON_IDENTITY_FIELDS: &[&str] = &[
    // Execution ORDER, not state.
    "depends_on",
    // WHICH host — each machine keeps its own lock, so folding it in would
    // only make the same declaration hash differently per host.
    "machine",
    // Selection filters for `--tag` / `--arch` / `--group`: they decide
    // whether this run touches the resource, not what it converges to.
    "tags",
    "arch",
    "resource_group",
    "when",
    // Expansion directives. `count` and `for_each` are consumed BEFORE
    // planning — by the time a resource is hashed it is already one expanded
    // copy, and the copies differ in the fields the template filled in.
    "count",
    "for_each",
    // Protection and trigger policy: they change what forjar is ALLOWED to do
    // and when it re-runs, not the state the host ends up in.
    "lifecycle",
    "triggers",
    // `phony` names an ACTION with no converged form at all.
    "phony",
];

/// Injectively render one YAML value, sorting mapping keys.
///
/// Injective on purpose: strings are length-prefixed so `content: "a;b"` can
/// never render the same bytes as a two-element list. A collision here is a
/// resource silently reported `unchanged`, which is the defect this module
/// exists to stop.
///
/// Sorting is what makes it deterministic. `overlay_hosts`, `inputs` and
/// `backup_remote_config` are `HashMap`s whose iteration order varies run to
/// run; unsorted, the same declaration would hash differently on every plan
/// and every resource would replan as `Update` forever.
fn write_canonical(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push('~'),
        Value::Bool(b) => out.push_str(if *b { "T" } else { "F" }),
        Value::Number(n) => {
            let text = n.to_string();
            out.push_str(&format!("n{}:{text}", text.len()));
        }
        Value::String(text) => out.push_str(&format!("s{}:{text}", text.len())),
        Value::Sequence(items) => {
            out.push('[');
            for item in items {
                write_canonical(out, item);
                out.push(';');
            }
            out.push(']');
        }
        Value::Mapping(map) => write_canonical_mapping(out, map),
        Value::Tagged(tagged) => {
            out.push_str(&format!("!{}", tagged.tag));
            write_canonical(out, &tagged.value);
        }
    }
}

/// Render a mapping with its entries in canonical-key order.
fn write_canonical_mapping(out: &mut String, map: &Mapping) {
    let mut entries: Vec<(String, &Value)> = map
        .iter()
        .map(|(k, v)| {
            let mut key = String::new();
            write_canonical(&mut key, k);
            (key, v)
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    out.push('{');
    for (key, value) in entries {
        out.push_str(&key);
        out.push(':');
        write_canonical(out, value);
        out.push(';');
    }
    out.push('}');
}

/// Canonical form of the whole RESOLVED declaration, minus the denylist.
///
/// This is the material fix for #403 / audit E01: the hash now covers every
/// field serde can see, so a field added to `Resource` is converged by default
/// instead of being silently ignored until someone remembers to list it.
fn canonical_declaration(resource: &Resource) -> String {
    let value = match serde_yaml_ng::to_value(resource) {
        Ok(value) => value,
        // Folded in rather than swallowed: collapsing every unserialisable
        // resource to one constant would make them all hash equal, which is
        // the exact failure this module is about.
        Err(e) => return format!("declaration_unserialisable:{e}"),
    };
    let Value::Mapping(mut map) = value else {
        return "declaration_not_a_mapping".to_string();
    };
    for field in NON_IDENTITY_FIELDS {
        map.remove(Value::String((*field).to_string()));
    }
    let mut out = String::new();
    write_canonical_mapping(&mut out, &map);
    out
}

/// GH-206: fold the CONTENT of a `source:` file into the desired state.
///
/// `source:` names a PATH, but the bytes it points at are what actually gets
/// deployed. Hashing only the path meant editing the referenced file left the
/// hash identical, so `plan` reported `NoOp` and `apply` skipped the resource
/// while printing "unchanged" over stale content on the machine. For a tool
/// whose entire contract is "converge to declared state", silently not
/// converging while reporting success is the worst available failure mode.
/// Observed live in paiml/infra PMAT-204.
///
/// This is exactly the invariant `canonical_declaration` above already states
/// for a different field: two resources differing ONLY in that field MUST hash
/// differently or `plan` will false-report `NoOp`.
///
/// Returns an EMPTY string when there is no `source:`, so nothing is appended
/// and every source-less resource keeps its existing hash. Field order is hash
/// identity; only resources that declare `source:` gain a component.
///
/// The path is read exactly as written, matching `resources::file`'s own
/// `source_file_base64` - both resolve relative to the process CWD - so the
/// planner hashes precisely the bytes apply would upload.
fn canonical_source_content(resource: &Resource) -> String {
    let Some(src) = resource.source.as_deref() else {
        return String::new();
    };
    match hasher::hash_file(std::path::Path::new(src)) {
        Ok(digest) => format!("source_content:{digest}"),
        // Unreadable is itself part of the observed state: fold the error kind
        // in so a source file appearing or disappearing changes the hash rather
        // than leaving the resource pinned at "unchanged". apply still fails
        // loudly with "cannot read source file".
        Err(e) => format!("source_unreadable:{src}:{e}"),
    }
}

/// FJ-036: canonical form of the reaper a `disk_budget` resource GENERATES.
///
/// Every other resource's desired state is fully described by its declaration.
/// A `disk_budget` is not: its real payload is a shell script synthesised by
/// forjar, so two forjar versions can produce different reapers from an
/// identical YAML block. Without this component the planner compares only the
/// declaration, reports "unchanged", and leaves the machine running the OLD
/// generated reaper indefinitely — which is precisely the silent desync the
/// resource exists to eliminate, reintroduced one level up.
///
/// Empty for every other resource type, so no existing hash changes.
fn canonical_generated_script(resource: &Resource) -> String {
    if resource.resource_type != ResourceType::DiskBudget {
        return String::new();
    }
    // Hash the WHOLE generated surface, not just `apply`. The state query is
    // what drift compares against; if its shape changes and the desired-state
    // hash does not, `apply` reports "unchanged" forever while `drift` reports
    // "drifted" forever, and nothing re-records the state. Covering all three
    // scripts makes any codegen change re-converge exactly once.
    let parts = [
        crate::core::codegen::apply_script(resource),
        crate::core::codegen::state_query_script(resource),
        crate::core::codegen::check_script(resource),
    ];
    let mut joined = String::new();
    for part in &parts {
        match part {
            Ok(script) => joined.push_str(script),
            Err(e) => return format!("generated_script_error:{e}"),
        }
        joined.push('\0');
    }
    format!("generated_script:{}", hasher::hash_string(&joined))
}

/// Compute a hash of the desired state for comparison.
///
/// FJ-2200: Contract — determinism: same resource always produces same hash.
///
/// Three components, joined with NULs:
///   1. [`HASH_GENERATION`] — the canonical-form version (see module header).
///   2. The whole declaration minus [`NON_IDENTITY_FIELDS`].
///   3. Anything the declaration does NOT describe: the bytes behind
///      `source:`, and the scripts a `disk_budget` GENERATES.
///
/// Components 2 and 3 exist for the same reason. A resource's desired state is
/// whatever `apply` would make true; a field, a referenced file's contents and
/// a generated reaper all change that, and all three must move the hash or
/// `plan` false-reports `NoOp`.
pub fn hash_desired_state(resource: &Resource) -> String {
    let declaration = canonical_declaration(resource);
    // Owned; empty when there is no `source:`.
    let source_content_canon = canonical_source_content(resource);
    // Owned; empty for every type except disk_budget.
    let generated_script_canon = canonical_generated_script(resource);

    let mut components: Vec<&str> = vec![HASH_GENERATION, &declaration];
    if !source_content_canon.is_empty() {
        components.push(&source_content_canon);
    }
    if !generated_script_canon.is_empty() {
        components.push(&generated_script_canon);
    }

    let joined = components.join("\0");
    let result = hasher::hash_string(&joined);

    // FJ-2200 / idempotent-apply-v1 contract: determinism postcondition.
    // Re-derives the declaration rather than re-hashing the same string: the
    // real determinism risk is `HashMap` iteration order inside `overlay_hosts`
    // / `inputs` / `backup_remote_config`, and re-hashing an already-built
    // string cannot see it.
    debug_assert_eq!(
        declaration,
        canonical_declaration(resource),
        "hash_desired_state: canonical form is not deterministic"
    );

    result
}
