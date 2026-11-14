//! forjar#1000: one `github_runner` seat identity, one resource.
//!
//! A seat's verdict is keyed on its identity (org or repo, plus runner `name`).
//! Two resources holding the same identity on different machines let the
//! first register the runner and the second find it listed, be reported
//! `adopted` by apply and `ok` by check, and be recorded converged with nothing
//! installed. The identity can be repeated by separate resources, by two
//! machine keys for one host, by a `{{params.*}}` template, by a different
//! spelling, by `for_each`, or by an `absent` twin that would deregister the
//! other's runner. `forjar validate` must refuse every one of them, naming
//! both resource ids, before any token is requested.
//!
//! The controls keep the rule narrow: different names, the same name in two
//! repos, and the same name at org and at repo scope are different seats.
//!
//! Usage: cargo test --test falsification_github_runner_one_seat

use std::process::Command;

const SCOPE_ORG: &str = "    scope: org\n    group: default\n";

/// One `github_runner` resource. `head` holds the lines that differ between
/// cases (`machine`, `scope`/`repo`/`group`, `state`, `for_each`).
fn seat(id: &str, head: &str, owner: &str, name: &str) -> String {
    format!(
        "  {id}:\n    type: github_runner\n{head}\
         \x20   owner: {owner}\n    name: {name}\n\
         \x20   labels: [self-hosted, linux, x64, gate]\n\
         \x20   version: 2.330.0\n    sha256: {zeros}\n\
         \x20   work_dir: /var/lib/runner/{id}\n    user: runner\n\
         \x20   app:\n      app_id: 123456\n      installation_id: 7890123\n\
         \x20     private_key: \"{{{{secrets.runner-app-pem}}}}\"\n",
        zeros = "0".repeat(64),
    )
}

/// `head` for an org-scoped seat on `machine`.
fn on(machine: &str) -> String {
    format!("    machine: {machine}\n{SCOPE_ORG}")
}

/// `forjar validate` on a manifest with the machines `local`, `peer` and
/// `local-again` (a second key for the host `local` names) and `resources`.
/// Returns combined stdout and stderr, and whether it exited 0.
fn validate(resources: &[String]) -> (String, bool) {
    let dir = tempfile::tempdir().unwrap();
    let secrets = dir.path().join("secrets");
    std::fs::create_dir_all(&secrets).unwrap();
    std::fs::write(secrets.join("runner-app-pem"), "example-marker-key").unwrap();
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: '1.0'\nname: github-runner-seat\n\
             params:\n  seat: host-gate-1\n\
             secrets:\n  provider: file\n  path: \"{secrets}\"\n\
             machines:\n\
             \x20 local:\n    hostname: localhost\n    addr: localhost\n    transport: local\n\
             \x20 local-again:\n    hostname: localhost\n    addr: localhost\n    transport: local\n\
             \x20 peer:\n    hostname: peer\n    addr: 127.0.0.2\n\
             resources:\n{body}",
            secrets = secrets.display(),
            body = resources.concat(),
        ),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(dir.path())
        .args(["validate", "-f", &cfg.display().to_string()])
        .output()
        .expect("forjar must run");
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.success(),
    )
}

/// The manifest must be refused, and the refusal must name `ids` and say the
/// seat is already declared.
fn refused(resources: &[String], ids: &[&str]) {
    let (out, ok) = validate(resources);
    assert!(
        !ok,
        "validate accepted one seat held by two resources:\n{out}"
    );
    for needle in ids
        .iter()
        .copied()
        .chain(["github_runner", "already declared"])
    {
        assert!(
            out.contains(needle),
            "the refusal must name {needle}:\n{out}"
        );
    }
}

fn accepted(resources: &[String]) {
    let (out, ok) = validate(resources);
    assert!(ok, "two different seats must pass validate:\n{out}");
}

#[test]
fn one_seat_same_name_on_two_machines_is_refused() {
    refused(
        &[
            seat("seat-a", &on("local"), "example-org", "host-gate-1"),
            seat("seat-b", &on("peer"), "example-org", "host-gate-1"),
        ],
        &["seat-a", "seat-b"],
    );
}

