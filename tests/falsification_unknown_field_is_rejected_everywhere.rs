//! An unknown field must fail every verb that consumes a config, not just `validate`.
//!
//! THE FLAW THIS CLOSES (paiml/forjar#272).
//!
//! `validate` carried its own inline unknown-field check that hard-errors, with
//! a comment naming the stakes exactly right: "P0 — silent data loss". Every
//! other verb — plan, apply, drift, codegen, prove, cbom, the MCP handlers —
//! went through `parse_and_validate()`, which is `parse_and_validate_opts(path,
//! false)`: unknown fields printed as a warning and then DISCARDED.
//!
//! Discarded is the operative word. The field does not merely go unreported —
//! it is dropped, and the typed field it was meant to set falls back to its
//! default. Measured in paiml/infra 2026-08-20: a CIFS mount declared
//! `fs_type: cifs` (the field is `fstype`). `validate` exited 3 with a helpful
//! "did you mean 'fstype'?"; `plan` on the same file reported
//! `1 to add ... mount /mnt/unas` and would have written an fstab line with a
//! filesystem type the author never asked for.
//!
//! The asymmetry is what makes it dangerous: `validate` is the ONLY verb that
//! catches this, and it is the one verb not in the plan -> apply path the tool
//! itself encourages. Anyone following the documented workflow never sees it.
//!
//! WHAT THIS TEST MUST NOT BECOME. Asserting that stderr contains the word
//! "unknown" would pass today — the warning is already printed. The defect is
//! that the command SUCCEEDS anyway, so every case here asserts on the exit
//! status and, for apply, on whether the host was touched.

use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// A config whose only defect is one misspelled field on an otherwise valid
/// resource. `fs_type` is the real typo that exposed this; the correct spelling
/// is `fstype`.
fn config_with_typo(dir: &std::path::Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        r#"version: "1.0"
name: unknown-field
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
resources:
  a-mount:
    type: mount
    machine: local
    source: "//198.51.100.9/share"
    path: /tmp/forjar-unknown-field-probe
    fs_type: cifs
"#,
    )
    .unwrap();
    cfg
}

/// The same config with the field spelled correctly, to prove the tests above
/// fail on the TYPO and not merely on the resource being a mount.
fn config_without_typo(dir: &std::path::Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        r#"version: "1.0"
name: unknown-field
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
resources:
  a-mount:
    type: mount
    machine: local
    source: "//198.51.100.9/share"
    path: /tmp/forjar-unknown-field-probe
    fstype: cifs
"#,
    )
    .unwrap();
    cfg
}

#[test]
fn validate_rejects_the_unknown_field() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config_with_typo(dir.path());
    let out = forjar()
        .args(["validate", "-f", cfg.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "validate must reject an unknown field; this is the behaviour the other \
         verbs are being brought into line with"
    );
}

#[test]
fn plan_rejects_the_unknown_field_too() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config_with_typo(dir.path());
    let out = forjar()
        .args(["plan", "-f", cfg.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "plan accepted a config validate rejects. stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Apply must REFUSE, and must not have written anything.
///
/// This case is deliberately built on a `file` resource rather than the mount
/// above. The first version of this test used the mount and passed BEFORE the
/// fix — not because apply rejected the unknown field, but because
/// `mount.cifs` cannot mount 198.51.100.9 and failed for its own reasons:
///
///   JIDOKA: local/a-mount failed — exit code 1: mount.cifs: permission denied
///
/// A test that goes green while the defect is fully present is worse than no
/// test. A `file` resource applies successfully on any host, so the ONLY thing
/// that can make this fail is the config being rejected — and the side-effect
/// assertion pins it down: with the bug, apply writes the file and exits 0.
#[test]
fn apply_rejects_the_unknown_field_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("written-by-apply");
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: unknown-field
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
resources:
  a-file:
    type: file
    machine: local
    path: "{}"
    content: "written\n"
    contents: "this field does not exist"
"#,
            target.display()
        ),
    )
    .unwrap();

    let out = forjar()
        .args([
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--yes",
            "--no-tripwire",
            "--state-dir",
            dir.path().join("state").to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        !out.status.success(),
        "apply accepted a config validate rejects — and this is the WRITE path.          stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !target.exists(),
        "apply refused with a non-zero exit but still wrote {} — refusing must          happen before any host is touched",
        target.display()
    );
}

#[test]
fn codegen_rejects_the_unknown_field_too() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config_with_typo(dir.path());
    let out = forjar()
        .args(["codegen", "-f", cfg.to_str().unwrap(), "-r", "a-mount"])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "codegen emitted shell for a config validate rejects — the emitted \
         script would carry the defaulted field. stdout:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// The counter-case. Without this, every assertion above would also pass if the
/// commands had simply been broken for all mount configs.
#[test]
fn the_same_config_spelled_correctly_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config_without_typo(dir.path());
    for verb in [
        vec!["validate", "-f", cfg.to_str().unwrap()],
        vec!["plan", "-f", cfg.to_str().unwrap()],
    ] {
        let out = forjar().args(&verb).output().unwrap();
        assert!(
            out.status.success(),
            "`{}` rejected a VALID config — the fix has over-reached. stderr:\n{}",
            verb[0],
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
