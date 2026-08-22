//! GH-247 end-to-end: `forjar verify` through the binary.
//!
//! Driven through the CLI rather than `core::verify` directly, deliberately.
//! forjar has shipped a subcommand that was unreachable from `main` while its
//! unit tests passed; a function that works and a verb that works are different
//! claims, and this file makes the second one.

use std::path::Path;
use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// A project whose recipe writes `command` into `out.txt`.
///
/// Non-determinism goes in `gen.sh`, not in the recipe — forjar's bashrs I8
/// gate statically rejects `date +%s%N` written directly in a `command:` with
/// DET002, so an inline version never reaches apply. That is the same reason
/// the issue's own reproduction uses an external generator: a static gate
/// cannot see inside `gen.sh`, and cannot see inside ffmpeg or an LLM call
/// either. Verifying reproducibility is precisely the gap that leaves.
fn project(dir: &Path, command: &str) -> std::path::PathBuf {
    let proj = dir.join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    let gen = proj.join("gen.sh");
    std::fs::write(&gen, format!("#!/bin/bash\n{command}\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&gen, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: verify-e2e
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  frame:
    type: task
    machine: local
    working_dir: "{}"
    cache: true
    task_inputs: []
    output_artifacts: ["out.txt"]
    command: |
      ./gen.sh
"#,
            proj.display()
        ),
    )
    .unwrap();
    cfg
}

fn apply(cfg: &Path, state: &Path) {
    let out = forjar()
        .args([
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            state.to_str().unwrap(),
            "--no-tripwire",
            "--yes",
        ])
        .output()
        .expect("apply runs");
    assert!(
        out.status.success(),
        "apply must succeed to record a hash. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn verify(cfg: &Path, state: &Path, extra: &[&str]) -> std::process::Output {
    let mut args = vec![
        "verify",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    forjar().args(&args).output().expect("verify runs")
}

#[test]
fn a_nondeterministic_artifact_is_reported_and_exits_nonzero() {
    // Case A from the issue: an opaque generator. `date` stands in for ffmpeg,
    // whisper, or an LLM call — none of which a static gate can see inside.
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let cfg = project(dir.path(), "date +%s%N > out.txt");

    apply(&cfg, &state);
    let out = verify(&cfg, &state, &[]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        !out.status.success(),
        "a non-reproducing artifact must fail the command, so CI can gate on \
         it.\n{combined}"
    );
    assert!(
        combined.contains("DIVERGED"),
        "the verdict must name the resource that did not reproduce.\n{combined}"
    );
}

#[test]
fn verify_never_writes_the_declared_output() {
    // THE PROMISE, end to end. The issue states it as the hard requirement:
    // "it must never write to the declared output path, on match or mismatch."
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let cfg = project(dir.path(), "date +%s%N > out.txt");

    apply(&cfg, &state);
    let artifact = dir.path().join("proj").join("out.txt");

    // Stand in for a human-corrected artifact: overwrite what apply produced.
    std::fs::write(&artifact, "HUMAN CORRECTED\n").unwrap();
    let before = std::fs::read_to_string(&artifact).unwrap();

    let out = verify(&cfg, &state, &[]);
    assert!(
        !out.status.success(),
        "precondition: this must diverge, or the test proves nothing"
    );

    assert_eq!(
        before,
        std::fs::read_to_string(&artifact).unwrap(),
        "verify overwrote the declared output — the one thing it must never do"
    );
}

#[test]
fn a_deterministic_artifact_reproduces_and_exits_zero() {
    // The gate must be passable, or it trains people to ignore it.
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let cfg = project(dir.path(), "printf 'STABLE\\n' > out.txt");

    apply(&cfg, &state);
    let out = verify(&cfg, &state, &[]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "a reproducing artifact must pass.\n{combined}"
    );
    // NOT `contains("reproduced")` — that substring also matches "not
    // reproduced", so the first version of this assertion passed while every
    // resource was being skipped for want of a recorded hash.
    assert!(
        combined.contains("1 reproduced, 0 not reproduced"),
        "{combined}"
    );
}

#[test]
fn json_output_is_machine_readable() {
    // The issue asks for machine-readable output "so CI can gate on it". A
    // human-prose-only verdict would leave every consumer scraping text.
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let cfg = project(dir.path(), "date +%s%N > out.txt");

    apply(&cfg, &state);
    let out = verify(&cfg, &state, &["--json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout
        .lines()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or_else(|| panic!("no JSON object in output:\n{stdout}"));

    let parsed: serde_json::Value =
        serde_json::from_str(line).unwrap_or_else(|e| panic!("not valid JSON: {e}\n{line}"));
    assert_eq!(
        parsed["not_reproduced"], 1,
        "JSON must report the divergence: {line}"
    );
    assert_eq!(parsed["results"][0]["verdict"], "diverged", "{line}");
}
