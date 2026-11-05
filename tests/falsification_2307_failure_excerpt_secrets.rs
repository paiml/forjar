//! PMAT-5880 / paiml/infra#2307: a runtime secret must reach a 0600 file on the
//! machine and NOTHING else forjar writes or prints.
//!
//! #406 closed the run transcripts under `state/<m>/runs/`. It did not close the
//! failure EXCERPT: when a command echoes its secret (`set -x`, an error naming
//! its argument) and fails, `failure_text::exec_failure` copies the tail of
//! stdout/stderr, unredacted, into the lock, `events.jsonl`, `last-apply.yaml`
//! and the process's own stderr. This test greps every file under the state dir
//! AND the captured process output, at both wave widths, for the canary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CANARY: &str = "2307-CANARY-Hs8Qw3Zx7Lp1Vb6Nm-NEVER-LOGGED";
const SECRET_ENV: &str = "FORJAR_SECRET_GATE_TOKEN";

fn walk(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return found;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            found.extend(walk(&p));
        } else {
            found.push(p);
        }
    }
    found
}

/// Apply a config whose task writes the secret to a 0600 file, then echoes it
/// on both streams. `fail` makes the task exit 3 after echoing.
fn run(parallel: bool, fail: bool) -> (tempfile::TempDir, i32, String) {
    let tail = if fail { "exit 3" } else { "true" };
    let body = format!(
        "echo \"out={{{{secrets.gate-token}}}}\"\n\x20     echo \"err={{{{secrets.gate-token}}}}\" >&2\n\x20     {tail}"
    );
    run_body(parallel, &body, "")
}

/// Same config, caller-supplied tail of the command and extra resource keys.
fn run_body(parallel: bool, body: &str, extra: &str) -> (tempfile::TempDir, i32, String) {
    run_check(parallel, body, extra, None)
}

/// `check`: a completion_check expression replacing the default `test -f <out>`.
fn run_check(
    parallel: bool,
    body: &str,
    extra: &str,
    check: Option<&str>,
) -> (tempfile::TempDir, i32, String) {
    let dir = tempfile::tempdir().unwrap();
    let out_file = dir.path().join("delivered-token");
    let check = check.map_or_else(|| format!("test -f {}", out_file.display()), str::to_string);
    let cfg = dir.path().join("forjar.yaml");
    fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: t2307\npolicy: {{ parallel_resources: {parallel} }}\n\
             machines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\n\
             secrets:\n  provider: env\n  ephemeral: true\n\
             resources:\n  deliver:\n    type: task\n    machine: local\n    command: |\n\
             \x20     umask 077\n\
             \x20     printf %s '{{{{secrets.gate-token}}}}' > {out}\n\
             \x20     {body}\n\x20   completion_check: {check}\n{extra}",
            out = out_file.display(),
        ),
    )
    .unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .env(SECRET_ENV, CANARY)
        .args([
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            dir.path().join("state").to_str().unwrap(),
            "--yes",
        ])
        .output()
        .expect("forjar failed to start");
    let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&o.stderr));
    (dir, o.status.code().unwrap_or(-1), s)
}

fn assert_no_leak(dir: &tempfile::TempDir, printed: &str, what: &str) {
    let mut leaked: Vec<String> = Vec::new();
    for p in walk(&dir.path().join("state")) {
        if let Ok(b) = fs::read(&p) {
            if String::from_utf8_lossy(&b).contains(CANARY) {
                leaked.push(p.strip_prefix(dir.path()).unwrap().display().to_string());
            }
        }
    }
    if printed.contains(CANARY) {
        leaked.push("<forjar stdout/stderr>".into());
    }
    assert!(leaked.is_empty(), "{what}: plaintext secret in {leaked:?}");
}

