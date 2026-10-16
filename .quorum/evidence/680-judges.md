# Judges — forjar#680: apply -r a runs with b's secret unset and never renders b

Lanes and their findings: see `680-lanes.md`. Line numbers are at head
345c3e30. Every probe below ran on a build host at that head with
`cargo test --test falsification_680_apply_r_ignores_out_of_scope_secret`,
and the tree was restored and clean afterwards.

## CONFIRMED

1. [scoped] C1 — `apply -r a` succeeds while out-of-scope `b` references an unset secret, writes `a` and its dependency `dep`, and never renders `b`, so a scoped apply needs only the secrets of its own closure.
   - evidence: `tests/falsification_680_apply_r_ignores_out_of_scope_secret.rs:90` asserts the scoped apply exits 0, `tests/falsification_680_apply_r_ignores_out_of_scope_secret.rs:92` reads `a` back as `plain`, and `tests/falsification_680_apply_r_ignores_out_of_scope_secret.rs:98` asserts `b` was never written; the fixture's `depends_on: [dep]` is at `tests/falsification_680_apply_r_ignores_out_of_scope_secret.rs:64`. At 345c3e30: 2 passed, 0 failed.
2. [control] C2 — The fixture is live: an unscoped `apply` of the same config fails and the failure names `unset-key`, so C1 passing is a property of the `-r` scope and not of a secret that happens to resolve.
   - evidence: `tests/falsification_680_apply_r_ignores_out_of_scope_secret.rs:107` runs the unscoped apply, `tests/falsification_680_apply_r_ignores_out_of_scope_secret.rs:116` asserts it failed and `tests/falsification_680_apply_r_ignores_out_of_scope_secret.rs:120` asserts the output contains `unset-key`. It passed at 345c3e30 and under all three probes below, so the control never moved with the code it guards.
3. [before] C3 — Resolving every resource's templates before `resolve_selection` in `cmd_apply_scoped` turns the scoped test red, so the test catches the exact defect #674 found in `plan -r`.
   - evidence: probe 1 inserted a loop calling `resolve_resource_templates_with_secrets` over `config.resources` directly above the call: 1 passed, 1 failed. `tests/falsification_680_apply_r_ignores_out_of_scope_secret.rs:90` failed with `secret 'unset-key' not found`; the control stayed green.
4. [dropped] C4 — Dropping the `resolve_selection` call turns the scoped test red, because `b` stays in the config and the executor then resolves it, so removing the scoping cannot pass silently.
   - evidence: probe 2 replaced the call with `let _ = (&selectors, verbose);`: 1 passed, 1 failed. `tests/falsification_680_apply_r_ignores_out_of_scope_secret.rs:90` failed with `secret 'unset-key' not found` while the control at `tests/falsification_680_apply_r_ignores_out_of_scope_secret.rs:107` stayed green. This is the receipt's falsification.
5. [header] C5 — A whole-file resolve placed after `resolve_selection`, where the executor runs, leaves both tests green, as the file header at `tests/falsification_680_apply_r_ignores_out_of_scope_secret.rs:12` says, because by then `b` is no longer in the config.
   - evidence: probe 3 inserted the same loop directly below the call: 2 passed, 0 failed. The header's claim is measured, not asserted, so the test pins the scoping and nothing narrower.
