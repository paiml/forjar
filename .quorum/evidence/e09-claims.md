# Quorum evidence — #412 (CRUX audit E09) — adjudicated claims

## CONFIRMED — 6 claims survived refutation

1. [probe] (explains-symptom) `post_apply` fired TWICE per resource under `--parallel` and once sequentially, and which scheduler a resource met decided what it got.
   - evidence: on the wave path the hook ran in the spawned thread and again through the record phase's verification (`record_wave_outcomes` at src/core/executor/machine_wave.rs:150 reached `post_apply_failure` at :231 after `execute_wave_io` at :18 had already run the hook at :141). Measured on 1.24.0 with two independent resources, only the flag differing. Pinned by `hooks_fire_exactly_once_per_resource_on_both_paths` in tests/falsification_e09_one_scheduler.rs — the hook appends to a counter file; with `src/core/executor` reverted to main the count is 2 on the wave path.

2. [probe] (explains-symptom) Five behaviours existed on the sequential path only: `--retry`, the run's `meta.yaml` resource map, `--trace`'s script echo, `--progress`, and the FJ-2701 task input cache.
   - evidence: src/core/executor/resource_ops.rs:17 modelled retryability for the single-resource executor (`Failed { should_stop, retryable }` and the #165 arm at :254) and nothing on the wave path read it; the wave path never wrote `meta.yaml`'s per-resource map, never echoed the script under `--trace`, never counted for `--progress`, and re-ran every cached task. Pinned by the four cases in tests/falsification_e09_one_scheduler_b.rs (`retry_reruns_the_failed_resource_the_same_number_of_times`, `trace_prints_the_generated_script_on_both_paths`, `progress_reports_every_resource_on_both_paths`, `the_input_cache_holds_on_both_paths`), each comparing the two paths on the same fixture.

3. [probe] (explains-symptom) A thread panic in a wave was recorded against the resource at index 0 rather than the one that died.
   - evidence: the join arm at src/core/executor/machine_wave.rs:81 printed "wave execution thread panicked" and :96 recorded "thread panicked" with no resource identity; the `WaveResult` now carries `resource_id` (machine_b.rs) so the record phase names the resource that failed. Pinned by `a_failure_is_attributed_to_the_resource_that_failed`, RED with the executor reverted.

4. [design] The sequential path is the wave scheduler at width 1 — one implementation, one feature set — and sequential output, locks and events are unchanged.
   - evidence: `apply_machine` at src/core/executor/machine.rs:56 chose between `execute_parallel_waves` (:268) and the single-resource loop at :191. The schedule is now width-1 waves in plan order, and `execute_single_wave` has NO width branch: every wave goes through `execute_wave_parallel` → `execute_wave_io` → `record_wave_outcomes`. The second implementation is gone from the tree: `apply_single_resource`, `execute_resource`, `handle_resource_output`, `should_skip_single`, `resource_filtered_out` (src/core/executor/resource_ops.rs:171-470 at base) and `apply_and_record_outcome` (helpers.rs) are deleted, resource_ops.rs is 501 → 171 lines, and the `should_stop`/`retryable` flags nobody read any more are gone from `ResourceOutcome::Failed`. `the_lock_is_identical_between_the_two_paths` and `the_event_stream_and_the_run_log_agree_between_the_two_paths` compare the artefacts byte-for-byte after scrubbing run ids and timestamps only.

5. [design] The record phase has its own module and its own struct, so neither executor file approaches the 500-line budget and the wave's facts travel as one value.
   - evidence: `record_wave_outcomes` moved from src/core/executor/machine_wave.rs:150 to machine_wave_record.rs; `WaveRecord` replaces the eleven positional parameters the function had grown; `execute_wave_parallel` at src/core/executor/machine_b.rs:147 builds it once. `prepare_wave_resources` (machine_b.rs:217) gained the input-cache check as its own helper so its cognitive complexity fell from 27 to 17.

6. [design] The parity tests cannot pass by construction alone.
   - evidence: every case runs the SAME fixture twice — once with `--parallel`, once without — through the built binary and compares the results to each other, not to a constant; the fixtures carry two or three resources with real hooks that append to files, so a scheduler that ran nothing would produce empty counter files on both paths and fail the `== 1` assertion, not pass an equality.

## REFUTED — 3 claims killed

0. [design] refuted 1/1 — (the worker's first cut) "There is one scheduler."
   - corrected by the agy lane: the first cut had only RELOCATED the fork — `execute_single_wave` still routed `wave.len() == 1` to `apply_and_record_outcome` and everything else to `execute_wave_parallel`, so the parity tests were passing because the five features had been PORTED to both sides, which is the option the judges rejected. Fixed in the same branch by deleting the width-1 branch and the whole single-resource path it called; the tree then compiles only through the wave path (clippy `-D warnings` listed every retired function as dead, which is the proof the fork was the only caller).

1. [design] refuted 1/1 — Keeping two schedulers and porting the five features to the wave path is the smaller change.
   - corrected: It is the change #393/#394 already made twice this cycle (run log, verification), each time on the wave path, leaving the two-implementations problem intact — the next divergence is one feature away. The ticket's success criterion is a parity test that stays green because there is one implementation; that is what width-1 waves give.

2. [probe] refuted 1/1 — The orchestrator's falsification (executor reverted to main) proves each hunk individually.
   - corrected: It proves the CHANGE as a whole (0 of 4 green in the first binary against main's executor); the per-hunk table the worker was briefed to produce was not produced within its turn budget and is recorded as a gap in the receipt rather than claimed. The double-fire, the attribution and the lock/event parity are each pinned by a distinct test, so a partial revert of any one of those three hunks is caught by its own case.
