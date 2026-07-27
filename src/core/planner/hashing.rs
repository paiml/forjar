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
}

/// Compute a hash of the desired state for comparison.
///
/// FJ-2200: Contract — determinism: same resource always produces same hash.
pub fn hash_desired_state(resource: &Resource) -> String {
    let type_str = resource.resource_type.to_string();
    // Owned canonicalization of overlay_hosts; kept alive for the borrow below.
    let overlay_hosts_canon = canonical_overlay_hosts(resource);
    let mut components: Vec<&str> = vec![&type_str];

    collect_core_fields(&mut components, resource);
    collect_phase2_fields(&mut components, resource, &overlay_hosts_canon);

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
