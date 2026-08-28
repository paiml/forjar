//! Generated shell must survive forjar's OWN I8 gate, and the unobservable
//! sentinel must name the command it was asked to name.
//!
//! THE FLAW THIS CLOSES.
//!
//! (1) `sh_squote` rendered an embedded single quote with the classic POSIX
//! close/escape/reopen idiom `'\''`. That is correct shell. But forjar lints
//! every script it generates with bashrs before executing it
//! (`validate_before_exec` -> `purifier::validate_script`, Error severity), and
//! bashrs' SC2075 is a line-scoped regex with no quote-state tracking:
//! `'[^']*\'[^']*'`. It matches the CORRECT idiom, because it cannot tell it
//! from the genuine error `echo 'can\'t'`. So any generated script carrying a
//! config value with an apostrophe was rejected by forjar before it ever
//! reached a host.
//!
//! Measured (#350), on a task with no `completion_check`:
//!
//!     DRIFTED: ci-budget-activation (transport error: I8 violation —
//!       script failed bashrs validation: bashrs lint errors:
//!       [error] SC2075: Escaping a single quote in single quotes won't work.
//!
//! The `unobservable:` sentinel from #279 is merely the most common way to hit
//! it — a task command ending in `echo '…'` is an everyday shape — but the same
//! landmine sat under output_artifact paths, package names, mount labels and
//! cron commands. The FJ-154 injection-hardening tests construct exactly such
//! values, so the hardening and the I8 gate were in direct contradiction.
//!
//! (2) `sh_squote` STRIPS control characters (right for a shell word), so
//! interpolating a multi-line command into the sentinel welded each line to the
//! next: `set -eu` + `sudo systemctl daemon-reload` became
//! `set -eusudo systemctl daemon-reload`. The sentinel named a command that was
//! never written and never run — the one thing it exists to communicate.
//!
//! WHAT THIS TEST MUST NOT BECOME. Asserting a particular escape SPELLING would
//! pin an implementation, not a property. The load-bearing assertions here run
//! forjar's real gate over the real generated script, and execute the sentinel
//! under both `sh` and `bash` to prove the observable's bytes do not depend on
//! which shell the target happens to have.

use forjar::core::codegen::state_query_script;
use forjar::core::purifier::validate_script;
use forjar::core::shell_escape::sh_squote;
use forjar::core::types::{MachineTarget, Resource, ResourceType};
use std::process::Command;

/// The exact shape from #350: multi-line, and ending in a quoted `echo`.
const COMMAND: &str = "set -eu\nsudo systemctl daemon-reload\nsudo systemctl restart \
                       systemd-journald\necho 'ci-budget: systemd reloaded, journald restarted'\n";

fn unobservable_task() -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        machine: MachineTarget::Single("m1".to_string()),
        command: Some(COMMAND.to_string()),
        ..Default::default()
    }
}

/// FALSIFY-350-A: forjar must not generate shell its own gate rejects.
#[test]
fn the_sentinel_passes_forjars_own_gate() {
    let q = state_query_script(&unobservable_task()).expect("state_query_script");
    assert!(
        q.contains("unobservable:no-completion-check:"),
        "fixture no longer reaches the sentinel branch: {q}"
    );
    validate_script(&q).unwrap_or_else(|e| {
        panic!("forjar generated shell its own I8 gate rejects:\n{e}\n--- script ---\n{q}")
    });
}

/// FALSIFY-350-B: the sentinel exists to tell an operator WHICH command needs a
/// check. A rendering that welds line 1 onto line 2 names a command nobody
/// wrote.
#[test]
fn the_sentinel_does_not_weld_lines_together() {
    let q = state_query_script(&unobservable_task()).expect("state_query_script");
    assert!(
        !q.contains("set -eusudo"),
        "the sentinel welded two lines into a command that was never run: {q}"
    );
    assert!(
        q.contains("set -eu\\nsudo systemctl daemon-reload"),
        "the sentinel did not render the newline it dropped: {q}"
    );
}

/// FALSIFY-350-C: the sentinel's stdout is what drift hashes, so it must not
/// depend on whether the target's `/bin/sh` is dash or bash. dash's XSI `echo`
/// expands `\n`; `printf '%s\n'` does not touch its argument.
#[test]
fn the_sentinel_reads_the_same_under_sh_and_bash() {
    let q = state_query_script(&unobservable_task()).expect("state_query_script");
    let run = |shell: &str| {
        let out = Command::new(shell)
            .arg("-c")
            .arg(&q)
            .output()
            .unwrap_or_else(|e| panic!("spawn {shell}: {e}"));
        assert!(out.status.success(), "{shell} rejected the sentinel: {q}");
        out.stdout
    };
    let with_sh = run("sh");
    let with_bash = run("bash");
    assert_eq!(
        String::from_utf8_lossy(&with_sh),
        String::from_utf8_lossy(&with_bash),
        "the observable's bytes depend on the target's shell"
    );
    assert!(
        String::from_utf8_lossy(&with_sh).contains("ci-budget: systemd reloaded"),
        "the apostrophe-bearing text was lost: {}",
        String::from_utf8_lossy(&with_sh)
    );
}

/// FALSIFY-350-D: the family-level assertion. The sentinel was one symptom; the
/// defect belongs to the canonical escaper, which every resource handler uses.
#[test]
fn the_canonical_escaper_never_emits_shell_the_gate_rejects() {
    for value in [
        "it's",
        "x';reboot;'",
        "/etc/o'brien.conf",
        "don't't",
        "'",
        "a'b'c'd",
    ] {
        let script = format!("echo {}\n", sh_squote(value));
        validate_script(&script).unwrap_or_else(|e| {
            panic!("sh_squote({value:?}) produced shell the I8 gate rejects:\n{e}\n{script}")
        });
    }
}

/// FALSIFY-350-E: whatever spelling the escaper uses, it must still be ONE
/// shell word that the payload cannot break out of. Proved by running it, not
/// by matching it — an escaper that satisfied D by deleting the quote would
/// fail here.
#[test]
fn the_escaped_value_is_still_exactly_one_inert_word() {
    for value in ["it's", "x';reboot;'", "$(id)", "a'b", "`id`"] {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("printf '%s' {}", sh_squote(value)))
            .output()
            .expect("run printf");
        assert!(out.status.success(), "{value:?} did not parse as one word");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            value,
            "the escaped word did not round-trip to its literal value"
        );
    }
}
