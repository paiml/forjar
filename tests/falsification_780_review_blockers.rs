//! #780 review blockers (forjar#828 isolation merged onto the #660 loader).
//!
//! 1. An `outputs:` entry that references a FAILED data source must be skipped with the
//!    resources that reference it, not abort `apply` before its post-actions run.
//! 2. `codegen` must emit what `apply` runs: a data value is a literal, never re-scanned.
//! 3. `explain` prints a data value as written: no U+E001/U+E002 stand-in reaches stdout.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(forjar())
        .args(args)
        .output()
        .expect("forjar failed to start");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn write_cfg(dir: &tempfile::TempDir, body: &str) -> PathBuf {
    let cfg = dir.path().join("forjar.yaml");
    fs::write(&cfg, body).unwrap();
    cfg
}

/// Blocker 1: the post-apply hook is the witness that apply reached its post-actions.
#[test]
fn an_output_of_a_failed_data_source_does_not_abort_the_post_actions() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("post-apply-ran");
    let unrelated = dir.path().join("unrelated.txt");
    let body = format!(
        "version: \"1.0\"\nname: o780\n\
machines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\n\
data:\n\x20 broken: {{ type: command, value: \"exit 127\" }}\n\
outputs:\n\x20 from_broken: {{ value: \"{{{{data.broken}}}}\" }}\n\x20 plain: {{ value: \"fixed\" }}\n\
policy:\n\x20 post_apply: \"touch {}\"\n\
resources:\n\x20 unrelated: {{ type: file, machine: local, path: {}, content: \"fine\\n\" }}\n",
        marker.display(),
        unrelated.display()
    );
    let cfg = write_cfg(&dir, &body);
    let state = dir.path().join("state");
    let (ok, so, se) = run(&[
        "apply",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
        "--yes",
    ]);
    let all = format!("{so}{se}");
    assert!(!ok, "a failed data source must still fail the run:\n{all}");
    assert!(unrelated.exists(), "unrelated resource applied:\n{all}");
    assert!(
        marker.exists(),
        "post_apply hook must run despite the failed source's output:\n{all}"
    );
    assert!(
        all.contains("from_broken"),
        "the skipped output must be named:\n{all}"
    );
}

// base64 of "v={{params.port}}\n" (the literal) and of "v=8080\n" (a re-scanned value).
const LITERAL_B64: &str = "dj17e3BhcmFtcy5wb3J0fX0K";
const EXPANDED_B64: &str = "dj04MDgwCg==";

const LITERAL_CFG: &str = "version: \"1.0\"\nname: l780\n\
machines: { local: { hostname: localhost, addr: 127.0.0.1 } }\n\
params: { port: \"8080\" }\n\
data:\n  lit: { type: command, value: \"printf '%s' '{{params.port}}'\" }\n\
resources:\n  r: { type: file, machine: local, path: /tmp/forjar-780-never, content: \"v={{data.lit}}\\n\" }\n";

/// Blocker 2: the data value is the literal text `{{params.port}}`; codegen must not expand it.
#[test]
fn codegen_emits_a_data_value_as_a_literal() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_cfg(&dir, LITERAL_CFG);
    let (ok, so, se) = run(&["codegen", "-f", cfg.to_str().unwrap(), "-r", "r"]);
    assert!(ok, "codegen failed:\n{so}{se}");
    assert!(
        so.contains(LITERAL_B64),
        "the data value must stay literal:\n{so}"
    );
    assert!(
        !so.contains(EXPANDED_B64),
        "data value was re-scanned:\n{so}"
    );
}

/// Blocker 3: no stand-in character may be printed by `explain`, text or JSON.
#[test]
fn explain_prints_no_stand_in_characters() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_cfg(&dir, LITERAL_CFG);
    for json in [false, true] {
        let mut args = vec!["explain", "-f", cfg.to_str().unwrap(), "r"];
        if json {
            args.push("--json");
        }
        let (ok, so, se) = run(&args);
        assert!(ok, "explain (json={json}) failed:\n{so}{se}");
        assert!(
            !so.contains('\u{e001}') && !so.contains('\u{e002}'),
            "stand-in leaked (json={json}):\n{so}"
        );
        // The text form prints the content; the JSON form carries it inside the base64 script.
        let want = if json { LITERAL_B64 } else { "{{params.port}}" };
        assert!(so.contains(want), "value as written (json={json}):\n{so}");
    }
}
