# Judges — lint toolchain override (PMAT-567)

Round count and heads: see `PMAT-567-lanes.md`.

## CONFIRMED

1. [diff] C3 — No `|| true` appears as live shell in the step; the only occurrence of that string in the file is inside a comment explaining why it is not used, and `rustup override unset` was measured to exit 0 when there is no override to remove, so the unconditional call is safe under `set -euo pipefail`.
   - evidence: all three lanes confirmed; the exit status was measured in a scratch directory rather than inferred from documentation.
2. [test] C4 — The test carries an anti-vacuity arm asserting `rust-toolchain.toml` still lists clippy among its components, because if that list ever loses it then honouring the pin stops guaranteeing clippy and the other assertions would pass over a job failing for the original reason.
   - evidence: `tests/falsification_lint_refuses_a_toolchain_override.rs:141` reads the toolchain file directly rather than trusting the workflow's account of it.
3. [diff] C6 — The branch adds no production Rust: the diff is one workflow file and one test file, so nothing about the crate's shipped behaviour changes and the release this lands in carries no new behaviour bullet.
   - evidence: `git diff --stat 07f6d122 HEAD` lists exactly `.github/workflows/lint.yml` and `tests/falsification_lint_refuses_a_toolchain_override.rs:1`.

## REFUTED

1. [shell] C2 as first implemented — The step compared the active toolchain's version against the pinned channel, and `cut -d- -f1` cannot tell a version's hyphens from a target triple's, so the guard was correct only for the single channel spelling in use the day it was written.
   - corrected: replaced with `cargo clippy --version`, and pinned at `tests/falsification_lint_refuses_a_toolchain_override.rs:115` so the measurement cannot quietly become a parse again. Ask for the property, do not parse for it; that removes the class rather than the three instances found.
2. [shell] C2, second counterexample — A `channel = "stable"` pin never matches a resolved version at all, so the comparison would have false-FAILED on the most ordinary pin a Rust project can have, on every runner, immediately.
   - corrected: no version string is parsed or compared anywhere in the step any more.
3. [shell] C2, third counterexample — A date-pinned nightly (`nightly-2026-01-01-x86_64-unknown-linux-gnu`) reduces under the same pipeline to `nightly`, and a beta (`1.93.0-beta.1`) reduces to `1.93.0`, giving a false FAIL and a false PASS respectively from one line of parsing.
   - corrected: same repair; the three counterexamples came from three lanes independently, which is what argued for replacing the approach rather than hardening the parser.
4. [shell] C1 as worded — `rustup override unset` removes the override for the CURRENT directory only, so one set on a PARENT directory of the workspace survives it and the job stays silently retargeted; the claim that a stale override can no longer retarget the job was therefore too strong.
   - corrected: the error message now names a parent-directory override and `RUSTUP_TOOLCHAIN` in the service environment as causes this workflow cannot reach, instead of implying they cannot arise.
5. [shell] C2, robustness — Under `set -euo pipefail` a rustup that cannot resolve any toolchain aborts the script before the named refusal is printed, so the guarantee that an environment failure is always distinguishable by name is conditional on rustup itself working.
   - corrected: recorded rather than papered over; the failing command's own output is the diagnosis in that case, and the arm pinned at `tests/falsification_lint_refuses_a_toolchain_override.rs:115` still requires the step to ask for a runnable clippy rather than to infer one.
6. [test] C5 as first written — The falsification test was VACUOUS under its own declared mutation: `step.contains("rustup override unset")` is satisfied by the step's comments and by its error message, so replacing the real command with `rustup override list` left the test GREEN.
   - corrected: anchored at `tests/falsification_lint_refuses_a_toolchain_override.rs:100` to a line that IS the command, and the mutation comment now records that the obvious `sed` phrasing also deletes the error message and reddens two tests, so it is not an address.
7. [test] C5, scope — `toolchain_step()` sliced from the toolchain step through the unrelated `Cache cargo` step up to `Clippy`, three steps wide, so a substring appearing in an unrelated step would have satisfied assertions about this one.
   - corrected: the slice at `tests/falsification_lint_refuses_a_toolchain_override.rs:81` now ends at the next `- name:` of any kind, which is what the function's name claims.
