# Quorum evidence — PMAT-222 — the claims put to the lanes

Plan stage (one teamwork lane, before a line of the fix was written):

P1. A `(String, String)` tuple key is the right shape for the re-keyed probe map.
P2. The plan names every site the new type reaches: the two producers, the two consumers, and their tests.
P3. The lock's `input_hash`/`output_hash` are already right per machine because the lock is per machine (the issue's own premise), so only the read side needs re-keying.

Diff stage (three quorum lanes, base pinned at bc2819cc, diff at 3d8a1606):

C1. `ProbeMap` is keyed by (machine, resource id), one map per machine; `probe_all` probes once and records under every machine `is_local` admits and no other; `probe_config` returns it.
C2. `determine_present_action` looks up `(machine_name, resource_id)`; no consumer in src/ reads a digest by resource id alone.
C3. The census decides from the map alone and uses `probe_covers` only to word the reason; `probe_covers` keeps its one definition.
C4. The binary falsifier is RED on the base for the right reason (far planned Update from the local probe) and none of its assertions is vacuous.
C5. The planner-boundary test and the two probe-map unit tests are non-vacuous.
C6. The contract formula, invariant, scenario and rows FALSIFY-PQ-012..014 cite tests that exist by name.
C7. `api::ProbeMap` is re-exported, the surface ceiling is 12, the changelog says so, and no README, book or contract sentence is now false.
C8. Every touched src/ file stays under 500 lines; no unwrap/expect/#[allow] added outside tests.
C9. No config shape plans WORSE under the new key.
C10. No plan path lost its probe map (dry-run, plan file, MCP, verb).

Every lane verdict was a claim: the orchestrator re-ran the acceptance tests, the touched suites, clippy, fmt, gate G and the workspace suite itself. No lane ran cargo.
