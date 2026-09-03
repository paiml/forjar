//! forjar#374, round two: the three doors the quorum review found after the
//! first tree — `--check --abort-on-drift` is not a read, the operator gate
//! honours `-m`, and the fleet leg of `--canary-machine` asks once.

#[path = "common/canary_authz.rs"]
mod harness;
use harness::*;

// ── Quorum review of #374: three more doors ────────────────────────────────

/// `--abort-on-drift` runs `cmd_drift` from `apply_pre_checks`, which probes
/// every host in scope. A probe is an execution against the fleet whatever flag
/// sits next to it, so `--check --abort-on-drift` is not a read.
#[test]
fn a_check_with_abort_on_drift_is_not_a_read() {
    let sb = Sandbox::fleet("check-abort-on-drift");
    let out = sb.apply(Some("mallory"), &["--check", "--abort-on-drift"]);
    refused(&out, "`apply --check --abort-on-drift --operator mallory`");
    assert!(sb.nothing_was_written(), "a refused read wrote files");
}

/// The gate checks the machines the invocation TOUCHES. An operator listed on
/// `sandbox` alone keeps `apply -m sandbox`, loses `apply -m prod`, and loses
/// the unscoped apply that would touch both.
#[test]
fn a_machine_scoped_operator_keeps_their_own_machine() {
    let sb = Sandbox::new(
        "scoped-operator",
        &[("sandbox", &["bob"]), ("prod", &["alice"])],
    );
    let own = sb.apply(Some("bob"), &["--yes", "-m", "sandbox"]);
    assert!(
        own.status.success(),
        "bob is listed on sandbox and was refused `apply -m sandbox`:\nstderr: {}",
        stderr(&own)
    );
    assert!(
        sb.canary_file().exists(),
        "bob's own machine did not converge"
    );
    assert!(!sb.prod_file().exists(), "a -m sandbox apply touched prod");

    let other = sb.apply(Some("bob"), &["--yes", "-m", "prod"]);
    refused(&other, "`apply -m prod --operator bob`");
    assert!(!sb.prod_file().exists(), "bob converged prod");

    let all = sb.apply(Some("bob"), &["--yes"]);
    refused(&all, "`apply --operator bob` (unscoped, touches prod)");
    assert!(
        !sb.prod_file().exists(),
        "an unscoped apply converged prod for bob"
    );
}

/// The fleet leg asks ONCE. With three machines and exactly two "y" answers on
/// stdin — one for the canary leg, one for the fleet — both remaining machines
/// converge; per-machine prompting would have hit EOF on the third and left
/// `prod2` unconverged.
#[test]
fn the_fleet_leg_asks_once_not_once_per_machine() {
    let sb = Sandbox::new(
        "fleet-asks-once",
        &[
            ("sandbox", &["alice"]),
            ("prod", &["alice"]),
            ("prod2", &["alice"]),
        ],
    );
    // A third managed file, on the third machine.
    let d = sb.dir.display().to_string();
    let mut yaml = std::fs::read_to_string(sb.cfg()).unwrap();
    yaml.push_str(&format!(
        "  prod2_file:\n    type: file\n    machine: prod2\n    \
         path: {d}/prod2.txt\n    content: \"prod2\"\n"
    ));
    std::fs::write(sb.cfg(), yaml).unwrap();

    let out = sb.apply_with_stdin(Some("alice"), &["--canary-machine", "sandbox"], "y\ny\n");
    assert!(
        sb.canary_file().exists(),
        "the canary leg did not converge:\n{}",
        stdout(&out)
    );
    assert!(
        sb.prod_file().exists() && sb.dir.join("prod2.txt").exists(),
        "two answers, three machines: the fleet leg asked per machine and ran out of \
         stdin before prod2.\nexit {:?}\nstdout: {}\nstderr: {}",
        out.status.code(),
        stdout(&out),
        stderr(&out)
    );
    assert_eq!(
        stderr(&out).matches("remaining machine(s)?").count(),
        1,
        "the fleet confirmation must appear exactly once"
    );
}
