//! forjar#1000 (phase 1): the `github_runner` resource's manifest contract, C6
//! of `contracts/github-runner-v1.yaml`.
//!
//! `scope_is_validated`: `scope: repo` without `repo`, `scope: org` with
//! `repo`, an empty label list and a missing `sha256` each fail `forjar
//! validate`, and the refusal names the field. A manifest that is correct in
//! all four respects is accepted, so a validator that refuses everything does
//! not discharge the four refusals.
//!
//! Every case drives the real binary over a tempdir fixture, because the
//! property is what `forjar validate` prints and exits with, not what a library
//! function returns.
//!
//! Phase 2 (registration against a local fake of the GitHub API) adds the
//! falsifiers for C1 to C5:
//!
//! * F1 (C1): apply and check with a marker key; the marker, the JWT and the
//!   fake's token appear nowhere in stdout, stderr, the state dir or the event
//!   log.
//! * F2 (C2): the fake lists the seat with equal labels in another order; the
//!   seat receives 0 commands.
//! * F3 (C3): the fake lists the seat with one label fewer; apply exits
//!   non-zero, names the missing label, and the fake records 0
//!   registration-token requests.
//! * F4 (C4): the fake lists nothing; one token request, then the fake lists
//!   the seat; the second apply makes 0 requests for a token.
//! * F5 (C5): a tarball with one byte changed is refused and `config.sh` is
//!   never executed.
//!
//! They are not written here as ignored or `unimplemented!()` stubs: this
//! repository carries no skipped test and no known-failure row on a PR path,
//! and a stub that cannot fail is the vacuous green that rule exists to
//! prevent. They land with the fake they need.
//!
//! Usage: cargo test --test falsification_github_runner

use std::path::Path;
use std::process::Command;

/// A well-formed `github_runner` resource, one `key: value` line per entry
/// (nested keys carry their own indent). Each case below changes exactly one of
/// these and nothing else.
const SEAT: &[&str] = &[
    "    type: github_runner",
    "    machine: local",
    "    scope: org",
    "    owner: example-org",
    "    name: host-gate-1",
    "    labels: [self-hosted, linux, x64, gate]",
    "    group: default",
    "    version: 2.330.0",
    "    sha256: 0000000000000000000000000000000000000000000000000000000000000000",
    "    work_dir: /var/lib/runner/gate-1",
    "    user: runner",
    "    app:",
    "      app_id: 123456",
    "      installation_id: 7890123",
    "      private_key: \"{{secrets.runner-app-pem}}\"",
];

/// Write a manifest holding one seat built from [`SEAT`] with `edit` applied to
/// its lines, and return the manifest's path.
fn manifest(dir: &Path, edit: impl FnOnce(&mut Vec<String>)) -> String {
    let secrets = dir.join("secrets");
    std::fs::create_dir_all(&secrets).unwrap();
    std::fs::write(secrets.join("runner-app-pem"), "example-marker-key").unwrap();

    let mut seat: Vec<String> = SEAT.iter().map(|l| (*l).to_string()).collect();
    edit(&mut seat);

    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: '1.0'\n\
             name: github-runner-seat\n\
             secrets:\n  provider: file\n  path: \"{secrets}\"\n\
             machines:\n  local:\n    hostname: localhost\n    addr: localhost\n    transport: local\n\
             resources:\n  gate-seat-1:\n{seat}\n",
            secrets = secrets.display(),
            seat = seat.join("\n"),
        ),
    )
    .unwrap();
    cfg.display().to_string()
}

/// `forjar validate -f <cfg>`: combined stdout and stderr, and whether it
/// exited 0.
fn validate(dir: &Path, cfg: &str) -> (String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(dir)
        .args(["validate", "-f", cfg])
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

/// Replace the seat line starting with `key` (e.g. `    scope:`), or drop it
/// when `replacement` is `None`.
fn set(seat: &mut Vec<String>, key: &str, replacement: Option<&str>) {
    let at = seat
        .iter()
        .position(|l| l.starts_with(key))
        .unwrap_or_else(|| panic!("fixture has no `{key}` line"));
    match replacement {
        Some(line) => seat[at] = line.to_string(),
        None => {
            seat.remove(at);
        }
    }
}

/// Assert `validate` refused the manifest and named each of `fields` (in
/// backticks, as the message writes a field) and the resource.
fn assert_refused_naming(out: &str, ok: bool, fields: &[&str]) {
    assert!(!ok, "validate accepted an invalid github_runner:\n{out}");
    assert!(
        out.contains("gate-seat-1") && out.contains("github_runner"),
        "the refusal must name the resource and its type:\n{out}"
    );
    for field in fields {
        assert!(
            out.contains(&format!("`{field}`")),
            "the refusal must name the field `{field}`:\n{out}"
        );
    }
}

/// Control: all four rules satisfied is accepted. Without it, "validate
/// refuses every github_runner" would discharge the four refusals below.
#[test]
fn f6_control_a_valid_seat_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = manifest(dir.path(), |_| {});
    let (out, ok) = validate(dir.path(), &cfg);
    assert!(ok, "a valid github_runner must pass validate:\n{out}");
}

/// Control: `scope: repo` WITH `repo` is accepted, so the repo-scope rule is a
/// conditional and not a ban on `scope: repo`.
#[test]
fn f6_control_a_repo_scoped_seat_with_a_repo_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = manifest(dir.path(), |seat| {
        set(seat, "    scope:", Some("    scope: repo"));
        set(seat, "    group:", None);
        seat.push("    repo: example-repo".to_string());
    });
    let (out, ok) = validate(dir.path(), &cfg);
    assert!(ok, "a repo-scoped seat with a repo must pass:\n{out}");
}

#[test]
fn f6_scope_repo_without_repo_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = manifest(dir.path(), |seat| {
        set(seat, "    scope:", Some("    scope: repo"));
        set(seat, "    group:", None);
    });
    let (out, ok) = validate(dir.path(), &cfg);
    assert_refused_naming(&out, ok, &["scope", "repo"]);
}

#[test]
fn f6_scope_org_with_repo_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = manifest(dir.path(), |seat| {
        seat.push("    repo: example-repo".to_string());
    });
    let (out, ok) = validate(dir.path(), &cfg);
    assert_refused_naming(&out, ok, &["scope", "repo"]);
}

#[test]
fn f6_empty_label_list_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = manifest(dir.path(), |seat| {
        set(seat, "    labels:", Some("    labels: []"));
    });
    let (out, ok) = validate(dir.path(), &cfg);
    assert_refused_naming(&out, ok, &["labels"]);
}

#[test]
fn f6_missing_sha256_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = manifest(dir.path(), |seat| {
        set(seat, "    sha256:", None);
    });
    let (out, ok) = validate(dir.path(), &cfg);
    assert_refused_naming(&out, ok, &["sha256"]);
}

// Injection falsifier: a `$(...)` payload in `sha256` is refused at validate
// and never runs anything.
#[test]
fn f6_sha256_must_be_64_hex_and_a_payload_never_runs() {
    for bad in ["\"$(touch MARKER)\"", "abc123", "\"`touch MARKER`\""] {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("MARKER");
        let payload = bad.replace("MARKER", &marker.display().to_string());
        let cfg = manifest(dir.path(), |seat| {
            set(seat, "    sha256:", Some(&format!("    sha256: {payload}")));
        });
        let (out, ok) = validate(dir.path(), &cfg);
        assert_refused_naming(&out, ok, &["sha256"]);
        assert!(out.contains("64 hex"), "{out}");
        assert!(!marker.exists(), "the payload ran: {payload}");
    }
}
