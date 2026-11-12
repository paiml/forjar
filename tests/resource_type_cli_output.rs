//! forjar#433 behavioural tests: the real `forjar` binary, with `ResourceType`
//! in its snake_case spelling. Each check below compared the string against a
//! PascalCase literal and went dead (or inverted) when Display changed.

use std::path::Path;
use std::process::Command;

fn forjar(dir: &Path, args: &[&str]) -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("run forjar");
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn config(dir: &Path, resources: &str) -> String {
    let body = format!(
        "version: '1.0'\nname: t433\nparams: {{}}\nmachines:\n  local:\n    hostname: h\n    addr: 127.0.0.1\n    user: nobody\n    arch: x86_64\nresources:\n{resources}"
    );
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, body).expect("write config");
    p.to_string_lossy().into_owned()
}

fn validate(resources: &str, flag: &str) -> String {
    let d = tempfile::tempdir().expect("tmp");
    let f = config(d.path(), resources);
    forjar(d.path(), &["validate", "-f", &f, flag])
}

#[test]
fn health_conflicts_do_not_flag_a_real_service() {
    let out = validate(
        "  s:\n    type: service\n    machine: local\n    name: nginx\n    state: running\n  d:\n    type: docker\n    machine: local\n    name: web\n    image: nginx\n    state: running\n",
        "--check-resource-health-conflicts",
    );
    assert!(
        !out.contains("s has service state"),
        "service flagged: {out}"
    );
    assert!(
        out.contains("d has service state but is type docker"),
        "{out}"
    );
}

#[test]
fn state_consistency_reports_invalid_state_for_type() {
    let out = validate(
        "  f:\n    type: file\n    machine: local\n    path: /tmp/x\n    state: file\n",
        "--check-resource-state-consistency",
    );
    assert!(out.contains("state 'file' invalid for type file"), "{out}");
}

#[test]
fn provider_support_flags_file_provider_package() {
    let out = validate(
        "  p:\n    type: package\n    machine: local\n    provider: file\n    packages: [curl]\n",
        "--check-resource-provider-support",
    );
    assert!(out.contains("cannot manage packages"), "{out}");
}

#[test]
fn idempotency_hints_flag_stateless_file() {
    let out = validate(
        "  f:\n    type: file\n    machine: local\n    path: /tmp/x\n    content: hi\n",
        "--check-resource-idempotency-hints",
    );
    assert!(
        out.contains("  f "),
        "file without state not flagged: {out}"
    );
}

#[test]
fn drift_risk_scores_file_and_service() {
    let out = validate(
        "  f:\n    type: file\n    machine: local\n    path: /tmp/x\n    content: hi\n  s:\n    type: service\n    machine: local\n    name: nginx\n",
        "--check-resource-drift-risk",
    );
    assert!(out.contains("f — risk score 4"), "file risk wrong: {out}");
    assert!(
        out.contains("s — risk score 3"),
        "service risk wrong: {out}"
    );
}

#[test]
fn mount_points_check_sees_a_mount() {
    let out = validate(
        "  m1:\n    type: mount\n    machine: local\n    path: /mnt/a\n    source: /dev/a\n  m2:\n    type: mount\n    machine: local\n    path: /mnt/a\n    source: /dev/b\n",
        "--check-mount-points",
    );
    assert!(
        out.contains("m1 <-> m2"),
        "duplicate mount path not reported: {out}"
    );
}

fn state_dir() -> tempfile::TempDir {
    let d = tempfile::tempdir().expect("tmp");
    std::fs::create_dir_all(d.path().join("web")).expect("mkdir");
    std::fs::write(
        d.path().join("web/state.lock.yaml"),
        "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  s:\n    type: service\n    status: converged\n    hash: \"blake3:a\"\n  m:\n    type: mount\n    status: converged\n    hash: \"blake3:b\"\n  d:\n    type: docker\n    status: converged\n    hash: \"blake3:c\"\n",
    )
    .expect("write lock");
    d
}

#[test]
fn status_cost_estimate_prices_by_type() {
    let d = state_dir();
    let sd = d.path().to_string_lossy().into_owned();
    let out = forjar(d.path(), &["status", "--state-dir", &sd, "--cost-estimate"]);
    assert!(out.contains("service") && out.contains("@ 3.0"), "{out}");
    assert!(out.contains("mount") && out.contains("@ 4.0"), "{out}");
    assert!(out.contains("docker") && out.contains("@ 5.0"), "{out}");
}

#[test]
fn status_resource_cost_weights_by_type() {
    let d = state_dir();
    let sd = d.path().to_string_lossy().into_owned();
    let out = forjar(d.path(), &["status", "--state-dir", &sd, "--resource-cost"]);
    // service 3.0 + mount 2.5 + docker 5.0
    assert!(out.contains("10.5"), "weighted cost wrong: {out}");
}

#[test]
fn status_lock_listing_prints_serde_spelling() {
    let d = state_dir();
    let sd = d.path().to_string_lossy().into_owned();
    let out = forjar(d.path(), &["status", "--state-dir", &sd, "--resource-size"]);
    assert!(!out.contains("Service") && !out.contains("Mount"), "{out}");
}

#[test]
fn csv_export_prints_serde_spelling() {
    let d = state_dir();
    let sd = d.path().to_string_lossy().into_owned();
    let out = forjar(d.path(), &["export", "--state-dir", &sd, "--format", "csv"]);
    assert!(
        out.contains(",service,") && out.contains(",mount,"),
        "{out}"
    );
    assert!(
        !out.contains(",Service,") && !out.contains(",Mount,"),
        "{out}"
    );
}
