# Quorum evidence — #374 (fix/canary-operator-auth) — adjudicated claims

## CONFIRMED — 9 claims survived refutation

1. [probe] (explains-symptom) `apply --canary-machine` converged the WHOLE fleet for an operator the config does not list, and `--refresh-only` rewrote every lock for them.
   - evidence: at the merge-base `check_operator_auth` was the first line of `apply_execute` (`src/cli/dispatch_apply_b.rs:325`), the LAST stage of the dispatcher, and every early exit — `apply_early_exits` (:94), `apply_pre_checks` (:123), `apply_mode_exits` (:166) — returned above it. `cmd_apply_canary_machine` (`src/cli/apply_variants.rs:38`) and `cmd_refresh_only` (:274) are early exits. Measured on 1.24.0 with `allowed_operators: [alice]` on two machines: `apply --canary-machine sandbox --operator mallory` → 2 machines converged, exit 0. Pinned by `canary_apply_refuses_an_unlisted_operator`, `canary_apply_writes_nothing_when_refused`, `canary_apply_does_not_converge_the_rest_of_the_fleet` and `refresh_only_refuses_an_unlisted_operator` in `tests/falsification_canary_apply_is_authorized.rs`; the control `control_the_ordinary_apply_refuses_an_unlisted_operator` proves the fixture's gate is real.

2. [design] The gate is now positional, not per-exit: `dispatch_apply_cmd` runs `check_operator_auth` before any exit, hook or backup unless the invocation is a pure read.
   - evidence: `src/cli/dispatch_apply_b.rs:16` (`dispatch_apply_cmd` at the merge-base) is where the two GH-211 cross-cutting checks already lived; the operator gate joins them, so an exit added later is gated by default. #370 had patched ONE exit (`--plan-file`) at its own call site; that copy is now redundant, not load-bearing.

3. [probe] (explains-symptom) `--canary-machine` hard-coded `yes = true` into both legs, so the remaining fleet rolled out with no confirmation prompt — for AUTHORIZED operators too.
   - evidence: `src/cli/apply_variants.rs:38` at the merge-base built both `ApplyConfig`s with `yes: true` regardless of the command line. `--yes` is now threaded from the arguments and each leg asks in turn. Pinned by `canary_apply_does_not_imply_yes` (authorized operator, no `--yes`, stdin closed as in CI: the fleet leg must not converge).

4. [design] The read-only modes (`--check`, `--diff-only`, `--output-scripts`, `--dry-run-{graph,cost,verbose}`) stay ungated deliberately, and the line is fail-safe.
   - evidence: `is_read_only_apply_mode` mirrors the dispatch order, names the reads explicitly, and treats anything unnamed — `--refresh-only`, `--plan-file`, `--dry-run`, a plain apply, `--canary-machine --check` — as gated; forgetting a new read can only over-refuse. The reads print what the ungated `check`/`plan`/`graph` verbs print to anybody, and `check_operator_auth` iterates EVERY machine regardless of `-m`, so gating them would cost a listed operator `apply -m theirs --check`. Pinned by `a_listed_operator_still_gets_a_read_with_a_hook` and `a_listed_operator_still_gets_a_canary_rollout` (no regression for an authorized operator) and `a_read_flag_does_not_launder_a_canary_rollout`.