#[test]
fn one_seat_two_machine_keys_for_one_host_are_refused() {
    refused(
        &[
            seat("seat-a", &on("local"), "example-org", "host-gate-1"),
            seat("seat-b", &on("local-again"), "example-org", "host-gate-1"),
        ],
        &["seat-a", "seat-b"],
    );
}

#[test]
fn one_seat_a_params_template_for_the_same_name_is_refused() {
    refused(
        &[
            seat("seat-a", &on("local"), "example-org", "host-gate-1"),
            seat("seat-b", &on("peer"), "example-org", "\"{{params.seat}}\""),
        ],
        &["seat-a", "seat-b"],
    );
}

#[test]
fn one_seat_a_different_spelling_is_refused() {
    refused(
        &[
            seat("seat-a", &on("local"), "example-org", "host-gate-1"),
            seat("seat-b", &on("peer"), "example-org", "Host-Gate-1"),
        ],
        &["seat-a", "seat-b"],
    );
}

#[test]
fn one_seat_an_absent_twin_is_refused() {
    let absent = format!("{}    state: absent\n", on("peer"));
    refused(
        &[
            seat("seat-a", &on("local"), "example-org", "host-gate-1"),
            seat("seat-b", &absent, "example-org", "host-gate-1"),
        ],
        &["seat-a", "seat-b"],
    );
}

#[test]
fn one_seat_a_for_each_copy_with_one_name_is_refused() {
    let head = format!("{}    for_each: [x, y]\n", on("local"));
    refused(
        &[seat("seat", &head, "example-org", "host-gate-1")],
        &["seat-x", "seat-y"],
    );
}

#[test]
fn one_seat_control_different_names_are_accepted() {
    accepted(&[
        seat("seat-a", &on("local"), "example-org", "host-gate-1"),
        seat("seat-b", &on("peer"), "example-org", "host-gate-2"),
    ]);
}

#[test]
fn one_seat_control_one_name_in_two_repos_or_at_org_is_accepted() {
    let repo = |m: &str, r: &str| format!("    machine: {m}\n    scope: repo\n    repo: {r}\n");
    accepted(&[
        seat(
            "seat-a",
            &repo("local", "one"),
            "example-org",
            "host-gate-1",
        ),
        seat("seat-b", &repo("peer", "two"), "example-org", "host-gate-1"),
        seat("seat-c", &on("local-again"), "example-org", "host-gate-1"),
    ]);
}

// `{{item}}` in `name` is judged per copy, after expansion: two `for_each`
// resources with disjoint items are two sets of seats, not one raw string.
#[test]
fn one_seat_control_disjoint_for_each_names_are_accepted() {
    let a = format!("{}    for_each: [a, b]\n", on("local"));
    let b = format!("{}    for_each: [c, d]\n", on("peer"));
    accepted(&[
        seat("seat-l", &a, "example-org", "runner-{{item}}"),
        seat("seat-p", &b, "example-org", "runner-{{item}}"),
    ]);
}

#[test]
fn one_seat_overlapping_for_each_names_are_refused() {
    let a = format!("{}    for_each: [a, b]\n", on("local"));
    let b = format!("{}    for_each: [b, c]\n", on("peer"));
    refused(
        &[
            seat("seat-l", &a, "example-org", "runner-{{item}}"),
            seat("seat-p", &b, "example-org", "runner-{{item}}"),
        ],
        &["seat-l-b", "seat-p-b"],
    );
}

// `repo: "{{item}}"` expands per copy: one name in two repos is two seats.
#[test]
fn one_seat_control_for_each_over_repos_is_accepted() {
    let head =
        "    machine: local\n    scope: repo\n    repo: \"{{item}}\"\n    for_each: [one, two]\n";
    accepted(&[seat("seat", head, "example-org", "host-gate-1")]);
}

// An org seat must not carry a repo, templated per copy or not.
#[test]
fn one_seat_an_org_for_each_with_a_repo_is_refused() {
    let head = format!(
        "{}    repo: \"{{{{item}}}}\"\n    for_each: [one, two]\n",
        on("local")
    );
    let (out, ok) = validate(&[seat("seat", &head, "example-org", "host-gate-1")]);
    assert!(!ok, "an org seat with a repo must be refused:\n{out}");
    assert!(out.contains("`repo` must not be set"), "{out}");
}
