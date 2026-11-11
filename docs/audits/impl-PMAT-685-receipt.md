# Implementation receipt — PMAT-685 — lint and the quality gate lint the apply script of a secret-bearing resource

verdict: PASS — `forjar lint` and the quality gate now generate and lint the apply script of a resource that carries `{{secrets.*}}`, from a copy whose secret references are placeholders; a script that still cannot be generated is reported, not skipped.

## Identity

- ticket PMAT-685 (forjar#685; master forjar#901); kind: code
- PR #903, branch `forjar-cop/packC`. The same PR carries the cron check fix of PMAT-848, with its own receipt `docs/audits/impl-PMAT-848-receipt.md`.
- files: `src/cli/lint.rs`, `src/cli/tests_lint_secret.rs`, `src/core/codegen/dispatch.rs`, `src/core/quality_gate/checks.rs`, `src/core/quality_gate/mod.rs`, `src/core/quality_gate/tests_secret_apply.rs`, `src/core/resolver/mod.rs`, `src/core/resolver/resource.rs`, `docs/roadmaps/roadmap.yaml` (row GH-685 renamed PMAT-685)

## Root cause (five whys)

1. A task whose command held both `{{secrets.x}}` and a hard-coded key passed `forjar lint` and the quality gate clean.
2. The apply script of that resource was never linted.
3. Both callers dropped the `Err` of `codegen::apply_script` without a word.
4. `apply_script` is the I8 chokepoint: it refuses a resource with an unresolved secret reference.
5. Lint and the gate read the UNRESOLVED config, so every secret-bearing resource hit that refusal.

## Contract and falsifier

Contract: `codegen::apply_script_for_lint(resource)` equals `apply_script(resource)` when the resource has no unresolved secret reference; otherwise it generates the script from `resolver::with_secret_placeholders(resource)`, in which each `{{secrets.*}}` is `forjar-lint-placeholder`. The copy is never executed or written. `apply_script` and the I8 refusal are unchanged. If the apply script of a secret-bearing resource cannot be generated, lint emits `bashrs: <id>/apply not linted: <error>`.

Falsifiers: `cli::lint::tests_lint_secret::secret_bearing_task_apply_script_is_linted`, `secret_bearing_task_with_clean_apply_script_stays_clean`, `core::quality_gate::tests_secret_apply::generate_scripts_includes_apply_of_a_secret_bearing_resource`, `gate_reports_a_lint_error_in_a_secret_bearing_apply_script`, `masking_leaves_a_resource_without_a_secret_untouched`.

## Evidence (kfocus, `nice -n 19`, 2026-11-11, head e33a2e5 plus this receipt)

RED, with `lint.rs` and `quality_gate/checks.rs` calling `codegen::apply_script` again (`cargo test --lib secret_`):

    test core::quality_gate::tests_secret_apply::generate_scripts_includes_apply_of_a_secret_bearing_resource ... FAILED
    test cli::lint::tests_lint_secret::secret_bearing_task_with_clean_apply_script_stays_clean ... FAILED
    test core::quality_gate::tests_secret_apply::gate_reports_a_lint_error_in_a_secret_bearing_apply_script ... FAILED
    test cli::lint::tests_lint_secret::secret_bearing_task_apply_script_is_linted ... FAILED
    test result: FAILED. 224 passed; 4 failed; 0 ignored; 0 measured; 13547 filtered out; finished in 0.49s

GREEN:

    cargo test --lib secret         test result: ok. 272 passed; 0 failed; 0 ignored
    cargo test --lib lint           test result: ok. 89 passed; 0 failed; 0 ignored
    cargo test --lib quality_gate   test result: ok. 47 passed; 0 failed; 0 ignored

`cargo fmt --all -- --check` rc=0; `cargo clippy --all-targets -- -D warnings` rc=0.

## Not in this PR

#663 and #738, which the PR also closes, were fixed on main by ac292488 (#767); nothing for them changes here.

IMPL-PMAT-685-RECEIPT-END
