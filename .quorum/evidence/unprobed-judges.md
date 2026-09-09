# Quorum evidence — PMAT-221 — adjudicated claims

## CONFIRMED

1. [action] THE CENSUS CHANGES NO ACTION — every counter and every PlannedChange.action is what it was before the diff; the census is a pure read of the finished change set, taken after propagation.
- evidence: the fallthrough this ticket is about is src/core/planner/mod.rs:352 at the merge base, and the diff adds no branch to it; the census in src/core/planner/unprobed.rs:46 filters the finished changes and returns a separate list. All three diff lanes confirmed, and the orchestrator's rerun of the eight unit cases includes the_census_never_changes_the_action.

2. [rule] NOOP ONLY, AFTER PROPAGATION — a resource that will run anyway hides nothing behind a missing probe, and naming it would be the unconditional banner the forjar#342 contract forbids; a NoOp a rebuilt prerequisite just promoted is no longer counted as silence.
- evidence: src/core/planner/unprobed.rs:46 selects NoOp rows, and the call sits after propagate_changes in src/core/planner/mod.rs; the binary case at tests/falsification_planner_names_what_it_did_not_probe.rs:288 asserts an Update-planning task is absent from the list on the TTY and in --json.

3. [definition] ONE PREDICATE — probe_covers wraps the transport predicate once, and the probe, the executor's pre-plan probe and the census all ask it, so what the census reports as unmeasured is exactly the set the probe skipped.
- evidence: at the merge base the predicate was inlined twice, src/core/task/probe.rs:293 and src/core/executor/mod.rs:422; both sites now call probe_covers, and the_probe_coverage_predicate_has_one_definition counts the wrapping site and fails on a second copy in either file.

4. [seal] SEAL COMPATIBILITY — an empty census serialises to the bytes the base produced, so every plan file sealed before the field existed still verifies; a non-empty census round-trips through save and load, and a malformed one is refused rather than read as empty.
- evidence: the field is skipped when empty at src/core/types/plan_types.rs:60; the reader that built the body at src/cli/plan_file.rs:253 on the merge base now reads unprobed_from_doc and refuses a bad shape as PLAN_MALFORMED; three new cases in src/cli/tests_plan_file.rs pin all three behaviours and the seal verifies in each.

5. [surfaces] EVERY SHIPPED PLAN SURFACE — the TTY rendering, plan --json, the MCP/verb PlanOutput, apply --dry-run text and JSON, the sealed plan file and its apply --plan-file preview all carry the TOTAL list and fold the prose into the one disclosure.
- evidence: the forjar#342 sites at the merge base, src/cli/print_helpers.rs:107, src/cli/plan_json.rs:78 and src/mcp/handlers.rs:145, each gain the third blind spot through one value function reached by a named shim; the verb case the_verb_surface_carries_the_census and the JSON case pin two of them from the real binary.

6. [wording] NOT "N DRIFTED" — the disclosure counts what the plan did not MEASURE, says so in those words, and names forjar apply --refresh and forjar drift as the instruments that can answer.
- evidence: the sentence is a value beside scope_disclosure at src/cli/print_helpers.rs:107 on the merge base, None when the list is empty; the reason strings name the machine this host does not answer for; the binary cases assert the phrase did not measure and the literal forjar drift.

## REFUTED

7. [breakage] THE PLAN LEFT ~50 STRUCT LITERALS UNADDRESSED — the teamwork lane read the new field as a compile break across the test tree and asked for an on-the-fly census computed at presentation time instead.
- corrected: an on-the-fly census would be a second measurement taken after the plan and could disagree with it, and it cannot travel with a sealed plan file. The literal sites were enumerated and edited by script — twenty-one files including examples/planner_proof_sat_why.rs, which the first sweep missed and clippy --all-targets caught; struct-update sites were left to their base. Measured: cargo test --workspace, 314 binaries, 19,603 passed, 0 failed.

8. [surface] THE PLAN-FILE PREVIEW WAS MISSED — apply --plan-file's preview of the sealed body is a shipped plan surface the plan did not name, so a reviewer would read = beside a resource nothing looked at.
- corrected: the lane was right; the surface was added to phase 2's scope and wired through the same value function (src/cli/apply_from_plan.rs). The PMAT-220 shape repeats here: a surface enumerated from memory is the one that stays silent, which is why the contract now lists every surface by name.

9. [keying] THE CENSUS HIDES A REGRESSION — the probe map is keyed by resource id, so a resource declared on a local and a remote machine carries the local probe's verdict for the remote row, and a plan that reports the remote row as measured would be wrong.
- corrected: pre-existing at src/core/task/probe.rs:176, the insert by id on the merge base, and left unchanged on purpose; the census asks the machine predicate before the map, so the remote row is named as unmeasured even though the map has an entry, pinned by a_local_probe_says_nothing_about_a_remote_machine. The keying itself is filed as forjar#499 rather than widened under this ticket.

10. [contract] EVERY FALSIFY ROW RESOLVES — one diff lane refuted the amendment: FALSIFY-PQ-009 bundled the plan-file rule into its sentence but cited the Update-path test, and a_sealed_plan_file_carries_the_census was cited by no row at all; the other two lanes named the same gap.
- corrected: PQ-009 was narrowed to the rule its test pins and FALSIFY-PQ-011 added citing the plan-file case; gate G re-run after the edit: 40 contracts validate, pv lint 0 errors, every citation resolves.
