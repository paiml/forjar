//! What a generation records, and what `undo` must refuse when it cannot.
//!
//! The first fix for #376 recorded the RAW BYTES of the config file the
//! operator named. Three adversarial verifiers showed that captures less than
//! the desired state, and that `undo` then converged the host FORWARDS while
//! exiting 0 and printing "1 converged" — the original defect's exact
//! signature, surviving its own fix in three shapes:
//!
//! | shape | what escaped the snapshot | measured symptom |
//! |---|---|---|
//! | `includes:` | the included bodies, re-read live at replay | host stayed on the newest version |
//! | `-p ver=…` | overrides, merged at apply and never written down | host landed on the param DEFAULT — bytes no generation ever held |
//! | `source:` | the payload FILE the config points at | host stayed on the newest payload, lock stamped with the OLD hash |
//!
//! TWO DIFFERENT REMEDIES, and the difference is the point.
//!
//! `includes:` and `-p` are already resolved in the config VALUE that apply
//! holds: includes are merged during parse, and `apply_param_overrides` runs
//! before the generation is recorded. Serialising that value instead of
//! re-reading the file captures both for free, so those two are FIXED — undo
//! reverts them correctly.
//!
//! `source:` is not fixable that way. The bytes live in a file outside the
//! config, and a generation records the config, not the tree around it. So undo
//! REFUSES: jidoka — a machine that cannot do the job correctly stops and
//! signals rather than producing a defective part. A refusal that leaves the
//! host untouched is a correct outcome here; exiting 0 having silently moved
//! the host the wrong way is not.
//!
//! WHY THE ORACLE IS THE BYTES AT THE PATH. Every assertion reads the declared
//! file. `--force` is on during undo's replay, so "1 converged" prints whether
//! or not different bytes were written — the summary is the defect's own
//! output and cannot be the oracle.

use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

fn run(args: &[&str]) -> (i32, String) {
    let out = forjar().args(args).output().unwrap();
    let mut merged = String::from_utf8_lossy(&out.stdout).into_owned();
    merged.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), merged)
}

fn apply_with(cfg: &Path, state: &Path, extra: &[&str]) -> (i32, String) {
    let cfg = cfg.display().to_string();
    let state = state.display().to_string();
    let mut args = vec!["apply", "-f", &cfg, "--state-dir", &state, "--yes"];
    args.extend_from_slice(extra);
    run(&args)
}

fn undo(cfg: &Path, state: &Path) -> (i32, String) {
    run(&[
        "undo",
        "-f",
        &cfg.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--yes",
    ])
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

// ---------------------------------------------------------------- includes --

/// A stack whose resources live in an include, so the top-level file names
/// almost nothing that matters.
fn write_include_stack(dir: &Path, content: &str) -> PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        r#"version: "1.0"
name: inc-stack
policy:
  snapshot_generations: 10
machines:
  box:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
includes: ["resources.yaml"]
"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("resources.yaml"),
        format!(
            r#"version: "1.0"
name: inc-stack
resources:
  demo_file:
    type: file
    machine: box
    path: {}
    content: "{content}\n"
"#,
            dir.join("demo.txt").display()
        ),
    )
    .unwrap();
    cfg
}

#[test]
fn an_included_resource_is_reverted_like_any_other() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let state = d.join("state");

    for v in ["v1", "v2", "v3"] {
        let cfg = write_include_stack(d, v);
        let (rc, out) = apply_with(&cfg, &state, &[]);
        assert_eq!(rc, 0, "apply {v} failed:\n{out}");
    }
    let cfg = d.join("forjar.yaml");
    assert_eq!(read(&d.join("demo.txt")), "v3\n");

    let (rc, out) = undo(&cfg, &state);
    assert_eq!(rc, 0, "undo should succeed:\n{out}");
    assert_eq!(
        read(&d.join("demo.txt")),
        "v2\n",
        "the resource came from an include, and undo must revert it like any \
         other — recording only the top-level file left the include to be \
         re-read LIVE, so the host never moved:\n{out}"
    );
}

#[test]
fn an_includes_stack_records_generations_at_all() {
    // The include merge replaced the base's policy block wholesale, and an
    // include silent about policy handed over a DEFAULT one — wiping
    // `snapshot_generations`. No generation was recorded, so undo refused with
    // "does not enable them" against a config that plainly did (#379).
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let state = d.join("state");
    let cfg = write_include_stack(d, "v1");
    let (rc, out) = apply_with(&cfg, &state, &[]);
    assert_eq!(rc, 0, "apply failed:\n{out}");

    assert!(
        state.join("generations").is_dir(),
        "policy.snapshot_generations is set in the base config; an include that \
         says nothing about policy must not silently erase it:\n{out}"
    );
}

// ------------------------------------------------------------------ params --

fn write_param_stack(dir: &Path) -> PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: param-stack
policy:
  snapshot_generations: 10
params:
  ver: "THE-DEFAULT"
