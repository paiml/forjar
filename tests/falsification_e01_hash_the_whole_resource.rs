//! forjar#403 (CRUX audit E01): `hash_desired_state` covered 35 of 122
//! `Resource` fields, so a changed declaration was reported `unchanged` and
//! never applied.
//!
//! WHAT WAS OBSERVABLY WRONG. `determine_present_action` returns `NoOp` iff
//! the recorded lock hash equals `hash_desired_state(resource)`, and
//! `should_skip_single` then prints `unchanged`. The hasher was a hand-written
//! ALLOWLIST: `uid`, `ssh_authorized_keys`, release `tag`, `driver_version`,
//! model `checksum`, `working_dir`, `timeout` and `sudo` were not on it, so
//! two configs differing in any of them hashed to the SAME string. `plan`
//! reported no change; `apply` printed `unchanged` over a machine that still
//! held the old value — permanently, because nothing would ever re-hash
//! differently.
//!
//! WHY THESE ASSERTIONS. Each `*_moves_the_hash` test declares one resource
//! twice through the real YAML parser, differing in exactly ONE field that the
//! old allowlist omitted, and asks the hasher whether it can tell them apart.
//! With the allowlist restored every one of them goes red, because the field
//! is simply never read. The two `does_not_move_the_hash` tests are the other
//! half of the contract — selection filters and `HashMap` insertion order must
//! NOT churn the hash — and they are green on both trees; they are here so a
//! future "hash everything, including `tags`" cannot land quietly.

use forjar::core::parser::parse_config;
use forjar::core::planner::hash_desired_state;
use forjar::core::types::Resource;

/// Parse a one-resource config and return that resource, resolved the way
/// `plan` and `apply` see it.
fn resource(body: &str) -> Resource {
    let yaml = format!(
        "version: \"1.0\"\nname: e01\nmachines:\n  box:\n    hostname: box\n    addr: 192.0.2.10\n    user: root\nresources:\n  r:\n{body}"
    );
    let cfg = parse_config(&yaml).unwrap_or_else(|e| panic!("config must parse: {e}\n{yaml}"));
    cfg.resources
        .get("r")
        .cloned()
        .expect("resource `r` is declared")
}

/// Indent a resource body under `resources.r`.
fn body(lines: &[&str]) -> String {
    lines
        .iter()
        .map(|l| format!("    {l}\n"))
        .collect::<String>()
}

fn assert_moves(field: &str, before: &[&str], after: &[&str]) {
    let a = hash_desired_state(&resource(&body(before)));
    let b = hash_desired_state(&resource(&body(after)));
    assert_ne!(
        a, b,
        "changing `{field}` must change the desired-state hash, or apply reports `unchanged` \
         over a host that still holds the old value (#403)"
    );
}

fn assert_holds(what: &str, before: &[&str], after: &[&str]) {
    let a = hash_desired_state(&resource(&body(before)));
    let b = hash_desired_state(&resource(&body(after)));
    assert_eq!(
        a, b,
        "{what} must NOT change the desired-state hash — it decides whether a run touches the \
         resource, not what the resource converges to"
    );
}

#[test]
fn uid_moves_the_hash() {
    assert_moves(
        "uid",
        &["type: user", "machine: box", "name: deploy", "uid: 1000"],
        &["type: user", "machine: box", "name: deploy", "uid: 1001"],
    );
}

#[test]
fn ssh_authorized_keys_move_the_hash() {
    assert_moves(
        "ssh_authorized_keys",
        &[
            "type: user",
            "machine: box",
            "name: deploy",
            "ssh_authorized_keys: ['ssh-ed25519 AAAA1 a']",
        ],
        &[
            "type: user",
            "machine: box",
            "name: deploy",
            "ssh_authorized_keys: ['ssh-ed25519 AAAA2 b']",
        ],
    );
}

#[test]
fn task_timeout_moves_the_hash() {
    assert_moves(
        "timeout",
        &[
            "type: task",
            "machine: box",
            "command: 'true'",
            "timeout: 30",
        ],
        &[
            "type: task",
            "machine: box",
            "command: 'true'",
            "timeout: 60",
        ],
    );
}

#[test]
fn task_working_dir_moves_the_hash() {
    assert_moves(
        "working_dir",
        &[
            "type: task",
            "machine: box",
            "command: 'make'",
            "working_dir: /srv/a",
        ],
        &[
            "type: task",
            "machine: box",
            "command: 'make'",
            "working_dir: /srv/b",
        ],
    );
}

#[test]
fn task_sudo_moves_the_hash() {
    assert_moves(
        "sudo",
        &[
            "type: task",
            "machine: box",
            "command: 'systemctl restart x'",
            "sudo: false",
        ],
        &[
            "type: task",
            "machine: box",
            "command: 'systemctl restart x'",
            "sudo: true",
        ],
    );
}

#[test]
fn release_tag_moves_the_hash() {
    assert_moves(
        "tag",
        &[
            "type: github_release",
            "machine: box",
            "repo: paiml/forjar",
            "tag: v1.23.0",
        ],
        &[
            "type: github_release",
            "machine: box",
            "repo: paiml/forjar",
            "tag: v1.23.1",
        ],
    );
}

#[test]
fn gpu_driver_version_moves_the_hash() {
    assert_moves(
        "driver_version",
        &["type: gpu", "machine: box", "driver_version: '550.0'"],
        &["type: gpu", "machine: box", "driver_version: '560.0'"],
    );
}

#[test]
fn model_checksum_moves_the_hash() {
    assert_moves(
        "checksum",
        &[
            "type: model",
            "machine: box",
            "name: m",
            "source: /models/m.gguf",
            "checksum: sha256:aaaa",
        ],
        &[
            "type: model",
            "machine: box",
            "name: m",
            "source: /models/m.gguf",
            "checksum: sha256:bbbb",
        ],
    );
}

#[test]
fn selection_filters_do_not_move_the_hash() {
    assert_holds(
        "a `tags:` filter",
        &[
            "type: file",
            "machine: box",
            "path: /etc/motd",
            "content: hi",
            "tags: [a]",
        ],
        &[
            "type: file",
            "machine: box",
            "path: /etc/motd",
            "content: hi",
            "tags: [b, c]",
        ],
    );
}

#[test]
fn overlay_hosts_insertion_order_does_not_move_the_hash() {
    assert_holds(
        "HashMap insertion order",
        &[
            "type: overlay_interface",
            "machine: box",
            "overlay_ip: 10.0.0.1",
            "overlay_hosts: {a: 10.0.0.2, b: 10.0.0.3}",
        ],
        &[
            "type: overlay_interface",
            "machine: box",
            "overlay_ip: 10.0.0.1",
            "overlay_hosts: {b: 10.0.0.3, a: 10.0.0.2}",
        ],
    );
}
