//! forjar#567: `lint.yml` must not be silently retargeted by a rustup directory
//! override, and must not report an environment failure as a lint failure.
//!
//! # The defect
//!
//! `rust-toolchain.toml` pins 1.93.0 with `components = ["rustfmt", "clippy"]`,
//! so a runner that honours it has clippy because the pin brings it. A rustup
//! DIRECTORY OVERRIDE outranks that file, and the clean-room fleet carries stale
//! ones on its workspace directories.
//!
//! Measured on main at 07f6d122, two runners, one commit:
//!
//! ```text
//! /home/noah/eph-build2/...              1.93.0  overridden by rust-toolchain.toml  PASS
//! /home/noah/data/actions-runner-7/...   1.89.0  directory override for ...         FAIL
//! ```
//!
//! Both toolchains were installed on the failing runner; only 1.93.0 carries
//! clippy. So `lint` was red on main for six consecutive runs while the
//! identical tree was green on a PR — which reads as "main is broken, the PR
//! fixed it", and neither half is true.
//!
//! # What this test pins, and what it deliberately does not
//!
//! It pins the two things that make the job honest, both of which are cheap to
//! delete and impossible to notice missing:
//!
//! 1. the override is UNSET before clippy runs, so the pin wins;
//! 2. a runner that cannot actually run clippy is refused BY NAME, before the
//!    lint is invoked, so an environment that CANNOT run clippy and a codebase
//!    that FAILS clippy do not exit the same way.
//!
//! On (2): the check asks `cargo clippy --version` rather than comparing the
//! active toolchain's version against the pinned channel. Three review lanes
//! independently refuted the comparison — `cut -d- -f1` turns `nightly-2026-01-01`
//! into `nightly` and `1.93.0-beta.1` into `1.93.0`, and a `channel = "stable"`
//! pin never matches a resolved version — so it was correct only for the one
//! channel spelling in use the day it was written.
//!
//! It does NOT check that the fleet has no overrides — that is runner state this
//! repository cannot see or fix, and it is tracked on the infra side. This test
//! only holds the workflow to defending itself.
//!
//! # mutation
//!
//! In .github/workflows/lint.yml, change the bare `rustup override unset`
//! COMMAND (the one at the start of a line, not the several comment lines and
//! the error message that also name it) to `rustup override list`, and
//! `lint_unsets_a_directory_override_before_running_clippy` goes RED alone.
//!
//! Measured, because the obvious phrasing of this mutation is wrong: a
//! `sed -i '/rustup override unset/d'` deletes the command AND the `::error::`
//! line that quotes it, so TWO tests go red and the address stops being
//! specific. A mutation that takes out more than it names is not an address.
//!
//! For the second arm: change `if ! cargo clippy --version; then` to
//! `if false; then` and `a_toolchain_that_disagrees_with_the_pin_is_refused_by_name`
//! goes RED alone (measured: 2 passed, 1 failed).

fn lint_yml() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/.github/workflows/lint.yml"
    ))
    .expect(".github/workflows/lint.yml must exist: it is the workflow this test is about")
}

/// JUST the `Install Rust toolchain` step: from its own `- name:` to the next
/// `- name:` of any kind.
///
/// An earlier version of this ran to `- name: Clippy`, three steps later, and a
/// review lane pointed out the function was wider than its name — it spanned the
/// unrelated `Cache cargo` step, so a substring appearing there would have
/// satisfied assertions about the toolchain step. Harmless at the time, and
/// exactly the shape that stops being harmless without anyone noticing.
fn toolchain_step(src: &str) -> &str {
    let start = src
        .find("- name: Install Rust toolchain")
        .expect("lint.yml must have an 'Install Rust toolchain' step");
    let rest = &src[start..];
    let end = rest[1..]
        .find("- name: ")
        .map(|i| i + 1)
        .expect("the toolchain step must be followed by another step");
    &rest[..end]
}

#[test]
fn lint_unsets_a_directory_override_before_running_clippy() {
    let src = lint_yml();
    let step = toolchain_step(&src);
    // ANCHORED TO THE COMMAND, NOT THE PROSE ABOUT IT.
    //
    // The first version of this asserted `step.contains("rustup override unset")`,
    // and that is vacuous here: the step's own comments and its `::error::`
    // message both name the command, so replacing the real invocation with
    // `rustup override list` left the assertion green. Measured — the intended
    // mutation did not turn this test red, which is the whole property a
    // falsification test exists to have.
    let runs_unset = step.lines().any(|l| l.trim() == "rustup override unset");
    assert!(
        runs_unset,
        "no line of the toolchain step IS the command `rustup override unset` (a mention inside a \
         comment or an error message does not count), so a stale override on a runner's workspace \
         silently retargets the whole lint job at a toolchain this repository never pinned \
         (forjar#567). The step was:\n{step}"
    );
}

#[test]
fn a_toolchain_that_disagrees_with_the_pin_is_refused_by_name() {
    let src = lint_yml();
    let step = toolchain_step(&src);
    assert!(
        step.contains("cargo clippy --version"),
        "the toolchain step does not MEASURE that clippy is runnable before the lint runs. An \
         earlier draft parsed the active toolchain's version and compared it to the pinned \
         channel; three review lanes refuted that independently, because `cut -d- -f1` cannot \
         tell a version's hyphens from a target triple's and a `channel = \"stable\"` pin never \
         matches a resolved version at all. Ask for the property, do not parse for it. The step \
         was:\n{step}"
    );
    assert!(
        step.contains("ENVIRONMENT, not lint"),
        "the toolchain step has no message distinguishing an environment failure from a lint \
         failure. UNMEASURED must not read as MEASURED-AND-BAD. The step was:\n{step}"
    );
}

#[test]
fn the_pin_still_carries_clippy() {
    // ANTI-VACUITY for the two tests above: they are only worth anything while
    // the pinned toolchain is the one that HAS clippy. If the components list
    // ever loses it, unsetting the override would retarget the job at a
    // toolchain that fails for the original reason and these tests would still
    // pass.
    let toolchain =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/rust-toolchain.toml"))
            .expect("rust-toolchain.toml must exist: it is what the override was outranking");
    assert!(
        toolchain.contains("clippy"),
        "rust-toolchain.toml no longer lists clippy in its components, so honouring the pin no \
         longer guarantees clippy exists and forjar#567's fix is vacuous:\n{toolchain}"
    );
}
