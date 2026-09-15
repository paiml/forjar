//! PMAT-560: the emitted TEXT of the exec-parity fragments.
//!
//! The behavioural suite is `tests/falsification_unit_exec_parity.rs`, which
//! executes the scripts against a fake host. These pin only what the
//! behavioural suite cannot: that an undeclared service emits nothing new,
//! that the fragments compose through `verdict`, and that names are quoted.

use super::service::{apply_script, check_script, state_query_script};
use super::service_exec::{apply_tail, assertions, probe, query_lines};
use crate::core::types::{ExecParity, MachineTarget, Resource, ResourceType};

fn svc(exec_start: Option<&str>, exec_sha256: Option<&str>) -> Resource {
    Resource {
        resource_type: ResourceType::Service,
        machine: MachineTarget::Single("m1".into()),
        name: Some("a-unit".into()),
        state: Some("running".into()),
        enabled: Some(true),
        exec: ExecParity {
            exec_start: exec_start.map(str::to_string),
            exec_sha256: exec_sha256.map(str::to_string),
        },
        ..Default::default()
    }
}

#[test]
fn undeclared_emits_nothing() {
    let r = svc(None, None);
    assert!(assertions(&r, "u").is_empty());
    assert!(apply_tail(&r, "u").is_empty());
    assert!(query_lines(&r, "u").is_empty());
    for s in [check_script(&r), apply_script(&r), state_query_script(&r)] {
        assert!(!s.contains("ExecStart"), "{s}");
        assert!(!s.contains("sha256sum"), "{s}");
    }
}

#[test]
fn the_probe_quotes_the_unit_name() {
    let p = probe("it's-a-unit");
    assert!(p.contains("--value 'it'\"'\"'s-a-unit'"), "{p}");
    assert!(p.contains("sha256sum"), "{p}");
    assert!(
        !p.contains("head"),
        "no head under pipefail (PMAT-240): {p}"
    );
}

#[test]
fn a_path_alone_asserts_the_path_only() {
    let a = assertions(&svc(Some("/opt/x/run.sh"), None), "u");
    assert_eq!(a.len(), 2, "probe + one assertion: {a:?}");
    let joined = a.join("\n");
    assert!(joined.contains("exec_start:u:/opt/x/run.sh"), "{joined}");
    assert!(!joined.contains("exec_sha256"), "{joined}");
}

#[test]
fn a_digest_alone_asserts_the_digest_only() {
    let d = "0".repeat(64);
    let a = assertions(&svc(None, Some(&d)), "u");
    assert_eq!(a.len(), 2, "{a:?}");
    let joined = a.join("\n");
    assert!(joined.contains(&format!("exec_sha256:u:{d}")), "{joined}");
    assert!(!joined.contains("exec_start"), "{joined}");
}

#[test]
fn the_check_still_asserts_presence_first() {
    let s = check_script(&svc(Some("/opt/x/run.sh"), None));
    let active = s.find("is-active").expect("active");
    let exec = s.find("ExecStart").expect("exec");
    assert!(active < exec, "presence before parity: {s}");
    assert!(s.trim_end().ends_with("exit \"$__fj_diverged\""), "{s}");
}

#[test]
fn the_apply_tail_exits_one_on_divergence() {
    let s = apply_script(&svc(Some("/opt/x/run.sh"), None));
    assert!(s.contains("__fj_diverged=0"), "{s}");
    assert!(s.contains("FORJAR_FAIL: a-unit executes"), "{s}");
    assert!(s.trim_end().ends_with("exit 1\nfi"), "{s}");
}

#[test]
fn the_query_reports_both_facts_once_declared() {
    let s = state_query_script(&svc(None, Some(&"0".repeat(64))));
    assert!(s.contains("echo \"exec_start=$__fj_unit_prog\""), "{s}");
    assert!(s.contains("echo \"exec_sha256=$__fj_unit_sha\""), "{s}");
}
