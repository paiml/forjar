# Quorum evidence — PMAT-204 — refuter rulings

The refuter round (q2) put six claims to three lanes in standalone clones at the redaction rule's first commit. All three returned FAIL, unanimously per claim: K1, K2, K3, K5 and K6 killed, K4 (no pre-existing refusal lost) survived 3/3.

## refuter lane 1 (verdict FAIL)

Killed 5 out of 6 claims. K1, K2, K3, K5, and K6 were killed with concrete counterexamples or citations. K4 survived as no other diagnostic is dropped.

Findings:
- R1-F1 [measured] src/core/purifier_sec017.rs:101 — K1: The SEC017 exemption is bashrs's own judgement, not forjar's... Find any line where this exempts a script that a) forjar's I8 gate refused before PMAT-204 and b) really does set a world-writable mode or really is unsafe. (proposed fix: Update `symbolic_grants_world_write` to split by commas and evaluate all components, rather than just checking the first operation.)
- R1-F2 [measured] src/core/purifier_sec017.rs:90 — K2: The redaction cannot swallow a real mode... Try to build a mode literal that redaction destroys, or a quoting shape... (proposed fix: Variables inside quotes are not parsed as octal modes, so they are destroyed. Redaction must handle variables in mode arguments properly or restrict the exemption scope.)
- R1-F3 [measured] src/core/purifier_sec017.rs:146 — K3: forjar's own check world_writable_modes catches what bashrs misses... Find a world-writable mode forjar generates or a user could declare that it misses. (proposed fix: Update `symbolic_grants_world_write` to handle multiple comma-separated operations.)
- R1-F4 [asserted] src/core/purifier.rs:41 — K4: No pre-existing refusal was lost... Look for a severity filter, an allow-list, an early return or a code path that drops any other diagnostic. (proposed fix: None needed, the claim survives.)
- R1-F5 [cited] tests/falsification_chmod_path_is_not_a_mode.rs:70 — K5: The tests ... are non-vacuous: name any test that would pass against the code as it stood at 0776fd82 (before any fix), or that asserts only on text it constructs without going through validate_script. (proposed fix: `a_bare_chmod_666_still_fails_the_i8_gate` asserts that `validate_script` refuses the script with SEC017. Since the I8 gate already rejected this before PMAT-204, it would pass on 0776fd82. In `src/core/purifier_sec017.rs`, `redaction_keeps_the_mode_and_replaces_the_path` asserts directly on strings without going through `validate_script`.)
- R1-F6 [asserted] CHANGELOG.md:10 — K6: The CHANGELOG paragraph under `[Unreleased]` claims exactly what the code does and no more. (proposed fix: Update CHANGELOG to reflect that the gate can mistakenly exempt some true positives (such as `chmod 'u=rwx,a+w'` or `chmod "666$foo"`) due to flaws in redaction and `world_writable_modes`.)

## refuter lane 2 (verdict FAIL)

K1: KILLED. The script `echo \" ; (chmod 777 /foo) ; echo \"` executes a world-writable mode in a subshell, but because `world_writable_modes` fails to unquote `(chmod`, it misses it. Meanwhile, `redact_quoted_paths` ignores the backslashes and treats `\"` as an opening quote, redacting `chmod 777` entirely into `'/x'`. SEC017 stops reporting on the redacted line, exempting it from the I8 gate, making the script pass.

K2: KILLED. The mode literal `777` in `echo \" ; chmod 777 /foo ; echo \"` is destroyed by redaction. `redact_quoted_paths` does not handle escaped quotes `\"`, so it treats the entire sequence from the first `\"` to the second `\"` as a string, redacting `chmod 777 /foo` into `'/x'`. The redacted line lints clean, while the original executes the unsafe mode.

K3: KILLED. `world_writable_modes` misses multi-part symbolic modes like `chmod u=rwx,o=w /foo`. The `symbolic_grants_world_write` function only checks the first matched part before the comma (`u=rwx`) and ignores the world-writable `o=w` part completely.

K4: SURVIVED. I checked `validate_script` and `is_path_false_positive` for any path that could drop other diagnostics. The filter strictly only ignores `SEC017` findings when `sec017_is_path_only` returns true. No other error-severity diagnostics (e.g., syntax errors) are dropped.

K5: KILLED. The test `a_second_chmod_on_the_same_line_is_not_hidden_by_the_first` asserts that several scripts with an unquoted `chmod 777` are refused (`is_err()`). Before PMAT-204 (at `0776fd82`), SEC017 alone would have successfully caught and refused every script in that array, meaning the test passes against the old code. Also, tests like `world_writable_modes_are_named_wherever_they_sit` assert directly on functions without going through `validate_script`.

K6: KILLED. The CHANGELOG under `[Unreleased]` claims that 'every world-writable mode on any line... symbolic (a+w, o+w)... is refused'. This is false, as demonstrated in K3: valid world-writable symbolic modes like `u=rwx,o=w` are missed by the code and pass the gate.

