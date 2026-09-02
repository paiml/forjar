# Quorum evidence — #390-C — adjudicated claims

## CONFIRMED — 3 claims survived refutation

1. [probe] (explains-symptom) `--json` reported `"error": null` for every FAILED resource, making the machine-readable surface strictly worse than the console.
   - evidence: Reproduced against the built binary. `apply --json` on a failing task emitted `{"resource_id":"boom","status":"failed","exit_code":null,"hash":null,"error":null}`. Cause: `build_resource_reports` read `details["error"]`, a key `record_failure` never wrote — it constructed `details: HashMap::new()`. Fixed at src/core/executor/resource_ops.rs:141. After the fix the same command emits the full `NOT CONVERGED (script exit 1) …` text. Pinned by tests/falsification_390c_json_failure_detail.rs:96.

2. [probe] (explains-symptom) The report mapped the WHOLE persisted lock, so rows for resources this run never executed were reprinted as this run's outcome.
   - evidence: `build_resource_reports(lock)` iterated `lock.resources` unfiltered. Filling the error without fixing this would have upgraded an obviously-contentless stale row into a convincing wrong one, so both halves land together. Fixed at src/core/executor/machine_b.rs:100 by filtering on the union of `counters.converged_resources` and `counters.failed_resources`. Pinned by tests/falsification_390c_json_failure_detail.rs:149.

3. [design] (partially-explains) The fix was correctly deferred from 1.24.0 and is safe only because #390 landed first.
   - evidence: The failure string now lands in `state.lock.yaml`, which is re-serialised and blake3-sidecarred every run and commonly committed. Before #390 bounded all six `record_failure` call sites, an unbounded stderr could have gone into a hashed, committed file. The bound is enforced at src/core/executor/resource_ops.rs:141 by construction — the value written is the same one `failure_text` already excerpted.

## REFUTED — 2 claims killed

1. [probe] refuted 1/1 — The first version of this test suite verified the fix.
   - corrected: It did not. Only 1 of 3 tests went red against reverted code. Two were vacuous: one asserted `!blob.contains("\"error\":null") || blob.contains("JSONC")`, which passes whenever either disjunct holds, and it searched a TOP-LEVEL `resource_reports` that does not exist — the reports live under `machines[].resource_reports`. The other wrapped its assertion in `if let`, so it passed silently whenever the shape was not as expected. Rewritten to parse the real path and assert directly; all 3 now go red against pre-fix code. See tests/falsification_390c_json_failure_detail.rs:74 for the corrected accessor and the note recording the trap.

2. [design] refuted 1/1 — `exit_code` should be filled in the same change.
   - corrected: Not from here. `record_failure` does not receive an exit code — its signature is `(ctx, resource_id, resource_type, duration, error)` — so filling it would mean widening a function with six call sites, several of which (transport error, pre_apply gate) have no exit code to give. `exit_code` stays `None` and is named as a known limit rather than faked from a parsed string.
