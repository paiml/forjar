//! forjar#522: a ratchet measurement must not be able to eat the machine.
//!
//! # What happened
//!
//! `scripts/ratchets/comply-count.sh` measures one comply check by running
//! `pmat comply check --format json`. `pmat comply check` evaluates
//! `.pmat-ratchet.toml` (CB-2102) and RUNS every measurement command it
//! declares. A ratchet entry naming this script is therefore a cycle by
//! construction, and on 2026-09-11 one was declared. Measured on a 48-core box
//! that had been idle:
//!
//! | metric | value |
//! |---|---|
//! | total processes | 9,740 |
//! | `comply-count.sh` copies | 5,781 |
//! | `pmat comply check` copies | 1,462 |
//! | load average, 1 min | 379, peaking at 3,026 |
//!
//! It was killed by hand. The `.pmat-ratchet.toml` that caused it was never
//! committed, so the cycle cannot occur in this repository today — but that is
//! the absence of an input, not a safeguard, and the next person to declare a
//! ratchet metric would rediscover it.
//!
//! # What these pin
//!
//! The sentinel refuses the re-entry and exits NON-ZERO without printing a
//! count, because a `0` would read as "this check reports no findings" — the
//! largest improvement in the project's history, spelled identically to a
//! script eating the machine. And no committed ratchet config may name this
//! script, so the cycle cannot be re-declared without this test going red.

#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run(env: &[(&str, &str)], args: &[&str]) -> (i32, String, String) {
    let mut c = Command::new("bash");
    c.arg(repo().join("scripts/ratchets/comply-count.sh"))
        .args(args)
        .current_dir(repo());
    for (k, v) in env {
        c.env(k, v);
    }
    let out = c.output().expect("bash must run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Under the sentinel the script refuses, says why, and prints NO count.
#[test]
fn a_recursive_invocation_is_refused_and_prints_no_count() {
    let (code, stdout, stderr) = run(&[("COMPLY_COUNT_ACTIVE", "1")], &["CB-2110"]);
    assert_eq!(
        code, 3,
        "a re-entry must exit 3, not run:\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.trim().is_empty(),
        "a refused measurement printed {stdout:?} on stdout — a count there is read as \
         the check's finding count, and 0 would read as perfection"
    );
    assert!(
        stderr.contains("refusing to run inside itself"),
        "the refusal does not say what it is refusing:\n{stderr}"
    );
}

/// The sentinel is EXPORTED, so the re-entry the guard is about is the one it
/// catches: the child `pmat comply check` must see it.
#[test]
fn the_sentinel_reaches_the_child_process() {
    let body = std::fs::read_to_string(repo().join("scripts/ratchets/comply-count.sh"))
        .expect("the script must exist");
    assert!(
        body.contains("export COMPLY_COUNT_ACTIVE=1"),
        "the sentinel is set but not exported, so the child pmat run would not see it and \
         the cycle would be unguarded"
    );
    let guard = body
        .find("COMPLY_COUNT_ACTIVE:-")
        .expect("the script must test the sentinel");
    let set = body
        .find("export COMPLY_COUNT_ACTIVE=1")
        .expect("the script must set the sentinel");
    assert!(
        guard < set,
        "the sentinel is exported before it is tested, so the script would refuse itself \
         on the first call"
    );
}

/// No committed ratchet config may name this script.
///
/// The cycle needs two halves: a script that runs comply, and a config that
/// makes comply run the script. The sentinel breaks the second entry into the
/// loop; this refuses the declaration that opens it.
#[test]
fn no_committed_ratchet_config_names_this_script() {
    let cfg = repo().join(".pmat-ratchet.toml");
    if let Ok(body) = std::fs::read_to_string(&cfg) {
        assert!(
            !body.contains("comply-count.sh"),
            ".pmat-ratchet.toml declares a metric measured by comply-count.sh, which runs \
             `pmat comply check`, which runs every metric in .pmat-ratchet.toml. That cycle \
             took a 48-core box to 9,740 processes and load 3,026 (forjar#522)."
        );
        assert!(
            !body.contains("pmat comply check") && !body.contains("pmat comply ratchet"),
            ".pmat-ratchet.toml declares a metric whose command runs pmat comply, which \
             evaluates .pmat-ratchet.toml — a cycle by construction (forjar#522)"
        );
    }
}

/// The process cap is REALLY APPLIED, and it fails closed.
///
/// The first version of this case asserted on the script's TEXT — that it
/// contained `ps -L` and a particular `ulimit` expression. Three review lanes
/// refuted it as a test of spelling rather than behaviour: a correct refactor
/// would fail it and a broken cap with the right words would pass. So the cap
/// is driven instead, with a stubbed `ps` first on `PATH`:
///
/// | stub says | expected |
/// |---|---|
/// | one thread | the cap lands at 513, far below this account's real thread count, and the script's OWN fork fails — which is the only proof the `ulimit` took effect |
/// | nothing (exit 1) | refused, exit 4, no count |
/// | a normal count | measures |
///
/// The fail-closed half is the lanes' other finding: an unreadable count used
/// to print a warning on stderr and run unbounded anyway, which in a gate whose
/// caller captures stdout is indistinguishable from no cap at all.
#[test]
fn the_process_cap_is_applied_and_fails_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path();
    let stub = bin.join("ps");

    let with_stub = |body: &str, args: &[&str]| {
        std::fs::write(&stub, body).expect("write stub");
        let mut perms = std::fs::metadata(&stub).expect("stat").permissions();
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o755);
        }
        std::fs::set_permissions(&stub, perms).expect("chmod");
        let path = format!(
            "{}:{}",
            bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let out = Command::new("bash")
            .arg(repo().join("scripts/ratchets/comply-count.sh"))
            .args(args)
            .current_dir(repo())
            .env("PATH", path)
            .output()
            .expect("bash must run");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    };

    // A thread count of 1 puts the cap at 513, far below what this account is
    // really running. If the ulimit took effect the script cannot fork; if it
    // did not, the measurement would sail through. This is the case that
    // distinguishes an applied cap from a written one.
    let (code, stdout, stderr) = with_stub("#!/usr/bin/env bash\nprintf 'x\\n'\n", &["CB-2110"]);
    assert_ne!(
        code, 0,
        "with the cap at 513 the script measured anyway, so `ulimit -u` never took effect:\n         stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.trim().parse::<i64>().is_err(),
        "a capped run printed {stdout:?}, which a caller reads as the check's finding count"
    );

    // A `ps` that cannot answer must REFUSE, not warn and run unbounded.
    let (code, stdout, stderr) = with_stub("#!/usr/bin/env bash\nexit 1\n", &["CB-2110"]);
    assert_eq!(
        code, 4,
        "an unreadable thread count must refuse the measurement:\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.trim().is_empty(),
        "a refused measurement printed {stdout:?} on stdout"
    );
    assert!(
        stderr.contains("refusing to measure rather than run unguarded"),
        "the refusal does not say it is refusing rather than proceeding:\n{stderr}"
    );
}