Findings:
- R2-F1 [measured] src/core/purifier_sec017.rs:113 — K1: The SEC017 exemption is bashrs's own judgement, not forjar's (proposed fix: echo \" ; (chmod 777 /foo) ; echo \")
- R2-F2 [measured] src/core/purifier_sec017.rs:78 — K2: The redaction cannot swallow a real mode (proposed fix: echo \" ; chmod 777 /foo ; echo \")
- R2-F3 [measured] src/core/purifier_sec017.rs:152 — K3: forjar's own check world_writable_modes catches what bashrs misses (proposed fix: chmod u=rwx,o=w /foo)
- R2-F4 [asserted] src/core/purifier.rs:134 — K4: No pre-existing refusal was lost (proposed fix: SURVIVED. Reviewed the filters and early returns; only SEC017 is explicitly allowed to be dropped if path-only.)
- R2-F5 [asserted] tests/falsification_chmod_path_is_not_a_mode.rs:95 — K5: The tests ... are non-vacuous (proposed fix: a_second_chmod_on_the_same_line_is_not_hidden_by_the_first would pass against the old code since all its cases triggered SEC017 before PMAT-204.)
- R2-F6 [asserted] CHANGELOG.md:19 — K6: The CHANGELOG paragraph claims exactly what the code does and no more (proposed fix: Changelog claims every world-writable mode is refused, but K3 proves multi-part modes like u=rwx,o=w bypass it.)

## refuter lane 3 (verdict FAIL)

Evaluated claims K1-K6 regarding commit ec276fcf. Killed K1 and K2 using an escaped quote shape that tricks `redact_quoted_paths` into swallowing real modes. Killed K3 by demonstrating that `world_writable_modes` misses comma-separated symbolic modes (e.g. `u=r,o=w`). K4 survived as there are no other diagnostic-dropping filters. Killed K5 by identifying vacuous regression tests and module tests that bypass `validate_script`. Killed K6 because the CHANGELOG over-claims protection against all world-writable modes while missing comma-separated ones. Verdict is FAIL.

Findings:
- R3-F1 [cited] src/core/purifier_sec017.rs:105 — K1: The SEC017 exemption is bashrs's own judgement... Find any line where this exempts a script that I8 gate refused before PMAT-204 and really sets a world-writable mode. (proposed fix: KILLED. Counterexample: `echo "\"" ; chmod 666 file ; echo "\""`. This script is unsafe and was refused before (SEC017 flags chmod 666), but `redact_quoted_paths` pairs the quotes incorrectly, swallowing the mode and exempting it.)
- R3-F2 [cited] src/core/purifier_sec017.rs:105 — K2: The redaction cannot swallow a real mode... Try to build a quoting shape that makes redact_quoted_paths produce a line that lints clean while original was unsafe. (proposed fix: KILLED. Counterexample: `echo "\"" ; chmod 666 file ; echo "\""`. The escaped quote `\"` is parsed as a normal quote by `redact_quoted_paths`, swallowing the real `chmod 666` into `'/x'`.)
- R3-F3 [cited] src/core/purifier_sec017.rs:147 — K3: forjar's own check world_writable_modes catches what bashrs misses... Find a world-writable mode it misses. (proposed fix: KILLED. Counterexample: `chmod u=r,o=w file`. `symbolic_grants_world_write` only checks the first `who` group (`u`), completely missing the subsequent `o=w` that grants world write.)
- R3-F4 [cited] src/core/purifier.rs:84 — K4: No pre-existing refusal was lost... Look for a severity filter, allow-list, early return that drops any other diagnostic. (proposed fix: SURVIVED. `validate_script` only drops errors where `is_path_false_positive` is true, which strictly enforces `diag.code == "SEC017"`. No other diagnostics are dropped.)
- R3-F5 [cited] tests/falsification_chmod_path_is_not_a_mode.rs:50 — K5: The tests are non-vacuous: name any test that would pass against the code at 0776fd82, or that asserts only on constructed text without going through validate_script. (proposed fix: KILLED. `a_bare_chmod_666_still_fails_the_i8_gate` passes at 0776fd82 (pre-existing SEC017 behavior). Also `forjar_generated_lines_are_exempt` in `chmod_mode_tests` bypasses `validate_script`.)
- R3-F6 [cited] CHANGELOG.md:23 — K6: The CHANGELOG paragraph claims exactly what the code does and no more. (proposed fix: KILLED. The CHANGELOG claims "every world-writable mode ... is refused", but as proven in K3, the code misses `u=r,o=w`. Thus it claims more than what the code actually does.)

## What the orchestrator measured of them

Two kills reproduced and were fixed: the backslash-escaped quote (refused at baseline, accepted by that commit — a regression) and the comma-clause symbolic mode (accepted at baseline too — a hole the new check claimed to close and did not). The rest did not reproduce.

