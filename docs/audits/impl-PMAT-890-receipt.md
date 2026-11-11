# IMPL PMAT-890 receipt: `apply -t` includes the `depends_on` closure (PR #899)

## What changed

PR #899 makes `-t/--tag` a positive selector in the apply filter pipeline
(`src/cli/apply_selection/closure.rs`): the tagged resources are closed over
`depends_on` exactly like `-r`, so a tagged deploy never runs against a
prerequisite that was not converged. The same branch carries `plan --param`,
the `action: failed` run-log record (#688), the no-state refusal in `plan`
(#880) and the drift lock-file naming (#585).

This lane (a) merged `origin/main` into the branch, (b) made fmt, clippy and the
touched tests pass, (c) removed one new untyped-error site the branch added.

## Conflict resolution

Two files conflicted.

- `src/cli/apply_selection/closure.rs`, doc comment of `Selectors`. Main (#950)
  rewrote it to say `-m` is carried (`machine`) only so a contradiction with
  `--only-machine`/`--exclude-machine` is refused by name; its last sentence still
  said `-t` is "only existence-checked". The branch's side said `-t` is a positive
  selector closed over `depends_on`. Both intents are true of the code (`machine`
  field from main, `tag` closure from the branch), so the merged comment keeps
  main's `-m` sentences and the branch's `-t` sentence, and drops main's stale
  "tags stay a plan-level filter" clause that the branch makes false.
- `docs/roadmaps/roadmap.yaml`. Both sides appended rows at the same spot: main
  added PMAT-724, 986, 987, 988, 989; the branch added PMAT-890. Neither side's
  row is stale, so the result keeps all of main's rows unchanged, then the
  PMAT-890 row, each with its own fields.

## Five whys

Red 1: `test_fj017_plan_nonexistent_state_dir` failed.
1. Why red: the test calls `cmd_plan` with a missing state dir and `unwrap()`s.
2. Why does that now error: the branch added `missing_state_refusal` to `cmd_plan_with_pins` (GH-880).
3. Why was the test not updated: the #880 commit changed the contract, not the old test that encoded "everything shows as Create".
4. Why does that matter: two tests held opposite contracts for the same input.
5. Root cause: a contract change without its old test. Fix at the owner: the test now asserts the refusal names `no state at` and `--state-dir`.

Red 2: the untyped-error ratchet counted +1 site over main.
1. Why: the branch added `pub fn require_state_dir(..) -> Result<(), String>` in `state_visibility.rs`.
2. Why a second one: `src/cli/helpers.rs` already has a `require_state_dir` with different wording and `exists()` semantics, and the branch needed `is_dir()` plus the `--state-dir` hint.
3. Why a new `Result<_, String>`: the nearest caller returns `Result<(), String>`, so the author copied its shape.
4. Why it matters: `scripts/ratchets/untyped-error-sites.json` "MAY ONLY SHRINK".
5. Root cause: the helper's job is to produce the refusal text; the caller owns the `Err`. It is now `missing_state_refusal(&Path) -> Option<String>`, so the branch adds no site.

Not caused by this PR (open, see verdict): see "Remaining reds".

## Contract and falsifier tests

- `src/cli/tests_apply_selection_tag.rs`: `tag_selection_pulls_in_untagged_dependency`, `no_tag_selects_everything`.
- `src/cli/tests_apply_selection_closure.rs`: `an_unknown_tag_is_an_error_and_a_known_tag_selects_its_closure`.
- `src/cli/state_visibility.rs`: `missing_state_dir_errors_and_names_the_flag`, `existing_state_dir_passes`.
- `src/cli/tests_plan.rs`: `test_fj017_plan_nonexistent_state_dir` (now asserts the refusal).
- Integration: `falsification_688_failed_resource_action_failed`, `falsification_query_runlog`, `falsification_runlog_doctor_imagelog`, `falsification_store_validate_gc`, `falsification_apply_machine_contradiction`.
- No test gave two verdicts on the same inputs (no flips).

## Remaining reds, not fixed here

1. `falsification_untyped_error_sites_ratchet::the_live_tree_holds_both_ceilings`:
   measured `untyped_error_fns: 790, lock_literals_outside_state: 154` against
   ceilings 778 and 153. `origin/main` itself measures 790 and 154 (same counting
   rules, `git ls-tree` of `origin/main`), so it is red on main and not caused by
   this PR. The branch now equals main's count.
2. CI gate D (`scripts/dogfood/docs.sh`), README line 197 `forjar plan -f forjar.yaml`
   fails with `no state at <sandbox>/state`. `forjar init` creates `state/`, so the
   README flow is correct for a user; the gate's sandbox copies only the fixture
   YAML and never creates `state/` as `init` would. The fix belongs in
   `scripts/dogfood/docs.sh` (create `state/` in the work dir), which is outside
   this lane's scope. Reproduced on gx10 with the built binary.
3. CI `dogfood-guards` named test target `falsification_coverage_gate_mutation_scope`,
   removed on main by #973; the merge of main brings that fix.

## Commands run (host gx10 through fc-pre.sh)

- `cargo fmt --all -- --check`: exit 0
- `cargo clippy --all-targets -- -D warnings`: exit 0
- `cargo test --lib`: exit 0 (13794 passed, 4 ignored)
- `cargo test --test falsification_untyped_error_sites_ratchet`: exit 101 (790 > 778, same on main)
- `cargo test --test` for the five integration tests above: exit 0 each
- `bash scripts/dogfood/docs.sh` (debug binary, before the fix): exit 1 on line 197, see red 2

verdict: PARTIAL - fmt, clippy and the touched tests pass; the ratchet is red on main too and gate D needs a change in scripts/dogfood/docs.sh, outside scope

IMPL-PMAT-890-RECEIPT-END