/// And with no stub at all, on this machine, the guarded script still measures.
///
/// A guard that stops the thing it guards is not a guard. This is the case
/// that would have caught `ulimit -u 256`, which killed the script's own fork
/// on a host already running 2,352 threads.
///
/// The check it measures is DISCOVERED from the installed tool rather than
/// named here. A fixed id made this case depend on the roster: `CB-2110` was
/// in it at 14:00 on 2026-09-11 and gone at 18:34, after pmat was rebuilt
/// locally from a different source state under the same version string, and
/// the case then failed for a reason that had nothing to do with the guard.
/// Refusing a rotted id is the script's job and is asserted elsewhere; here
/// the subject is the guard, so the id is taken from what the tool carries.
#[test]
fn the_guarded_script_still_measures_on_this_machine() {
    let out = Command::new("pmat")
        .args(["comply", "check", "--format", "json"])
        .current_dir(repo())
        .output()
        .expect("pmat must run");
    let text = String::from_utf8_lossy(&out.stdout);
    let start = match text.find('{') {
        Some(i) => i,
        None => panic!("pmat comply check --format json printed no JSON object"),
    };
    let doc: serde_json::Value =
        serde_json::from_str(&text[start..]).expect("comply output must parse");
    // A check this script can actually count: one that PASSES (which it reads
    // as zero findings) or one whose message opens with a finding count. A
    // `Warn` carrying prose is refused by design, and picking one of those
    // would test the refusal rather than the guard.
    let id = doc["checks"]
        .as_array()
        .expect("checks must be an array")
        .iter()
        .filter(|c| {
            let status = c["status"].as_str().unwrap_or("").to_ascii_lowercase();
            let msg = c["message"].as_str().unwrap_or("");
            status == "pass"
                || msg
                    .split_whitespace()
                    .next()
                    .is_some_and(|w| w.parse::<i64>().is_ok())
        })
        .filter_map(|c| c["name"].as_str())
        .filter_map(|n| n.split(':').next())
        .find(|n| n.starts_with("CB-"))
        .expect("the roster must carry at least one countable CB check")
        .to_string();

    let (code, stdout, stderr) = run(&[], &[&id]);
    assert_eq!(
        code, 0,
        "the guard stopped the measurement of {id}, which this pmat does carry:\n\
         {stdout}\n{stderr}"
    );
    assert!(
        stdout.trim().parse::<i64>().is_ok(),
        "measuring {id} printed {stdout:?}, which is not a count"
    );
}
