# Quorum evidence — PMAT-222 — adjudicated claims

## CONFIRMED

1. [key] ONE MAP PER MACHINE — `ProbeMap` is keyed by (machine, resource id) and backed by one map per machine, so a lookup by two `&str` allocates nothing in the planner's inner loop; `probe_all` probes a resource once and records the digest under every machine `is_local` admits and under no other.
- evidence: src/core/task/probe.rs:80 declares `by_machine: HashMap<String, HashMap<String, IoDigest>>`; src/core/task/probe.rs:207 is the new `probe_all` signature returning `ProbeMap`; src/core/task/probe.rs:224 inserts under each admitted machine. All three lanes confirmed; the two unit tests in tests_probe.rs measure it.

2. [consumer] THE PLANNER READS BY ROW — `determine_present_action` asks the map for `(machine_name, resource_id)`, and no consumer in src/ reads a digest by resource id alone any more.
- evidence: src/core/planner/mod.rs:361 is the lookup; the orchestrator's rerun of lane 2's own grep (`probes.get(`, `HashMap<String, IoDigest>` across src/) finds only the struct field. All three lanes confirmed.

3. [census] THE MAP DECIDES, THE PREDICATE WORDS — `unprobed_reason` returns early when the map holds `(machine, resource)`, and `probe_covers` is consulted only to word the reason; the transport predicate is still wrapped exactly once.
- evidence: src/core/planner/unprobed.rs:81 is the early return, src/core/planner/unprobed.rs:84 the wording branch; the orchestrator's count of `crate::transport::controller_answers_for` is 1 in probe.rs, 0 in executor/mod.rs, 0 in unprobed.rs; `the_probe_coverage_predicate_has_one_definition` still passes.

4. [falsifier] RED FOR THE RIGHT REASON — the binary case converges on [box, far] with both loopback, moves far to a TEST-NET address, rewrites the local input, and asserts box=update, far=no_op, the census exactly [(build, far)], and a TTY with one rebuild and one unchanged row naming build@far only.
- evidence: tests/falsification_probe_answers_for_the_tree_it_was_taken_on.rs:190 asserts far is `no_op`, :195 pins the census list, :202 the TTY counters. Measured by the orchestrator on the pre-fix tree: FAILED with `left: Update, right: NoOp`; on the fixed tree 2 passed. No lane could run it; the measurement is the orchestrator's.

5. [boundary] THE PLANNER BOUNDARY AGREES — the planner-level case hands in the map the caller built, rewrites the local input and expects Update on box, NoOp on far and far named; the two probe-map unit tests pin keying and the shared digest for two local machines.
- evidence: src/core/planner/tests_unprobed.rs:239 passes `ProbeMap::default()` where a `HashMap::new()` used to compile, and :198 rewrites the mixed-machine doc to the new rule; the stale-local case failed on the pre-fix tree (`left: Update, right: NoOp`) and passes after. 263 lib tests over planner, task::tests_probe and the api ran green.

6. [contract] THE CORPUS NAMES THE NEW RULE — the `unprobed(plan)` formula reads `probe(m, r) absent`, one invariant and one scenario describe the forjar#499 state, and rows FALSIFY-PQ-012..014 cite the binary and boundary tests by their exact names.
- evidence: contracts/plan-declares-its-quantifier-v1.yaml:292 opens FALSIFY-PQ-012; gate G (`scripts/dogfood/contracts.sh`) re-run by the orchestrator: 40 contracts validate, citations resolve. All three lanes grepped the cited names.

7. [scope] UNDER 500 LINES, NO ESCAPE HATCH — every touched src/ file stays under the ceiling and no `unwrap`, `expect` or `#[allow]` was added outside tests.
- evidence: the orchestrator's `wc -l`: src/core/planner/mod.rs 488, src/core/task/probe.rs 352, src/core/planner/unprobed.rs 91; clippy `--all-targets -D warnings` exit 0 on the final tree. All three lanes confirmed.

8. [risk] NO CONFIG PLANS WORSE, ONE PLANS DIFFERENTLY — two machine names that are both this host share the one digest; the only shape that changes is a machine alias this host answers for that `probe_covers` misses, which now keeps config-hash planning AND is named where before it planned Update through the id key.
- evidence: lane 3 named the alias case under C9 and lanes 1–2 found none; the orchestrator judged it the intended trade — src/core/task/probe.rs:224 records only what was measured, and src/core/planner/unprobed.rs:84 names the rest. The predicate's coverage is forjar#485/#495's concern, unchanged by this diff.

9. [paths] EVERY PLAN PATH STILL PROBES — `plan()` builds the map through `probe_config` and the executor's pre-plan probe builds its own from resolved resources; no dry-run, plan-file, MCP or verb path lost its map, so nothing is newly named as unprobed.
- evidence: the forjar#497 suites re-run green by the orchestrator (`falsification_planner_names_what_it_did_not_probe` 6 passed, `falsification_plan_json_discloses_its_blind_spot` 5 passed); lane 1 grepped every `plan_with_probes` caller. All three lanes confirmed.

## REFUTED

10. [plan] A TUPLE KEY WOULD DO — the plan proposed `HashMap<(String, String), IoDigest>`; the teamwork lane refuted it because `(String, String)` does not implement `Borrow<(&str, &str)>`, so every lookup in the planner's loop would allocate two strings.
- corrected: one map per machine, src/core/task/probe.rs:80, with `get(&str, &str)` allocating nothing; the lane's finding is quoted in the struct's doc.

11. [plan] THE PLAN NAMED EVERY SITE — it named the two producers, the two consumers and their tests; the teamwork lane refuted it with ten more call sites in planner tests, the `api.rs` re-export and its ceiling, and `core/task/mod.rs`.
- corrected: all ten call sites now pass `ProbeMap::default()` (src/core/planner/tests_unprobed.rs:239 is one), src/api.rs:91 re-exports the type, src/tests_api.rs:155 carries the ceiling of 12; the lib target compiles and 263 targeted tests pass.

12. [premise] THE LOCK IS RIGHT PER MACHINE — the issue's own premise, that `record_io_hashes` is already right per machine because the lock is; the teamwork lane refuted it: the writer hashes the controller's tree into every machine's lock, and `check_task_input_cache` reads it back for a remote `cache: true` task.
- corrected: filed as forjar#501 with both sites and the base-directory mismatch named, not widened here; after this diff no probe is compared against a remote row, so the wrong number is inert for plan and live only for the cache.

13. [ceiling] THE SURFACE IS 12 — all three diff lanes confirmed C7 by reading the literal; the workspace gate refuted it: rustfmt had wrapped the five-item re-export over three lines and the ceiling test counts items per `pub use crate::` line, so it read 8.
- corrected: two single-line re-exports, src/api.rs:91 with the reason beside it; `api::tests` re-run 7 passed; the five whys are in `.pmat/jidoka.jsonl` and the receipt names the line-based counter as a fragility left in place.
