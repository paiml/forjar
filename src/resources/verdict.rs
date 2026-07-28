//! FJ-2720 (PMAT-199): check scripts report their verdict in the EXIT CODE.
//!
//! # The defect
//!
//! Every generator used to emit `<test> && echo 'exists:x' || echo 'missing:x'`.
//! That always exits 0 — a branch whose two arms are both `echo` cannot fail.
//! `cli::check::run_single_check` decides pass/fail from `out.success()`, and
//! nothing anywhere parsed the markers, so the verdict went into the void and
//! `forjar check` reported `pass` for every resource unconditionally.
//!
//! # Why the fix lives in the generators, not at the codegen boundary
//!
//! A tempting one-line fix is to post-process the script and fail on any
//! `missing:` marker. That is wrong: the SAME marker means opposite things
//! depending on desired state. For `state: absent`, `missing:` IS convergence
//! and `exists:` is the failure. Only the generator knows which way round its
//! resource points, so only the generator can decide the exit code.
//!
//! # Why assertions do not short-circuit
//!
//! A check with several assertions (a package list, a service's active AND
//! enabled, an overlay's ip/service/timer) must report EVERY marker, not stop
//! at the first failure — the markers are the operator's diagnostic. So the
//! script records failure in a flag and exits with it at the end, rather than
//! exiting from the failing branch.

use crate::core::shell_escape::sh_squote;

/// Shell variable accumulating divergence across a script's assertions.
const FLAG: &str = "__fj_diverged";

/// One assertion inside a check script.
///
/// Prints `converged_marker` and leaves the flag alone when `condition`
/// succeeds; prints `divergent_marker` and raises the flag when it does not.
///
/// `condition` is a shell condition (`test -f 'p'`, `command -v x >/dev/null`).
/// Markers are quoted here, so pass them unquoted.
pub fn assert_that(condition: &str, converged_marker: &str, divergent_marker: &str) -> String {
    format!(
        "if {condition}; then echo {}; else echo {}; {FLAG}=1; fi",
        sh_squote(converged_marker),
        sh_squote(divergent_marker)
    )
}

/// An assertion whose branches are already-written shell, for scripts that need
/// to compute something (a hash, a version) before deciding.
///
/// `on_converged` and `on_divergent` are shell fragments. The flag is raised
/// automatically after `on_divergent`.
pub fn assert_block(condition: &str, on_converged: &str, on_divergent: &str) -> String {
    format!("if {condition}; then\n  {on_converged}\nelse\n  {on_divergent}\n  {FLAG}=1\nfi")
}

/// Report divergence unconditionally — for a resource that cannot be checked
/// (unsupported provider, malformed config).
///
/// Reporting `pass` for something forjar does not understand is the
/// unconditional-success bug in miniature, so the honest answer is a failure.
pub fn always_diverged(marker: &str) -> String {
    format!("echo {}; {FLAG}=1", sh_squote(marker))
}

/// Assemble assertion lines into a check script whose exit code is the verdict.
///
/// Every generator's `check_script` must return through this, so there is
/// exactly one place that decides how a verdict becomes an exit status.
pub fn check_script_from(assertions: &[String]) -> String {
    if assertions.is_empty() {
        // No assertions means no evidence of convergence. Claiming success here
        // is precisely the defect this module exists to remove.
        return format!("echo 'forjar=no-assertions'\nexit 1");
    }
    format!("{FLAG}=0\n{}\nexit \"${FLAG}\"", assertions.join("\n"))
}

/// Convenience for the overwhelmingly common single-assertion case.
pub fn single(condition: &str, converged_marker: &str, divergent_marker: &str) -> String {
    check_script_from(&[assert_that(condition, converged_marker, divergent_marker)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(script: &str) -> i32 {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(script)
            .output()
            .expect("sh")
            .status
            .code()
            .unwrap_or(-1)
    }

    fn stdout(script: &str) -> String {
        String::from_utf8_lossy(
            &std::process::Command::new("sh")
                .arg("-c")
                .arg(script)
                .output()
                .expect("sh")
                .stdout,
        )
        .to_string()
    }

    #[test]
    fn converged_assertion_exits_zero_and_prints_its_marker() {
        let s = single("true", "exists:x", "missing:x");
        assert_eq!(run(&s), 0);
        assert!(stdout(&s).contains("exists:x"));
    }

    #[test]
    fn divergent_assertion_exits_nonzero_and_prints_its_marker() {
        let s = single("false", "exists:x", "missing:x");
        assert_ne!(run(&s), 0, "this is the whole point of the module");
        assert!(stdout(&s).contains("missing:x"));
    }

    #[test]
    fn every_assertion_reports_even_after_one_fails() {
        // Short-circuiting would hide the second missing package from the
        // operator. The flag exists so all markers still print.
        let s = check_script_from(&[
            assert_that("false", "installed:a", "missing:a"),
            assert_that("false", "installed:b", "missing:b"),
        ]);
        let out = stdout(&s);
        assert!(out.contains("missing:a"), "{out}");
        assert!(out.contains("missing:b"), "{out}");
        assert_ne!(run(&s), 0);
    }

    #[test]
    fn one_failure_among_many_still_fails_the_whole_check() {
        let s = check_script_from(&[
            assert_that("true", "ok:a", "bad:a"),
            assert_that("false", "ok:b", "bad:b"),
            assert_that("true", "ok:c", "bad:c"),
        ]);
        assert_ne!(run(&s), 0);
    }

    #[test]
    fn all_converged_exits_zero() {
        let s = check_script_from(&[
            assert_that("true", "ok:a", "bad:a"),
            assert_that("true", "ok:b", "bad:b"),
        ]);
        assert_eq!(run(&s), 0);
    }

    #[test]
    fn empty_assertion_set_is_a_failure_not_a_pass() {
        assert_ne!(run(&check_script_from(&[])), 0);
    }

    #[test]
    fn always_diverged_fails() {
        let s = check_script_from(&[always_diverged("unsupported:provider")]);
        assert_ne!(run(&s), 0);
        assert!(stdout(&s).contains("unsupported:provider"));
    }

    #[test]
    fn assert_block_raises_the_flag_on_the_divergent_branch() {
        let s = check_script_from(&[assert_block("false", "echo 'match:m'", "echo 'mismatch:m'")]);
        assert_ne!(run(&s), 0);
        assert!(stdout(&s).contains("mismatch:m"));

        let s = check_script_from(&[assert_block("true", "echo 'match:m'", "echo 'mismatch:m'")]);
        assert_eq!(run(&s), 0);
    }

    #[test]
    fn markers_containing_shell_metacharacters_are_quoted() {
        // A marker embeds config-derived text (a package name, a path). It must
        // not be able to close the quote and run a command.
        let s = single(
            "true",
            "installed:'; touch /tmp/forjar-pwn-verdict; '",
            "missing:x",
        );
        assert_eq!(run(&s), 0);
        assert!(
            !std::path::Path::new("/tmp/forjar-pwn-verdict").exists(),
            "marker escaped its quoting and executed a command"
        );
    }

    #[test]
    fn works_under_set_u() {
        // Several generators prepend `set -euo pipefail`; the flag must be
        // initialised before it is read, or `set -u` aborts the script.
        let s = format!("set -eu\n{}", single("true", "ok:x", "bad:x"));
        assert_eq!(run(&s), 0);
        let s = format!("set -eu\n{}", single("false", "ok:x", "bad:x"));
        assert_ne!(run(&s), 0);
    }
}