fn assert_delivered_0600(dir: &tempfile::TempDir) {
    use std::os::unix::fs::PermissionsExt;
    let f = dir.path().join("delivered-token");
    assert_eq!(
        fs::read_to_string(&f).unwrap(),
        CANARY,
        "delivery must work"
    );
    assert_eq!(
        fs::metadata(&f).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn failing_task_excerpt_never_carries_the_secret_width_1() {
    let (dir, rc, printed) = run(false, true);
    assert_ne!(rc, 0, "the task must fail for this to be the excerpt path");
    assert_delivered_0600(&dir);
    assert_no_leak(&dir, &printed, "failure, width 1");
}

#[test]
fn failing_task_excerpt_never_carries_the_secret_parallel() {
    let (dir, rc, printed) = run(true, true);
    assert_ne!(rc, 0);
    assert_delivered_0600(&dir);
    assert_no_leak(&dir, &printed, "failure, parallel");
}

#[test]
fn succeeding_task_output_never_carries_the_secret() {
    let (dir, rc, printed) = run(false, false);
    assert_eq!(rc, 0, "{printed}");
    assert_delivered_0600(&dir);
    assert_no_leak(&dir, &printed, "success");
}

// ---- PMAT-5880 review round: one test per path, each RED if its redact is removed ----

/// A 15-byte prefix of the canary: a cut that splits the secret leaks THIS, and
/// a whole-canary grep would miss it.
const PREFIX: &str = "2307-CANARY-Hs8";

fn state_text(dir: &tempfile::TempDir, name: &str) -> String {
    let hit = walk(&dir.path().join("state"))
        .into_iter()
        .find(|p| p.file_name().is_some_and(|n| n == name))
        .unwrap_or_else(|| panic!("no {name} under state/: the test would be vacuous"));
    fs::read_to_string(hit).unwrap()
}

fn assert_no_prefix(dir: &tempfile::TempDir, printed: &str, what: &str) {
    let mut leaked: Vec<String> = Vec::new();
    for p in walk(&dir.path().join("state")) {
        if let Ok(b) = fs::read(&p) {
            if String::from_utf8_lossy(&b).contains(PREFIX) {
                leaked.push(p.strip_prefix(dir.path()).unwrap().display().to_string());
            }
        }
    }
    if printed.contains(PREFIX) {
        leaked.push("<forjar stdout/stderr>".into());
    }
    assert!(leaked.is_empty(), "{what}: secret prefix in {leaked:?}");
}

const SECRET_REF: &str = "{{secrets.gate-token}}";

fn fill(ch: char, n: usize) -> String {
    format!("printf '{ch}%.0s' $(seq 1 {n})")
}

#[test]
fn lock_and_events_carry_the_redacted_excerpt_not_the_secret() {
    for parallel in [false, true] {
        let (dir, rc, printed) = run(parallel, true);
        assert_ne!(rc, 0);
        for name in ["state.lock.yaml", "events.jsonl"] {
            let text = state_text(&dir, name);
            assert!(!text.contains(CANARY), "{name} (parallel={parallel}) leaks");
            assert!(
                text.contains("***"),
                "{name} must carry the redacted excerpt"
            );
        }
        assert!(
            printed.contains("***"),
            "stderr must show the redacted text"
        );
        assert!(!printed.contains(CANARY));
    }
}

#[test]
fn secret_straddling_the_head_cut_leaks_no_prefix() {
    // 770 bytes of filler: 30 bytes of the secret fall inside the 800-byte head.
    for parallel in [false, true] {
        let body = format!(
            "{}; printf %s \"{SECRET_REF}\"; {}; exit 3",
            fill('a', 770),
            fill('b', 3000)
        );
        let (dir, rc, printed) = run_body(parallel, &body, "");
        assert_ne!(rc, 0);
        assert_no_prefix(&dir, &printed, &format!("head cut, parallel={parallel}"));
    }
}

#[test]
fn secret_straddling_the_tail_cut_leaks_no_suffix() {
    // The secret ends 1175 bytes before EOF, so it starts ~1215 from the end:
    // the 1200-byte tail window begins inside it.
    let suffix = &CANARY[CANARY.len() - 15..];
    let body = format!(
        "{}; printf %s \"{SECRET_REF}\"; {}; exit 3",
        fill('b', 3000),
        fill('c', 1175)
    );
    for parallel in [false, true] {
        let (dir, rc, printed) = run_body(parallel, &body, "");
        assert_ne!(rc, 0);
        let mut all = printed.clone();
        for name in ["state.lock.yaml", "events.jsonl"] {
            all.push_str(&state_text(&dir, name));
        }
        assert!(!all.contains(suffix), "tail cut leaked a suffix");
    }
}

#[test]
fn transport_error_arm_is_redacted() {
    // `eval` is rejected before execution (I8), and the rejection quotes the
    // numbered script, which holds the resolved secret.
    let body = format!("eval \"echo {SECRET_REF}\"");
    for parallel in [false, true] {
        let (dir, rc, printed) = run_body(parallel, &body, "");
        assert_ne!(rc, 0, "{printed}");
        assert_no_leak(&dir, &printed, &format!("Err arm, parallel={parallel}"));
    }
}

#[test]
fn sensitive_resource_withholds_the_excerpt() {
    let (dir, rc, printed) = run_body(
        false,
        "echo derived-$(printf %s x); exit 3",
        "    sensitive: true\n",
    );
    assert_ne!(rc, 0);
    assert!(
        !printed.contains("derived-x") && !state_text(&dir, "events.jsonl").contains("derived-x")
    );
}

#[test]
fn redact_transcript_empty_set_is_identity_and_nonempty_always_redacts() {
    use forjar::core::resolver::redact_transcript as r;
    assert_eq!(
        r("plain", &[]),
        "plain",
        "no secrets resolved: nothing to strike"
    );
    assert_eq!(
        r("plain", &[String::new()]),
        "plain",
        "an empty entry never matches"
    );
    assert_eq!(r("a SECRET b", &["SECRET".to_string()]), "a *** b");
    assert_eq!(r("", &["SECRET".to_string()]), "");
}

#[test]
fn not_converged_headline_prints_the_check_without_the_secret() {
    // The command exits 0; the check (which names the secret) fails, so the
    // headline echoes the RESOLVED completion_check, plaintext included.
    for parallel in [false, true] {
        let check = format!("test -f /nonexistent-2307 && test '{SECRET_REF}' = x");
        let (dir, rc, printed) = run_check(parallel, "true", "", Some(&check));
        assert_ne!(rc, 0, "{printed}");
        assert!(printed.contains("NOT CONVERGED"), "{printed}");
        assert_no_leak(&dir, &printed, &format!("headline, parallel={parallel}"));
    }
}
