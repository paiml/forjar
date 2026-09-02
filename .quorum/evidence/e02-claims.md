# Quorum evidence — #404 (CRUX audit E02) — adjudicated claims

## CONFIRMED — 9 claims survived refutation

1. [probe] (explains-symptom) The pre-apply drift gate ran BEFORE the executor started the ControlMaster, so every gate query paid a full SSH handshake.
   - evidence: At base, `check_pre_apply_drift` at src/cli/apply_drift.rs:49 is called from src/cli/apply_preflight.rs:89, twenty frames before `apply_machine` starts the master at src/core/executor/machine.rs:78. `build_ssh_args` found no socket and emitted a bare handshake per locked resource — 306 ms median against 6.7 ms multiplexed, 45×. Pinned by `ssh_master_opens_before_any_gate_query` in tests/falsification_e02_controlmaster_before_the_drift_gate.rs, which captures the binary's own `ssh` argv through a shim first on PATH.

2. [probe] (explains-symptom) The gate was SEQUENTIAL across machines while `forjar drift` had fanned the identical work out since FJ-1396.
   - evidence: `for (machine_name, lock) in &locks` at src/cli/apply_drift.rs:67 at base. Modelled at 100 machines × 50 resources that is 25–50 minutes of pure SSH setup before any convergence work. Pinned by `gate_fans_out_across_machines`, whose shim brackets each query with start/end markers and holds it open — overlapping brackets are structurally impossible under a sequential loop.

3. [probe] (explains-symptom) The gate ignored every resource selector, and `-g` could not have been honoured even in principle.
   - evidence: `group_filter` was not a parameter of `apply_pre_validate` at all — src/cli/apply_preflight.rs:89 passed machine, tag and resource but no group. Measured on the first cut: `apply -g net` probed the out-of-group resource and left `status: drifted` in `state.lock.yaml` for a resource the same run reported as skipped. `GateScope::covers` is now `resource_ops::resource_filtered_out` inverted, literally. Pinned by the three `gate_is_scoped_by_the_*_filter` cases.

4. [design] With the masters hoisted, the executor's own `start_control_master` must not tear down a socket it merely FOUND.
   - evidence: src/core/executor/machine.rs:78 matched `Ok(_) => true`, claiming ownership of any socket that existed; after the hoist that would close a connection the gate — or a concurrent apply — still owns. Now `Ok(started) => started`. Pinned by `a_control_master_this_run_did_not_open_survives_the_apply`; reverting this one hunk turns exactly that test red.

5. [agy] (explains-symptom) `--exclude`, `--skip` and `--subset` prune `config.resources`, but the gate walked the LOCK, so an excluded resource was still probed and still written `drifted`.
   - evidence: The first cut's own commit message recorded this as "STILL OPEN … a semantics call" and measured it: `apply --exclude alpha-b` left `status: drifted` on alpha-b. Taken: `GateScope::covers` in src/cli/apply_drift.rs now treats a locked id with no declaration as out of scope unconditionally, and `narrow` no longer borrows when the lock carries such an id. Pinned by `gate_is_scoped_by_exclude`, RED with the arm reverted.

6. [agy] (explains-symptom) The ControlMaster hoist narrowed the fleet by `-m` only, so `apply -r one-resource` opened a master to EVERY SSH machine in the file.
   - evidence: `ssh_machines_in_scope` filtered `config.machines` on `machine_filter` alone — an O(fleet) handshake bill for an O(1) apply, the exact cost this issue exists to remove. Taken: `GateScope::machines_in_scope` keeps a machine only if it hosts at least one declared resource the scope covers, and `apply.rs` builds one scope for both the hoist and the gate. Pinned by `resource_and_tag_filters_narrow_the_fleet` and `a_machine_with_nothing_in_scope_gets_no_master`; the first is RED with the narrowing reverted to `-m` only.

7. [agy] The fan-out was unbounded and `r?` on the first failure dropped the findings and the errors of every machine after it.
   - evidence: `std::thread::scope` over the whole lock slice spawned one thread and one ssh per machine at once, and the join loop returned on the first `Err`. Taken: `gate_parallel` now runs waves of `GATE_FANOUT` (32) machines and reports EVERY failure by machine name in one error. `forjar drift` (src/cli/drift.rs) still has neither bound nor aggregation and silently drops a panicked worker; that is recorded as a limit rather than widened into here.

8. [agy] Two copies of the gate predicate — `should_multiplex` re-deriving `tripwire && !force` — were a latent divergence.
   - evidence: The hoist decided "open a socket for the gate" with its own copy of the condition the gate used to decide "run". Taken: one `gate_will_run` in src/cli/apply_drift.rs, called from both.

9. [design] The falsification shim cannot pass vacuously.
   - evidence: The first `ssh` spawn must carry `ControlMaster=yes`; a stale socket makes `start_control_master` run `ssh -O check` first, which lacks the flag, so a leftover socket fails the test rather than passing it; and an empty spawn log panics. `clear_sockets` removes any socket from an earlier run before each case. The agy lane attacked exactly these three routes and confirmed all three closed.

## REFUTED — 3 claims killed

1. [agy] refuted 1/1 — `--plan-file` bypasses `cmd_apply_scoped`, so its gate queries still handshake before any master exists.
   - corrected: The `--plan-file` path never runs the drift gate. `apply_from_plan` (src/cli/dispatch_apply_b.rs) does not call `apply_pre_validate` or `check_pre_apply_drift`; the only caller of the gate is src/cli/apply.rs:92 inside `cmd_apply_scoped`. The executor then starts its own master at src/core/executor/machine.rs:78 before its first remote command, as it always did. There are no pre-master handshakes on that path to hoist. Localhost is excluded from the hoist for the same reason: there is no SSH handshake to amortise.

2. [agy] refuted 1/1 — Sorting the locks by machine name "makes the fan-out deterministic".
   - corrected: As worded, no. Sorting fixes the order the findings are COLLECTED and PRINTED in, which is what the `drift:` lines and the returned vector needed; which machine answers first is the network's to decide and no sort changes it. The comment at the sort site in src/cli/apply_drift.rs now says exactly that.

3. [design] refuted 1/1 — Closing the `--exclude`/`--skip` gap "means deciding whether the gate should still report a locked resource whose declaration is gone, which is a semantics call, not a filter copy" (the first cut's commit message).
   - corrected: It is not a semantics call. The apply gate exists to record drift THIS RUN will repair (forjar#305 — drift is recorded, then converged). A resource the executor cannot touch — pruned by a selector or deleted from the file — is out of scope by the fix's own principle, and writing `drifted` on it is the harm #404 fixed for `-r`. Orphaned lock entries remain `forjar drift`'s job, which still reports them. Decided, implemented, pinned.