5. [probe] (explains-symptom) A read mode carrying `--pre-script` was still an ungated execution: `apply --check --pre-script deploy.sh --operator mallory` ran `deploy.sh` and then printed check results with no refusal anywhere.
   - evidence: `apply_mode_exits` (`src/cli/dispatch_apply_b.rs:166` at the merge-base) sits BELOW `apply_pre_checks` (:123), which runs the pre-script. An invocation carrying `--pre-script`, `--pre-flight` or `--webhook-before` is therefore not a read whatever else is on it (`runs_an_external_hook`). Pinned by `an_unauthorized_apply_does_not_run_the_pre_script` and `a_read_mode_does_not_launder_the_pre_script_hook` (the script's marker file must not exist).

6. [design] The falsifiers cannot pass vacuously.
   - evidence: `refused()` asserts BOTH a non-zero exit AND the literal `not authorized` on stderr — a clap error or a missing binary fails the second assertion; every mutation case also asserts the filesystem (`canary_file()` / `prod_file()` absent); the fixture is built with `allowed_operators` present, and the control test proves the ordinary path refuses under the same fixture.

7. [probe] (found by the agy lane; explains-symptom) `apply --check --abort-on-drift` read as a read and skipped the gate, yet `--abort-on-drift` runs `cmd_drift` from `apply_pre_checks` — a probe of every host in scope, before the read.
   - evidence: `apply_pre_checks` (`src/cli/dispatch_apply_b.rs:123` at the merge-base) runs `cmd_drift` when `abort_on_drift` is set; the read-only predicate exempted `--check` without looking. `runs_an_external_hook` now includes `abort_on_drift`. Pinned by `a_check_with_abort_on_drift_is_not_a_read` (RED without the change, GREEN with it).

8. [probe] (found by the agy lane; explains-symptom) `check_operator_auth` iterated EVERY machine whatever `-m` said, so an operator listed on `sandbox` alone was refused `apply -m sandbox` for lacking `prod` — and moving the gate to the top extended that refusal to `--refresh-only`, a mode a scoped operator previously had.
   - evidence: `src/cli/dispatch_apply.rs` (`check_operator_auth`, `for (name, m) in &config.machines`) at the merge-base. The check now takes the machine filter and checks the machines the invocation touches; a filter naming nothing in the config falls through to the full iteration (fail-safe). The plan-file caller keeps the full iteration (its request carries no machine). Pinned by `a_machine_scoped_operator_keeps_their_own_machine` (bob on sandbox only: keeps `-m sandbox`, refused `-m prod`, refused unscoped).

9. [probe] (found by the agy lane) The fleet leg of `--canary-machine` asked once PER MACHINE, so an operator with N remaining machines answered N prompts and the first EOF silently aborted the rest.
   - evidence: `cmd_apply_canary_machine` (`src/cli/apply_variants.rs:38` at the merge-base) loops `cmd_apply` per remaining machine with the operator's `yes`, and `cmd_apply` prompts per machine when `yes` is false. It now asks once for the fleet (`confirm_fleet_rollout`) and hands each leg `true`. Pinned by `the_fleet_leg_asks_once_not_once_per_machine` (three machines, two "y" answers: both remaining machines converge, one fleet prompt).

## REFUTED — 4 claims killed

1. [design] refuted 1/1 — "Gate every apply mode, including the reads; simplest and safest."
   - corrected: `check_operator_auth` iterates every machine in the config regardless of `--machine`, so an operator listed on one machine would lose `apply -m theirs --check` with "not authorized for machine 'other'" — a measured refusal for zero confidentiality gain (the same output is available from ungated verbs). The reads stay open; the line is pinned in both directions.

2. [probe] refuted 1/1 — "Fixing the gate position also fixes the unconfirmed rollout."
   - corrected: they are independent defects. Every gate test passes `--yes` explicitly so that the restored prompt (which aborts the canary leg on EOF) cannot mask an unreached gate; `canary_apply_does_not_imply_yes` runs as the AUTHORIZED operator so that the gate cannot mask the missing prompt.

3. [design] refuted 1/1 (agy lane's charge, countered) — "The hook exemption misses `--post-script`."
   - countered: `post_script` runs at ONE site, inside `apply_execute` (`src/cli/dispatch_apply_b.rs:465`), which no read mode reaches; a read invocation carrying `--post-script` executes nothing. Not added to the predicate — adding it would over-refuse a listed operator's `--check --post-script` for no execution avoided. Recorded as divergence, not accepted.

4. [design] refuted 1/1 (agy lane's charge, countered) — "Exempting `--check` is a confidentiality leak."
   - countered: `apply --check` prints what the ungated `forjar check` / `plan` / `graph` verbs print to anyone who can read the config, none of which accepts `--operator`; gating one spelling of the read buys nothing while the others stay open. The line is recorded in the receipt; changing it is a verb-surface decision, not a #374 one.
