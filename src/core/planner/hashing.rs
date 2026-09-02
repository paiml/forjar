//! FJ-004: Desired-state hashing for the planner.
//!
//! Extracted from `planner::mod` to keep that file under the 500-line health
//! limit. Field ORDER here is hash identity: appending is safe, inserting or
//! reordering silently invalidates every recorded hash on every machine.

use crate::core::types::*;
use crate::tripwire::hasher;

fn push_opt<'a>(components: &mut Vec<&'a str>, field: &'a Option<String>) {
    if let Some(ref val) = *field {
        components.push(val);
    }
}

/// Push all items from a Vec<String> onto the components list.
fn push_list<'a>(components: &mut Vec<&'a str>, items: &'a [String]) {
    for item in items {
        components.push(item);
    }
}

/// Collect core resource fields (phase 1) into hash components.
///
/// Field order is stable and must not change — it determines hash identity.
fn collect_core_fields<'a>(components: &mut Vec<&'a str>, resource: &'a Resource) {
    push_opt(components, &resource.state);
    push_opt(components, &resource.provider);
    push_list(components, &resource.packages);
    push_opt(components, &resource.path);
    push_opt(components, &resource.content);
    push_opt(components, &resource.source);
    push_opt(components, &resource.name);
    push_opt(components, &resource.owner);
    push_opt(components, &resource.group);
    push_opt(components, &resource.mode);
    push_opt(components, &resource.fs_type);
    push_opt(components, &resource.options);
    push_opt(components, &resource.target);
    push_opt(components, &resource.version);
}

/// Canonicalize `overlay_hosts` (a `HashMap`) into a stable, hashable string.
///
/// FJ-035: the `/etc/hosts` managed block is part of the converged state, so
/// two overlay_interface resources differing ONLY in `overlay_hosts` MUST hash
/// differently or `plan` will false-report `NoOp` and never rewrite the block.
/// `HashMap` iteration order is non-deterministic, so we sort by (name, ip) to
/// get a deterministic, order-independent serialization. Returns an empty
/// string when the map is absent/empty (no contribution to the hash).
fn canonical_overlay_hosts(resource: &Resource) -> String {
    let Some(hosts) = resource.overlay_hosts.as_ref() else {
        return String::new();
    };
    if hosts.is_empty() {
        return String::new();
    }
    let mut entries: Vec<(&String, &String)> = hosts.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0).then(a.1.cmp(b.1)));
    let mut out = String::from("overlay_hosts:");
    for (name, ip) in entries {
        out.push_str(name);
        out.push('=');
        out.push_str(ip);
        out.push(';');
    }
    out
}

/// Collect phase 2 resource fields into hash components.
///
/// Field order is stable and must not change — it determines hash identity.
/// `overlay_hosts_canon` is the pre-computed canonical serialization of
/// `overlay_hosts` (see `canonical_overlay_hosts`); it is threaded in as an
/// owned `&str` because the source map cannot be borrowed as a single slice.
fn collect_phase2_fields<'a>(
    components: &mut Vec<&'a str>,
    resource: &'a Resource,
    overlay_hosts_canon: &'a str,
) {
    push_opt(components, &resource.image);
    push_opt(components, &resource.command);
    push_opt(components, &resource.schedule);
    push_opt(components, &resource.restart);
    push_opt(components, &resource.port);
    push_opt(components, &resource.protocol);
    push_opt(components, &resource.action);
    push_opt(components, &resource.from_addr);
    push_opt(components, &resource.shell);
    push_opt(components, &resource.home);
    if let Some(ref enabled) = resource.enabled {
        components.push(if *enabled { "enabled" } else { "disabled" });
    }
    push_list(components, &resource.ports);
    push_list(components, &resource.environment);
    push_list(components, &resource.volumes);
    push_list(components, &resource.restart_on);
    // FJ-035: overlay_interface identity-bearing fields. overlay_ip changes the
    // bound address / ExecStart line, overlay_iface changes the target NIC,
    // overlay_firewall changes the converged ufw state, and overlay_hosts
    // changes the managed /etc/hosts block — all must alter the desired-state
    // hash so plan does not wrongly report NoOp.
    push_opt(components, &resource.overlay_ip);
    push_opt(components, &resource.overlay_iface);
    if let Some(fw) = resource.overlay_firewall {
        components.push(if fw {
            "overlay_fw_on"
        } else {
            "overlay_fw_off"
        });
    }
    // FJ-035 MATERIAL FIX: overlay_hosts was omitted from the hash collector, so
    // two resources differing ONLY in their /etc/hosts map hashed equal and plan
    // false-reported NoOp. Folding the canonical serialization closes that hole.
    if !overlay_hosts_canon.is_empty() {
        components.push(overlay_hosts_canon);
    }
    // #390: `completion_check` is a `task`'s ASSERTION — the whole declared
    // state for a guard resource whose `command` only ever reports the
    // violation (#380's "a guard IS a forjar resource" reading). It was never
    // folded into the hash, so editing ONLY the check (tightening an
    // assertion, fixing a check that tested the wrong thing) left the hash
    // unchanged: a lock entry seeded or converged against the OLD check
    // compared equal to the NEW one and `plan` reported NoOp over a
    // resource whose declared condition had genuinely changed. Same defect
    // class as FJ-035's overlay_hosts, one field over: a field that changes
    // the converged meaning of the resource but not its own byte-for-byte
    // comparison against a prior hash.
    push_opt(components, &resource.completion_check);
}

/// Compute a hash of the desired state for comparison.
///
/// FJ-2200: Contract — determinism: same resource always produces same hash.
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
/// This is exactly the invariant `canonical_overlay_hosts` below already states
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

pub fn hash_desired_state(resource: &Resource) -> String {
    let type_str = resource.resource_type.to_string();
    // Owned canonicalization of overlay_hosts; kept alive for the borrow below.
    let overlay_hosts_canon = canonical_overlay_hosts(resource);
    // Owned; kept alive for the borrow below. Empty when there is no `source:`.
    let source_content_canon = canonical_source_content(resource);
    // Owned; empty for every type except disk_budget.
    let generated_script_canon = canonical_generated_script(resource);
    let mut components: Vec<&str> = vec![&type_str];

    collect_core_fields(&mut components, resource);
    collect_phase2_fields(&mut components, resource, &overlay_hosts_canon);
    // APPENDED last on purpose: inserting or reordering would invalidate every
    // recorded hash on every machine (see the module header).
    if !source_content_canon.is_empty() {
        components.push(&source_content_canon);
    }
    if !generated_script_canon.is_empty() {
        components.push(&generated_script_canon);
    }

    let joined = components.join("\0");
    let result = hasher::hash_string(&joined);

    // FJ-2200 / idempotent-apply-v1 contract: determinism postcondition —
    // calling again must produce the same hash
    debug_assert_eq!(
        result,
        hasher::hash_string(&joined),
        "hash_desired_state: determinism violated"
    );

    result
}
