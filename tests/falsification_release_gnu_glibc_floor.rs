//! forjar#1032: the x86_64-unknown-linux-gnu release asset must not need GLIBC_2.39.
//!
//! # The defect
//!
//! `release.yml` built the x86_64 gnu leg natively on the Ubuntu 24.04 clean-room
//! runner, so the binary recorded a `GLIBC_2.39` version node and refused to load on
//! older glibc. The aarch64 gnu legs use `cross` (old sysroot image, 2.18 floor).
//! `install.sh` also handed every glibc host the gnu asset without a version check.
//!
//! # mutations — each measured to redden this test
//!
//! 1. Drop `x86_64-unknown-linux-gnu` from the `cross build` arm of the build step → RED.
//! 2. Drop the x86_64 gnu case from the `cross` install prerequisite → RED.
//! 3. Make `detect_libc` return `gnu` for every glibc host again → the shell test is RED.

use std::process::Command;

fn release_text() -> String {
    let path = format!(
        "{}/.github/workflows/release.yml",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

/// The `case` pattern line that precedes `needle` in the step whose text starts at `step`.
fn arm_before(text: &str, step: &str, needle: &str) -> String {
    let start = text
        .find(step)
        .unwrap_or_else(|| panic!("release.yml has no step {step:?}"));
    let body = &text[start..];
    let at = body
        .find(needle)
        .unwrap_or_else(|| panic!("step {step:?} has no {needle:?}"));
    body[..at]
        .lines()
        .rev()
        .find(|l| l.trim_end().ends_with(')'))
        .unwrap_or("")
        .to_owned()
}

#[test]
fn x86_64_gnu_is_built_with_cross() {
    let text = release_text();
    let arm = arm_before(&text, "- name: Build release binaries", "cross build");
    assert!(
        arm.contains("x86_64-unknown-linux-gnu") && arm.contains("aarch64-unknown-linux-"),
        "the `cross build` arm must cover x86_64-unknown-linux-gnu and aarch64 linux; read {arm:?}"
    );
}

#[test]
fn x86_64_gnu_gets_the_cross_prerequisite() {
    let text = release_text();
    let arm = arm_before(
        &text,
        "- name: Install target prerequisites (Linux)",
        "cargo install cross",
    );
    assert!(
        arm.contains("x86_64-unknown-linux-gnu") && arm.contains("aarch64-"),
        "the cross install arm must cover x86_64-unknown-linux-gnu; read {arm:?}"
    );
}

#[test]
fn release_gate_content_unchanged_no_continue_on_error() {
    assert!(
        !release_text().contains("continue-on-error"),
        "release.yml must carry no continue-on-error"
    );
}

#[test]
fn install_sh_picks_musl_below_the_glibc_floor() {
    let out = Command::new("bash")
        .arg(format!(
            "{}/tests/install_glibc_floor.sh",
            env!("CARGO_MANIFEST_DIR")
        ))
        .output()
        .expect("run tests/install_glibc_floor.sh");
    assert!(
        out.status.success(),
        "install_glibc_floor.sh failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
