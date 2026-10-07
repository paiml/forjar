# Judges — forjar#674 at e23e1011

Three judges per claim: the three r4 lanes. Each item was checked against
the test runs on e23e1011 (281515fc with main c6c8591c merged in) (default and `encryption` builds) and two revert
probes.

## CONFIRMED

1. [scope] C1 — `plan -r a --output-dir D` succeeds with `b`'s secret unset and exports only `a`, because `export_scripts` now receives the plan's selection and skips everything outside it.
   - evidence: `src/cli/plan.rs:111` builds the selection from the filtered plan and `src/cli/print_helpers.rs:325` skips an id outside it. `tests/falsification_674_plan_export_selection_and_secrets.rs:126` asserts the scoped plan exits 0 and `tests/falsification_674_plan_export_selection_and_secrets.rs:137` asserts every exported file starts with `a.`. At e23e1011: 3 passed in both builds.
2. [redact] C2 — An unscoped export names each secret and never resolves it, so neither an unset key nor a key with a value on disk reaches a script.
   - evidence: `src/cli/print_helpers.rs:319` builds the redacted `SecretsConfig`; its provider arm returns `FORJAR_REDACTED_SECRET_<key>`. `tests/falsification_674_plan_export_selection_and_secrets.rs:160` and `tests/falsification_674_plan_export_selection_and_secrets.rs:161` assert both names, and `tests/falsification_674_plan_export_selection_and_secrets.rs:164` asserts no file carries the value that `secrets/known-key` holds on disk.
3. [age] C3 — An export writes an `ENC[age,...]` literal as written: no decrypt in an `encryption` build and no refusal in a default build.
   - evidence: `src/core/resolver/template.rs:203` returns before the FJ-200 decrypt block for the redacted provider. `tests/falsification_674_plan_export_selection_and_secrets.rs:192` asserts `plan -r d` exits 0 and `tests/falsification_674_plan_export_selection_and_secrets.rs:196` asserts the script holds the literal unchanged; green in the default and the `encryption` build.
4. [revert] C4 — The tests fail on the defect they name: the age guard reverted fails two of three, the whole fix reverted fails all three.
   - evidence: with `src/core/resolver/template.rs:203` reverted, `tests/falsification_674_plan_export_selection_and_secrets.rs:192` panicked with "compiled without encryption support" (default) and "FORJAR_AGE_KEY not set" (`encryption`), `tests/falsification_674_plan_export_selection_and_secrets.rs:151` failed too, and `tests/falsification_674_plan_export_selection_and_secrets.rs:126` stayed green. With the fix reverted to main, 0 passed and 3 failed.

## REFUTED

1. [noop] R1 — haiku-4-5 in r2 held that the selection read from `plan.changes` at `src/cli/plan.rs:111` drops a selected resource whose planned action is NoOp, so an unchanged resource would not be exported.
   - corrected: the planner pushes a `PlannedChange` for every resource it plans, NoOp included (`src/core/planner/mod.rs` builds each action and pushes it unconditionally), and existing planner tests count NoOp entries in `plan.changes`. The filtered plan therefore holds every selected id. The haiku lanes of r3 and r4, given the same diff, passed.
2. [age-r1] R2 — sonnet-5 in r1 held that `resolve_template_with_secrets` still decrypts an `ENC[age,...]` literal after substitution, so the redacted export could write plaintext or refuse in a default build.
   - corrected: true at a842da60, so it is refuted against the pushed head only. 281515fc added the early return at `src/core/resolver/template.rs:203`, and `tests/falsification_674_plan_export_selection_and_secrets.rs:196` pins it: reverting that guard turned the test red in both builds, at 281515fc and again at e23e1011.
