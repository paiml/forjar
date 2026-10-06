# Judges — forjar#590: --refresh consults a templated completion_check

Lanes and their findings: see `PMAT-590-lanes.md`. Line numbers are at head.

## CONFIRMED

1. [cause] C1 — The planner reads a lock entry as current only when its hash equals `hash_desired_state` over the resource resolved by `planner::resolve_or_fallback`; the entry `--refresh` recorded was hashed over the raw resource, so a template made the two differ and the command ran.
   - evidence: `src/core/planner/mod.rs:142` resolves with `&config.secrets` and `src/core/planner/mod.rs:306` compares `rl.hash` to `hash_desired_state(resource)` on the resolved resource; at base `converged_entry` was handed the raw resource. All three lanes confirmed.
2. [fix] C2 — `record_converged` resolves each candidate once with the config's own secrets, checks that resolved resource and records `converged_entry` over it; a candidate that cannot be resolved is not recorded.
   - evidence: `src/core/executor/refresh_seed.rs:118` is `resolve_like_the_planner`, `src/core/executor/refresh_seed.rs:160` calls it in the pipeline, `src/core/executor/refresh_seed.rs:42` now takes the resolved resource, and `src/core/executor/refresh_seed.rs:140` hashes it.
3. [test] C3 — The test drives the real binary, decides "the command ran" by a marker file rather than a summary line, covers a fresh state dir, a failed lock entry and `-r guard`, and requires an unlatched entry to hold through the next plain apply.
   - evidence: the marker assertion at `tests/falsification_590_refresh_templated_check.rs:114`, the follow-up apply at `tests/falsification_590_refresh_templated_check.rs:133`, the latch fixture at `tests/falsification_590_refresh_templated_check.rs:97`, and the control `the_same_guard_without_a_template_is_checked_not_run`.
4. [falsified] C4 — Each half of the change was reverted alone on t2build: hashing the raw resource again made the three templated tests fail and left the control green; resolving with the default secrets but hashing the resolved resource kept all four green.
   - evidence: measured on t2build: branch 4 passed; full fix reverted "1 passed; 3 failed", each panicking at the marker assertion; hash-only revert "1 passed; 3 failed"; secrets-only revert "4 passed"; restored, 4 passed and the tree clean.

## REFUTED

1. [lead] R1 — The issue's first lead: `check_passes_on` fails to resolve a templated resource and so silently reports the check as not passing.
   - corrected: resolution succeeds and the check runs and passes; reverting only the secrets argument leaves the test green. The divergence is the hash of the recorded entry versus the planner's, at `src/core/executor/refresh_seed.rs:140` against `src/core/planner/mod.rs:306`.
