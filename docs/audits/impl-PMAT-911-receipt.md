# Implementation receipt — PMAT-911 — `apply --refresh-only` re-reads only the resources the flags select

verdict: PASS — on PR #941's branch, `--refresh-only` narrowed by `-r` or by `-t` leaves every lock entry outside the selection untouched; the `-t` case was RED before the fix in this receipt's commit and is GREEN after it.

## Identity

- ticket PMAT-911 (forjar#911, train "apply selectors and pre-checks correctness"); kind: code
- PR #941, branch `fix/refresh-only-selection-470`, merge base with `main` @ 5b8f8123
- closes #470 and #435; #432 is documented, not fixed (see the PR body)
- files touched by the `-t` fix: `src/cli/dispatch_apply_check.rs`, `src/cli/tests_refresh_only_selection.rs`, `docs/roadmaps/roadmap.yaml` (row `GH-911` renamed to `PMAT-911`)

## What this receipt covers, and what it does not

Measured by forjar-cop on kfocus for this receipt: the `--refresh-only` selection path (#470), including the `-t/--tag` defect that agy's review found at a7a77e0.

Not re-measured here: the canary pre-hook change (#435) and the `--plan-file` notice (#432). Their evidence is the PR author's run recorded in the PR body ("canary falsifier 3 passed; lib filter 35 passed") and the PR's CI.

## Root cause of the `-t` defect (five whys)

1. `forjar apply --refresh-only -t alpha` refreshed resources that do not carry `alpha`.
2. `cmd_apply_refresh_only` narrowed the config with `resolve_selection` only.
3. `resolve_selection` existence-checks `-t` but does not narrow by it: in `apply` and `--check` the tag is a plan-level filter applied later by the executor (`has_tag`).
4. The refresh path has no plan step, so nothing downstream applied the tag.
5. The #470 tests covered `-r` only, so the missing tag step was not seen.

## Contract and falsifier

Contract: after `select_for_refresh(config, selectors, tag, _)`, `config.resources` holds exactly the selector closure, and when `tag` is `Some(t)`, only those resources with a tag equal to `t` (the executor's `has_tag` rule: exact match, no prefix match).

Falsifiers, in `src/cli/tests_refresh_only_selection.rs`:

- `select_for_refresh_tag_rule`: no tag keeps `[a, b]`; `alpha` keeps `[a]`; `beta` keeps `[b]`; `alph` keeps nothing.
- `tag_scoped_refresh_leaves_untagged_entry_untouched`: both files drift out-of-band, the refresh runs with `-t alpha`, and the lock entry of `b` (tag `beta`) is unchanged while `a` is re-read.

## Evidence (kfocus, rust 1.99.1, `nice -n 19`, 2026-11-11)

RED, with the tag narrowing removed from `select_for_refresh` and the tests kept (`cargo test --lib refresh_only_selection`), 2 of 4 failing:

    select_for_refresh_tag_rule: assertion `left == right` failed  left: ["a", "b"]  right: ["a"]
    tag_scoped_refresh_leaves_untagged_entry_untouched: "Refresh complete: 2 resources queried, 2 drifted"
    test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 13768 filtered out

GREEN, with the fix (`cargo test --lib refresh_only`):

    test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 13767 filtered out

Checks on the same tree: `cargo fmt --all -- --check` rc=0; `cargo clippy --lib --tests -- -D warnings` rc=0.

Independent repro of the defect before the fix: `handoff/agy/review-forjar-941-a7a77e0.md` (agy, FAIL @a7a77e0).

- Three-lane review of PR #941 at 78b309b (agy, Gemini 4 Carbon): 3/3 PASS; the refuted claims are listed under `quorum.refuted_claims` in the receipt. One lane finding (a doc comment on `Selectors.tag`) is fixed in 835caef0. Quorum receipt: `.quorum/fix-refresh-only-selection-470.json`, written by agy and committed unedited.

IMPL-PMAT-911-RECEIPT-END
