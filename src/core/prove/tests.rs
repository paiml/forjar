//! Falsification tests for `forjar prove` (L2 of `provable-iac-v1`).
#![allow(clippy::unwrap_used)]

use crate::core::parser::parse_config;
use crate::core::prove::{prove, Assurance, InvariantResult, ProofReport};

fn report_with(machines: &str, resources: &str) -> ProofReport {
    let yaml = format!("version: \"1.0\"\nname: t\nmachines:\n{machines}resources:\n{resources}");
    let cfg = parse_config(&yaml).unwrap_or_else(|e| panic!("parse failed: {e}\n---\n{yaml}"));
    prove(&cfg, "test.yaml")
}
fn report(resources: &str) -> ProofReport {
    report_with("  m1:\n    hostname: m1\n    addr: 1.2.3.4\n", resources)
}
fn inv<'a>(r: &'a ProofReport, id: &str) -> &'a InvariantResult {
    r.invariants.iter().find(|i| i.id == id).unwrap()
}

#[test]
fn acyclic_config_proves_i1() {
    let r = report(
        "  a: { type: file, machine: m1, path: /etc/a, content: x }\n  b: { type: file, machine: m1, path: /etc/b, content: y, depends_on: [a] }\n",
    );
    assert_eq!(inv(&r, "I1").state, Assurance::Proved);
    assert!(r.passed());
}

#[test]
fn cycle_falsifies_i1_and_blocks() {
    let r = report(
        "  a: { type: file, machine: m1, path: /etc/a, content: x, depends_on: [b] }\n  b: { type: file, machine: m1, path: /etc/b, content: y, depends_on: [a] }\n",
    );
    assert_eq!(inv(&r, "I1").state, Assurance::Falsified);
    assert!(!r.passed(), "a cycle must block (HARD)");
}

#[test]
fn dangling_dep_falsifies_i2() {
    let r =
        report("  a: { type: file, machine: m1, path: /etc/a, content: x, depends_on: [ghost] }\n");
    assert_eq!(inv(&r, "I2").state, Assurance::Falsified);
    assert!(!r.passed());
}

#[test]
fn two_files_same_path_falsify_i3() {
    let r = report(
        "  a: { type: file, machine: m1, path: /etc/motd, content: x }\n  b: { type: file, machine: m1, path: /etc/motd, content: y }\n",
    );
    assert_eq!(inv(&r, "I3").state, Assurance::Falsified);
}

#[test]
fn same_path_different_machines_is_not_a_conflict() {
    let r = report_with(
        "  m1:\n    hostname: m1\n    addr: 1.2.3.4\n  m2:\n    hostname: m2\n    addr: 5.6.7.8\n",
        "  a: { type: file, machine: m1, path: /etc/motd, content: x }\n  b: { type: file, machine: m2, path: /etc/motd, content: y }\n",
    );
    assert_ne!(inv(&r, "I3").state, Assurance::Falsified);
}

#[test]
fn opaque_resource_downgrades_i3_to_unknown() {
    let r = report(
        "  a: { type: file, machine: m1, path: /etc/a, content: x }\n  job: { type: cron, machine: m1, name: nightly, content: run.sh }\n",
    );
    assert!(r.has_opaque());
    assert_eq!(inv(&r, "I3").state, Assurance::Unknown);
}

#[test]
fn file_content_with_shell_syntax_does_not_falsify_i9() {
    // A file's content is its legitimate payload — $(...) here is NOT impurity.
    let r = report(
        "  z: { type: file, machine: m1, path: /etc/zshrc, content: \"eval $(brew shellenv)\" }\n",
    );
    assert_ne!(inv(&r, "I9").state, Assurance::Falsified);
    assert!(r.passed(), "legitimate shell content must not block");
}

#[test]
fn unpinned_package_is_advisory_not_blocking() {
    let r = report("  p: { type: package, machine: m1, packages: [ripgrep] }\n");
    let i9 = inv(&r, "I9");
    assert_eq!(i9.state, Assurance::Unknown, "unpinned = advisory Unknown");
    assert!(
        r.passed(),
        "an unpinned package must NOT block apply (advisory)"
    );
}

#[test]
fn plan_hash_is_stable_and_order_independent() {
    let a = report(
        "  a: { type: file, machine: m1, path: /etc/a, content: x }\n  b: { type: file, machine: m1, path: /etc/b, content: y }\n",
    );
    let b = report(
        "  b: { type: file, machine: m1, path: /etc/b, content: y }\n  a: { type: file, machine: m1, path: /etc/a, content: x }\n",
    );
    assert_eq!(
        a.plan_hash, b.plan_hash,
        "plan-hash must be order-independent"
    );
}
