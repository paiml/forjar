# Quorum evidence — PMAT-159 — adjudicated claims (majority of three judges)

Seventeen claims from three blind claim lanes plus an independent-stack review
were put to three refuters and then to three judges. The rule is a majority of
the three; a judge ruling of CONFIRMED-AS-NARROWED counts as survived, because
every one of them kept the substance and moved only the citation anchor.

Two claims drew two REFUTED votes and are recorded below with the correction.

## CONFIRMED — 15 claims survived refutation

1. [transport] A1 — the elevated script is written to a private `mktemp` file and the cleanup trap is armed before the file is filled.
   - evidence: all three judges kept the substance and moved only the anchor (J1 and J2 to 297, J3 to the enclosing signature at 279). Verified against the tree rather than taken on their word: at base `src/core/codegen/dispatch.rs:297` was the entire wrapper, `sudo bash /dev/fd/3 3<<'{delim}'`; at HEAD the same `format!` emits `mktemp "${TMPDIR:-/tmp}/forjar-sudo.XXXXXX"` guarded by `|| exit 1`, then `trap 'rm -f "$forjar_sudo_script"' EXIT INT TERM`, and only then `cat >"$forjar_sudo_script"`.

2. [transport] A2 — the script text crosses into the temp file through a single-quoted heredoc and reaches the elevated bash as a path argument.
   - evidence: the emitter writes `cat >"$forjar_sudo_script" <<'{delim}'` with the delimiter still chosen by `heredoc_delimiter("FORJAR_SUDO", &script)`, so outer-shell expansion stays disabled inside the payload, and the elevated command is `sudo bash "$forjar_sudo_script"` — a path, not an inherited descriptor. The fd-3 form it replaces is still readable at base in `src/core/codegen/dispatch.rs:297`. Refuter R3 sustained the claim; J1, J2 and J3 all kept it.

3. [transport] A3 — the wrapper propagates the script's exit status and leaves the elevated shell's stdin alone.
   - evidence: `sudo bash "$forjar_sudo_script"` is the last command of the else branch, and a bash EXIT trap that does not itself call `exit` leaves the status of that last command in place, so the wrapper exits with the elevated script's code. stdin is untouched because the payload no longer arrives on any descriptor — the property fd 3 was chosen for at `src/core/codegen/dispatch.rs:297` is preserved, not traded away, by the file transport.

4. [transport] A4 — `in_declared_privilege_context` is the only emitter of the sudo wrapper, which is what makes the bare trap and the fixed variable name safe.
   - evidence: the function is private and its base signature sits at `src/core/codegen/dispatch.rs:279`; its three callers are `apply_script`, `check_script` and `state_query_script`, each wrapping one whole generated script exactly once. All three judges confirmed. The HEAD comment records the bound instead of overstating it: a future caller that joins two wrapped scripts must move the else branch into a subshell, because bash REPLACES an EXIT trap rather than stacking it.

5. [transport] A5 — the `timeout:` wrapper is the only remaining user of `/dev/fd/`, and it is unaffected because it runs inside the already elevated shell.
   - evidence: `timeout_wrapped` still emits `timeout {timeout_secs} bash /dev/fd/3 3<<'{d}'` in `src/resources/task/helpers.rs` at base line 103, but that descriptor is opened by the elevated bash AFTER sudo has already done its closefrom, so nothing closes it. J1, J2 and J3 confirmed; the emulated suite pins the behaviour with `the_timeout_wrapper_still_runs_under_closefrom` rather than leaving it as prose.

6. [tests] B1 — the emulated test drives the production emitter under a fake `sudo` that closes every inherited descriptor at or above 3.
   - evidence: J1 and J3 confirmed as narrowed; J2 alone refuted, and on the procedural ground that a brand-new file carries no citation resolving at base, not on the substance. The fake toolchain is written by `fake_bin`, the script is fed to bash on stdin by `run_as_transport` exactly as the transports feed it, and the wrapper under test is whatever `apply_script` emits — the same code path whose base form is `src/core/codegen/dispatch.rs:297`.

7. [tests] B2 — the RED evidence is credible because the fixture first proves the fake sudo really closes the descriptor.
   - evidence: `the_fake_sudo_really_emulates_closefrom` runs the OLD `sudo bash /dev/fd/3 3<<'D'` form, still readable at `src/core/codegen/dispatch.rs:297`, through the same fake and requires `/dev/fd/3: No such file or directory` with exit 127. Without that self-probe a fake that quietly stopped closing descriptors would make every other assertion in the file pass for the wrong reason. J1 and J3 confirmed as narrowed; J2's refusal was again the missing base anchor, not the claim.

8. [tests] B5 — the unit tests in `tests_sudo.rs` are stronger than before but remain text assertions over the generated script.
   - evidence: unanimously confirmed by J1, J2 and J3, and sustained by R3. At base `src/core/codegen/tests_sudo.rs:47` read `assert!(script.contains("sudo bash /dev/fd/3 3<<'FORJAR_SUDO'"))`, and at HEAD the same `.contains()` style is kept and extended rather than replaced. J2 and J3 both recorded that refuter R1 hallucinated a `run_as_transport` helper in this file; it exists only in the two new integration tests.

