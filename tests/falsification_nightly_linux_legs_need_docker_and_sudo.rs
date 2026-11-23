//! forjar#1236: the nightly must not depend on which fleet runner picks a Linux leg.
//!
//! # The defect
//!
//! `[self-hosted, clean-room, X64]` matches the bare-metal intel runners AND
//! containerised/tier2 runners. The `aarch64-*` legs build with `cross`, which
//! needs the docker socket, and the `x86_64-musl` leg runs `sudo apt-get`.
//! Measured on paiml/forjar Nightly, 2026-11-20..21: the musl leg died
//! `sudo: The "no new privileges" flag is set` (run 44737069851) and an aarch64
//! leg died `permission denied while trying to connect to the docker API`
//! (run 44902991903). One red leg skips `release`, so the `nightly` tag stayed
//! on an old commit and fleet-bins raised NIGHTLY-STALE. A fourth failure was
//! docker `lease does not exist` inside `cross build` on an intel runner
//! (run 44863879433, exit 125), a daemon flake that a single retry clears.
//!
//! # mutations -- each measured to redden this test
//!
//! 1. Drop `intel` from any Linux leg's runner in `nightly.yml` -> RED.
//! 2. Remove the exit-125 retry from the `cross build` step -> RED.

use serde_yaml_ng::Value;

fn nightly() -> Value {
    let path = format!(
        "{}/.github/workflows/nightly.yml",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_yaml_ng::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

#[test]
fn every_linux_nightly_leg_is_pinned_to_bare_metal() {
    let wf = nightly();
    let legs = wf["jobs"]["build"]["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("nightly.yml build matrix");
    let mut checked = 0;
    for leg in legs {
        let target = leg["target"].as_str().expect("leg target");
        if !target.contains("-linux-") {
            continue;
        }
        checked += 1;
        let labels: Vec<&str> = leg["runner"]
            .as_sequence()
            .expect("leg runner labels")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert!(
            labels.contains(&"intel"),
            "nightly leg {target} runs on {labels:?}: it needs docker (cross) or sudo (musl-tools), \
             which only the bare-metal `intel` runners carry (forjar#1236)"
        );
    }
    assert_eq!(
        checked, 4,
        "vacuity guard: expected four Linux legs, saw {checked}"
    );
}

#[test]
fn cross_build_retries_a_docker_daemon_failure() {
    let text = std::fs::read_to_string(format!(
        "{}/.github/workflows/nightly.yml",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("read nightly.yml");
    let step = text
        .split("- name: Build release binary")
        .nth(1)
        .and_then(|s| s.split("- name: Package (unix)").next())
        .expect("Build release binary step");
    assert!(
        step.contains("cross build") && step.contains("125"),
        "the cross build must retry once on docker's exit 125 (daemon lease flake, forjar#1236)"
    );
}
