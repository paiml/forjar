# Implementation receipt — PMAT-590 — `apply --refresh` and a templated `completion_check`

verdict: PASS — on main a guard whose `completion_check` carries a `{{params.*}}` template is seeded by `--refresh` exactly like a template-free one, and two new tests pin that; the remote (ssh) variant of the original report is not measured here

## Identity

- ticket PMAT-590 (forjar#590); kind: test (regression pin, no production change)
- PR #946, branch `f20-r2/590-refresh-template`, change commit 7659c882
- files: `src/core/executor/tests_refresh_closure.rs` (+63 −1)

## Root cause (five whys, why a pin and not a fix)

1. #590 (reported on 1.31.0): one `{{params.*}}` template flipped a satisfied guard from `unchanged` to `FAILED` under `apply --refresh`.
2. On main the defect does not reproduce: the seeding path (`refresh_seed::check_passes_on` -> `lockless_check::host_check_passes`) resolves templates before it builds the check script.
3. Nothing tested that order: no test ran `--refresh` over a templated `completion_check`.
4. So a change that builds the check script from the unresolved resource would bring #590 back with every test green.
5. This PR adds the missing pair of tests; it changes no production code.

## Contract and falsifier

Contract: for a `type: task` resource on a local machine, `--refresh` seeds the resource as converged if and only if its `completion_check`, after template resolution, exits 0.

Falsifiers (`core::executor::refresh::tests_refresh_closure`):
- `refresh_seeds_a_templated_guard_like_a_plain_one` — two guards that differ only by the template are both seeded.
- `refresh_does_not_seed_a_templated_guard_whose_check_fails` — negative control: the same template resolving to a missing directory is not seeded.

## Evidence (kfocus, `nice -n 19`, 2026-11-11, `cargo test --lib refresh_closure`)

GREEN at 7659c882:

    test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 13764 filtered out

RED, mutation 1 — `host_check_passes` builds the check script from the unresolved resource (the #590 defect):

    test core::executor::refresh::tests_refresh_closure::refresh_seeds_a_templated_guard_like_a_plain_one ... FAILED
    test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 13764 filtered out

RED, mutation 2 — `host_check_passes` reports every executed check as passing:

    test core::executor::refresh::tests_refresh_closure::refresh_does_not_seed_a_templated_guard_whose_check_fails ... FAILED
    test core::executor::refresh::tests_refresh_closure::refresh_under_r_leaves_non_closure_and_checkless_deps_alone ... FAILED
    test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 13764 filtered out

Both mutations were reverted; neither is in this branch.

## Not covered

- The original report was against a remote (ssh) host. Both tests use the local transport; a remote-only cause is unmeasured.
- The tests cover `type: task` only.

- Three-lane review of PR #946 at f84ece9 (agy, Gemini 4 Carbon): 3/3 PASS; the refuted claims are listed under `quorum.refuted_claims` in the receipt. One lane finding (a test doc comment) is fixed in e9cdfcad. Quorum receipt: `.quorum/f20-r2-590-refresh-template.json`, written by agy and committed unedited.

IMPL-PMAT-590-RECEIPT-END