9. [root cause] C1 — the previous wrapper passed the script on fd 3 and sudo's closefrom closed it before exec.
   - evidence: confirmed by all three judges and sustained by R3. The exact string is at `src/core/codegen/dispatch.rs:297` at base: `sudo bash /dev/fd/3 3<<'{delim}'`. sudo closes every descriptor at or above 3 before it execs, so the elevated bash opened `/dev/fd/3`, found nothing and exited 127 — for apply, check and state_query alike, since all three share this one wrapper.

10. [teamwork] T1 — `mktemp` creates the payload file atomically at 0600 owned by the caller, which root can still read.
   - evidence: J1 and J2 confirmed as narrowed, J3 confirmed outright. R3 filed the only attack — an attacker-controlled `TMPDIR`, no explicit umask and no sticky-bit check — and all three judges ruled it immaterial for a tool running as the invoking user's own shell, since `mktemp` opens with `O_EXCL` and the user pointing their own `TMPDIR` somewhere hostile is attacking themselves. Emitted at the site that replaced `src/core/codegen/dispatch.rs:297`.

11. [teamwork] T2 — the `EXIT INT TERM` trap removes the temp file on every catchable path without clobbering the status.
   - evidence: J1 and J3 confirmed, J2 confirmed as narrowed. R3 weakened rather than killed it: SIGKILL is uncatchable by POSIX design, so no shell trap can cover it, and what survives is a 0600 file owned by the invoking user. The same signal set is what the repo's two other mktemp emitters use, so a ^C during a slow elevated apply leaves nothing behind rather than a readable script.

12. [teamwork] T3 — `mktemp` and `cat` are each guarded with `|| exit 1`, so a partial payload never reaches `sudo bash`.
   - evidence: sustained by R3 and confirmed by all three judges. The emitted sequence is `forjar_sudo_script="$(mktemp ...)" || exit 1` followed by `cat >"$forjar_sudo_script" <<'{delim}' || exit 1`, so an unwritable `TMPDIR` or a full disk halts the wrapper before it elevates anything at all. The base form at `src/core/codegen/dispatch.rs:297` was a single command and had no intermediate step to guard.

13. [teamwork] T4 — the single-quoted heredoc delimiter disables parameter expansion inside the payload.
   - evidence: `<<'{delim}'` is quoted exactly as the base fd-3 form at `src/core/codegen/dispatch.rs:297` quoted its own heredoc, so the transport change does not weaken payload isolation, and the delimiter is still chosen to avoid colliding with the body it bounds. R3 sustained the claim; J1 and J3 confirmed it, J2 confirmed it as narrowed. The collision half is pinned by the strictness test's `<<'FORJAR_SUDO'` assertion.

14. [teamwork] T5 — the emulated test is a real falsifier rather than another string check.
   - evidence: J1 and J3 confirmed as narrowed, J2 refuted only for the missing base anchor. Verified here by running it: with the base emitter restored over `src/core/codegen/dispatch.rs:297` the suite reports `bash: /dev/fd/3: No such file or directory`, exit 127, and `test result: FAILED. 4 passed; 6 failed`; with the fix in place all ten pass. The four that stay green are the fixture self-probes and the no-sudo control, which is the expected shape.

15. [teamwork] T6 — the commit message bounds the safety of the unscoped trap instead of asserting it generally.
   - evidence: J1 and J3 confirmed; J2 refuted only because a commit hash is not a repo-relative path citation. The bound is written into the code as well as into the message, at the function whose base signature is `src/core/codegen/dispatch.rs:279`: it names the three callers, states that each wraps one whole script exactly once, and says what a future joining caller must do instead of leaving the reader to infer it.

## REFUTED — 2 claims killed

1. [tests] B3 — the live-privilege test properly manages four combinations of sudo presence and `FORJAR_REQUIRE_SUDO_TESTS`, so its fail-closed branch protects CI.
   - corrected: J2 and J3 killed it outright and J1 narrowed it to the same finding. `FORJAR_REQUIRE_SUDO_TESTS` occurs in exactly two places in the repository, both inside the live test file itself — no workflow, Makefile or config sets it — so the panic branch never engages where the gate runs, and the test skips wherever passwordless sudo is absent. It is a live confirmation, not a falsifier; the emulated test is the sole runtime falsifier and is the target this receipt names.

2. [tests] B4 — the strictness test's ordered byte-offset checks still risk passing vacuously on a broken script.
   - corrected: J2 and J3 both killed the claim while reading opposite trees, so it was re-read at both ends here. At base, `tests/falsification_390e_nested_shell_strictness.rs:150` is the message of a plain `assert!(script.contains("sudo bash /dev/fd/3 3<<'"))` — J3's "no offset logic" describes the base file, not this branch. At HEAD that test resolves `mktemp`, `trap` and `cat` to byte offsets, asserts `mktemp < trap && trap < cat`, parses the delimiter out of the script and requires `sudo bash "$forjar_sudo_script"` to sit after the heredoc's closing line, which is J2's reading. The offsets exist, and they are what removes the vacuity the claim says remains.