machines:
  box:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
resources:
  demo_file:
    type: file
    machine: box
    path: {}
    content: "{{{{params.ver}}}}\n"
"#,
            dir.join("demo.txt").display()
        ),
    )
    .unwrap();
    cfg
}

#[test]
fn a_param_override_is_recorded_so_undo_does_not_fall_back_to_the_default() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let state = d.join("state");
    let cfg = write_param_stack(d);

    for v in ["v1", "v2", "v3"] {
        let (rc, out) = apply_with(&cfg, &state, &["-p", &format!("ver={v}")]);
        assert_eq!(rc, 0, "apply {v} failed:\n{out}");
    }
    assert_eq!(read(&d.join("demo.txt")), "v3\n");

    let (rc, out) = undo(&cfg, &state);
    assert_eq!(rc, 0, "undo should succeed:\n{out}");
    let got = read(&d.join("demo.txt"));
    assert_ne!(
        got, "THE-DEFAULT\n",
        "undo re-resolved {{{{params.ver}}}} to its default, landing the host on \
         bytes NO generation ever held — worse than not undoing:\n{out}"
    );
    assert_eq!(
        got, "v2\n",
        "undo must restore the value the target generation was applied with:\n{out}"
    );
}

// ------------------------------------------------------------------ source --

#[test]
fn a_source_backed_resource_is_refused_rather_than_converged_forward() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let state = d.join("state");
    let payload = d.join("payload.txt");
    let dst = d.join("dst.txt");

    let cfg = d.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: src-stack
policy:
  snapshot_generations: 10
machines:
  box:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
resources:
  from_source:
    type: file
    machine: box
    path: {}
    source: {}
"#,
            dst.display(),
            payload.display()
        ),
    )
    .unwrap();

    for p in ["P1", "P2", "P3"] {
        std::fs::write(&payload, format!("{p}\n")).unwrap();
        let (rc, out) = apply_with(&cfg, &state, &[]);
        assert_eq!(rc, 0, "apply {p} failed:\n{out}");
    }
    assert_eq!(read(&dst), "P3\n");

    let lock = state.join("box").join("state.lock.yaml");
    let lock_before = read(&lock);

    let (rc, out) = undo(&cfg, &state);

    assert_ne!(
        rc, 0,
        "the payload bytes live outside the config and the generation never \
         captured them, so replaying an old config against the CURRENT payload \
         would converge the host forwards. Undo must refuse:\n{out}"
    );
    assert_eq!(
        read(&dst),
        "P3\n",
        "a refusal must leave the host exactly as it was:\n{out}"
    );
    assert_eq!(
        read(&lock),
        lock_before,
        "a refusal must not stamp the lock — that is what made the corruption \
         self-consistent, so `drift` then reported clean over the wrong bytes:\n{out}"
    );
    assert!(
        out.contains("from_source"),
        "the refusal must name the resource that cannot be replayed:\n{out}"
    );
}

/// THE OVER-CORRECTION GUARD. Refusing every undo would satisfy the test above
/// and destroy the feature. An inline `content:` resource carries its bytes in
/// the config, so the generation captures them and undo must still work.
#[test]
fn an_inline_content_resource_is_never_refused() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let state = d.join("state");
    let cfg = d.join("forjar.yaml");

    for v in ["v1", "v2", "v3"] {
        std::fs::write(
            &cfg,
            format!(
                r#"version: "1.0"
name: inline-stack
policy:
  snapshot_generations: 10
machines:
  box:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
resources:
  demo_file:
    type: file
    machine: box
    path: {}
    content: "{v}\n"
"#,
                d.join("demo.txt").display()
            ),
        )
        .unwrap();
        let (rc, out) = apply_with(&cfg, &state, &[]);
        assert_eq!(rc, 0, "apply {v} failed:\n{out}");
    }

    let (rc, out) = undo(&cfg, &state);
    assert_eq!(
        rc, 0,
        "an inline-content stack is fully captured by the generation; refusing \
         it would trade the bug for a dead feature:\n{out}"
    );
    assert_eq!(read(&d.join("demo.txt")), "v2\n", "{out}");
}

// ------------------------------------------------- the recorded body itself --

#[test]
fn the_recorded_config_is_not_world_readable() {
    // The body is a full copy of the operator's config. A config kept at 0600
    // because it carries secrets must not be re-published at the default umask,
    // once per generation. Pristine forjar stored only a hash.
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let state = d.join("state");
    let cfg = write_param_stack(d);
    let (rc, out) = apply_with(&cfg, &state, &[]);
    assert_eq!(rc, 0, "apply failed:\n{out}");

    let body = state
        .join("generations")
        .join("0")
        .join(".applied-config.yaml");
    assert!(body.exists(), "the generation must record the config body");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&body).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "the recorded config is a verbatim copy of the operator's config \
             and must not widen its exposure; got {mode:o}"
        );
    }
}
