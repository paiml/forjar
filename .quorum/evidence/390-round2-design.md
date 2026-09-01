# Quorum evidence — #390 (round 2: fix design)

3 independent judges over 4 proposals.

## Judge 1
Winner: Proposal 1 [minimal] — the only proposal with no FATAL, verifiably implementable against this repo's hardest constraint (resource_ops.rs at exactly 500 lines, 1-for-1 swap), fixing the confirmed root cause at both duplicated sites with a head+tail excerpt that actually surfaces the reporter's pre-build diagnostics. Merge it as the base and graft the items below.

- minimal: correctness=8 implementability=9 operator_value=8
- observability: correctness=6 implementability=3 operator_value=9
- contract: correctness=8 implementability=3 operator_value=8
- parallel-gap: correctness=3 implementability=4 operator_value=4

**Must graft:** From P2: the NOT CONVERGED vs FAILED classification. P1's message still opens 'exit code 1' for a command that exited 0 — the exact frame that sent the reporter hunting a build error for six runs. Classify on the GH-254 marker as a shared const between the emitter (task.rs:253) and the classifier, print 'The command itself SUCCEEDED', and quote the resolved completion_check verbatim. Highest-value idea in the set and line-cheap in P1's new sibling module. HEDGE it: under `timeout:` (task.rs:196-199) or `sudo:` the nested bash does not inherit set -euo pipefail, so 'the command SUCCEEDED' is a claim the module cannot make — check resolved.timeout.is_some()/resolved.sudo and soften the wording there. | From P3 and P2: route output_verify::verify_against_host (output_verify.rs:179-184) through the same choke point. Verified: it prints out.stdout.trim() and DESTROYS out.stderr — the exact mirror of #390 — uncapped, with no log pointer, on a branch P1 does not touch. And task.rs:117-120 (verdict::always_diverged("task=pending")) means every plain `type: task` with no completion_check and no output_artifacts lands there. Without this graft P1 fixes one half of the not-converged family and leaves the other half printing the opposite asymmetry. | From P3: cap and route the three duplicated hook-failure constructors — output_verify.rs:94-98 (pre_apply), :111-115 (post_apply), machine_wave.rs:84-88 (exec_validated_hook). All stderr-only, all uncapped, all reaching the same ProvenanceEvent::ResourceFailed sink P1 is bounding. A cap a neighbouring string can walk around is not a cap. Note the sequential pre_apply gate (resource_ops.rs:226-245) never enters handle_resource_output and needs its own call site — that is the hole that made P2 fatal and P1 must not inherit it. | Fix P1's log_hint before merge: mirror capture_exec_output's `run_id.unwrap_or("run-adhoc")` (resource_ops.rs:324) instead of `let rid = run_id?;`, or the pointer vanishes in the one case where the log […1730 chars elided]

**Gaps none covered:** CASCADE-SKIPPED DEPENDENTS get no report from any of the four. Verified at machine.rs:229-236, machine.rs:311-320 and machine_b.rs:144-152: each does `counters.failed += 1; counters.failed_resources.insert(...)` then continue/return — no lock entry, no ProvenanceEvent, no run log, no failure string. The dossier establishes 4 of the reporter's 5 'failed' resources were exactly these. Every proposal routes only the executed resource through its new choke point, so after any of these fixes the reporter still gets four resources reported failed with nothing attached. | TERMINAL CONTROL BYTES. All four route raw remote stdout to eprintln!/eprint!. cmake, ninja and cargo stdout routinely carry ANSI CSI sequences and bare \r — precisely why the dossier measured cmake's stdout as ~50 `--` progress lines. Up to 2 KiB (P1) or 16 KiB (P4) of those bytes can repaint or overwrite the failure line itself. Only P1's critique raises it; no proposal sanitises. | SECRET REDACTION IS NAMED THEN DEFERRED BY EVERYONE. P1 points at src/core/script_secret_lint.rs, P3 files it as D9 against core::resolver::template::redact_secrets (zero non-test callers). All four move a task's stdout onto the console — […1123 chars elided]

## Judge 2
Winner: Proposal 1 [minimal] — one choke point in a new src/core/executor/failure_text.rs, plus two 1-for-1 call-site swaps at resource_ops.rs:411 and machine_wave.rs:180

- minimal: correctness=8 implementability=9 operator_value=8
- observability: correctness=6 implementability=4 operator_value=9
- contract: correctness=8 implementability=5 operator_value=8
- parallel-gap: correctness=5 implementability=4 operator_value=5

**Must graft:** From Proposal 3: route src/core/executor/output_verify.rs:179-184 (verify_against_host — the INVERTED asymmetry, stdout shown, stderr destroyed) and the three duplicated hook sites (output_verify.rs:94-98, output_verify.rs:111-115, machine_wave.rs:84-88) through the same renderer. This is the largest hole in the winner and it is not hypothetical: src/resources/task.rs:118-120 emits always_diverged("task=pending") for any task with no completion_check and no output_artifacts, and I reproduced two trivial echo tasks failing sequentially through exactly this branch with `apply exited 0 but the host does not report the declared state (check exit 1). task=pending`. P1's fix is completely inert there. output_verify.rs is 310 lines and the change shrinks it, so there is no cap cost. | From Proposal 2: classify NOT CONVERGED vs FAILED via a shared NOT_CONVERGED_MARKER const between the emitter (src/resources/task.rs:252-254) and the executor. The reporter's headline literally read `exit code 1:` for a command that exited 0, and that mislabel is half of why they reached for a caching theory. BUT hedge the prose: do not print 'The command itself SUCCEEDED' when `resolved.timeout.is_some() || resolved.sudo`, because src/resources/task.rs:196-199 nests `timeout N bash <<'FORJAR_TIMEOUT'` which does not inherit the outer `set -euo pipefail` (D7, still open). Trading one confidently-wrong label for a more authoritative one is the failure mode to avoid. | From Proposal 4: whoever lands D1 (#390-A) MUST also call update_run_meta from the wave path, not only capture_output. Verified: src/core/types/run_log_types.rs:71-80 shows summary.failed is incremented only by RunMeta::record_resource, and src/cli/logs.rs:106 returns None for any run with meta.summary.failed == 0. Both P2's and P3's D1 fixes omit it, so `forjar logs --failures` stays blind and the new pointer names a run the log viewer refuses to list. | From Proposal 4: capture BEFORE post-apply verification, and capture on […1714 chars elided]

**Gaps none covered:** THE CASCADE-SKIP SITES — no proposal fixes them, and the dossier says 4 of the reporter's 5 failed resources were dependents. src/core/executor/machine.rs:229-236, machine.rs:311-320 and machine_b.rs:144-152 each do `counters.failed += 1; counters.failed_resources.insert(...)` then continue/return, bypassing record_failure, the lock, events.jsonl and any run log entirely. After ANY of these four proposals the reporter still gets four resources counted failed with no lock entry, no ResourceFailed event, no console detail and no `forjar history` record. P4's PostApply token cannot constrain them because they never call settle. This is the largest uncovered surface in the issue. | THE MACHINE-READABLE SURFACES, FOR THE REPORTER'S ACTUAL ENVIRONMENT. He is on a stateless CI runner where --state-dir defaults to the relative `state` (src/cli/commands/apply_args.rs:61-62) and is deleted with the checkout, so --json is the surface a pipeline reads — and it emits "error": null, "exit_code": null for a FAILED resource (machine_b.rs:89 hardcodes exit_code: None; machine_b.rs:95-99 reads details["error"], a key record_failure never writes — resource_ops.rs:128 `details: HashMap::new()`) […2357 chars elided]

## Judge 3
Winner: minimal

- minimal: correctness=8 implementability=9 operator_value=8
- observability: correctness=7 implementability=3 operator_value=9
- contract: correctness=7 implementability=4 operator_value=8
- parallel-gap: correctness=3 implementability=2 operator_value=4

**Must graft:** From [observability]: the FAILED vs NOT CONVERGED classification, via a NOT_CONVERGED_MARKER const shared between the emitter at task.rs:252-255 and the classifier. The reporter's command exited 0 and forjar printed `exit code 1` — that mislabel is the other half of the misdiagnosis and no amount of stdout fixes it. Print the resolved completion_check verbatim; it is the thing that is actually false and it is named nowhere today. HEDGE the prose: do not say 'The command SUCCEEDED' when resolved.timeout.is_some() or resolved.sudo, because task.rs:196-199's nested `timeout N bash <<'FORJAR_TIMEOUT'` does not inherit `set -euo pipefail` (D7) and the claim is then known-wrong. Say 'the command's own completion_check re-assertion exited 1'. | From [contract]: route output_verify.rs:179-184 through the same choke point. I confirmed it renders out.stdout.trim() and destroys stderr — the exact mirror of #390, in the file P1 leaves untouched. P1's own critique measured that a plain task with no completion_check fails through this arm with no --- stdout section, no cap and no pointer at all. Fixing only the Ok(out) non-zero arm leaves the `task=pending` family in the pre-fix state. | From [contract]: state in the PR body that stderr into events.jsonl is UNBOUNDED today (resource_ops.rs:411 -> ProvenanceEvent::ResourceFailed, append-only, no rotation), so the 2000-byte-per-stream cap is a tightening of the worst case, not a new restriction. Without that framing the cap reads as a regression in review. | From [parallel-gap] and [observability]: absolutize the log pointer. P1's own critique measured it printing the relative `state/gx10/runs/...`, reproducing the very trap the dossier blames for the reporter losing his evidence — --state-dir defaults to relative `state` (apply_args.rs:62). Use std::fs::canonicalize with a fallback to the un-canonicalised path, and add P2's one-line NOTE when state_dir.is_relative() saying a stateless CI runner deletes it with the checkout. | […1238 chars elided]

**Gaps none covered:** CASCADE-SKIPPED DEPENDENTS ARE INVISIBLE, AND THEY WERE 4 OF THE REPORTER'S 5 FAILURES. I read machine.rs:229-236, machine.rs:311-320 and machine_b.rs:144-152: a resource whose dependency failed bumps counters.failed and counters.failed_resources and prints only `JIDOKA: skipping X — depends on failed 'Y'` — no record_failure, no lock entry, no ResourceFailed event, no run log. So `5 failed` in the summary resolves to ONE lock entry and one console block. Every proposal routes only executed resources through its new reporter; none reports or even files this. Only P4's critique raises it. An operator reconciling the count against the evidence hits a second, independent confusion this PR will not touch. | TERMINAL CONTROL CHARACTERS. All four route a remote build's raw stdout through eprintln! to the operator's terminal. cmake/ninja/cargo stdout routinely carries bare \r and ANSI CSI sequences; up to ~2 KiB of those can repaint or overwrite the failure line itself. Only P1's critique mentions it; no proposal sanitises. stdout is far likelier than stderr to carry progress-bar output — that is exactly why the dossier measured cmake's stdout as 15 lines of `--` progress. | SECRET […1973 chars elided]

## Proposals

### [minimal] #390 — one choke point for the failure string: a failed task's stdout reaches the operator, and the message names the log that holds the rest

FOLD IN: nothing from D1–D8. The whole fix is one new sibling module plus two one-line call-site swaps.

The reasoning is that the misdiagnosis in #390 had exactly one cause: the operator's only failure string was a pure function of `exit_code` and `stderr`, so edits that touched only stdout could not change it, and identical output across six runs looked like replay. Put stdout in that string, bounded, and the misdiagnosis becomes impossible — the reporter's `nvcc:` line would have appeared on run 1 and changed on run 2. Nothing in D1–D8 is needed for that, and every one of them costs a wider blast radius.

Critically, this fix works in the two places the reporter actually was:
  * under `--parallel`, where no run log exists at all (measured today: `state/gx10/runs` is not created), because stdout comes from the in-memory `ExecOutput`, not from disk;
  * on a stateless CI runner, where `--state-dir` defaults to the relative path `state` (src/cli/commands/apply_args.rs:60-61) and is deleted with the checkout, because the message is on the console and in `events.jsonl`, not only in a run log.

DELIBERATELY SPLIT OUT, each as its own issue:

* #390-A (D1 + D2) — the wave path writes no run log and skips `output_verify::post_apply_failure`. One change, one file, but it needs `execute_wave_io`'s return tuple (src/core/executor/machine_wave.rs:11) to carry the generated script, which today is built and dropped inside the thread at machine_wave.rs:27. That ripples into machine.rs. It is also a CORRECTNESS defect (measured live during this work: the same config gives […2714 chars elided]

**Hostile critique — fatal:** (none)

### [observability] forjar#390 — the failure message is the bug: two streams, three diagnoses, one `exit code 1:` line

IN SCOPE (this fix)

The core defect — the operator-visible failure string is built from stderr alone, at two duplicated sites (src/core/executor/resource_ops.rs:411 and src/core/executor/machine_wave.rs:180). Both are replaced by ONE call into a new module, which is itself half the fix: two copies of a diagnostic are two diagnostics, already free to drift.

D5 (VerbosityLevel is dead code promising exactly the missing feature) — IN. This is not an adjacent defect, it is the same defect wearing a different hat. The enum's own doc says `-vvv: Stream raw stdout/stderr to terminal in real-time`, src/cli/dispatch.rs:41 throws clap's count away on the router's first statement, and the reporter had a documented escape hatch that silently did nothing (measured: -v/-vv/-vvv are 1046 bytes each, byte-identical). Fixing the message while leaving a manual that lies about how to see more leaves the next reporter exactly where this one was. `set_verbosity` is wired in dispatch, the enum's doc is corrected to what forjar can honestly deliver today, and `streams_raw()` gets its first non-test consumer.

D1 (parallel wave writes NO run log) — IN, but only the capture half. The whole remedy rests on "the console shows a window, the run log has the rest", and under `--parallel` there IS no rest: measured A/B on one config with only `policy.parallel_resources` flipped, sequential wrote 8 files including a full `=== STDOUT ===` section, parallel wrote no `runs/` directory at all. A fix whose "full, untruncated output: <path>" line is a lie half the time is not a fix. The cost is small and […3457 chars elided]

**Hostile critique — fatal:** UNBOUNDED BLOBS NOW REACH state.lock.yaml — the fix violates the prompt's explicit blast-radius constraint and makes the situation strictly worse than 1.23.1. Only ONE of the four strings the fix hands to `record_failure` is capped (`short_error`, the `Ok(out)` non-zero path). The other three are uncapped and pass straight through: (a) `report_verify` returns `format!("{verdict} [full output: {}]", …)` with `verdict` produced verbatim by `src/core/executor/output_verify.rs:179-184` — `format!("apply exited 0 but the host does not report the declared state (check exit {}). {}", out.exit_code, out.stdout.trim())` — an UNCAPPED stdout blob; and by `check_post_hook` at `src/core/executor/output_verify.rs:111-115`, an uncapped `pout.stderr.trim()`. (b) `report_transport` returns `format!("transport error: {err}")`, uncapped. (c) The sequential pre_apply failure at […1990 chars elided]

### [contract] FJ-390: a failure report may not be silent about a stream that has bytes in it — one choke point, one bound, one ratchet

THE INVARIANT (this is what forjar violated, stated so it can be tested):

  For every execution that did not succeed, the operator-visible failure report is a
  TOTAL function of (exit_code, stdout, stderr). It may be SHORTER than the output —
  nobody reads 3 MB — but it may not be SILENT about a stream that has bytes in it:

      stdout.trim() != ""            =>  report contains a non-empty prefix of stdout
      stderr.trim() != ""            =>  report contains a non-empty prefix of stderr
      both empty                     =>  the report says so, in words
      |report| <= 2*STREAM_BUDGET + framing                       (bounded, always)
      |constructors of such a report| == 1                        (no second copy)

forjar violated clause 1 structurally: `format!("exit code {}: {}", out.exit_code, out.stderr.trim())`
never mentions `out.stdout`. It violated clause 4 (stderr was UNBOUNDED into events.jsonl —
a 3.4 MB stderr was measured crossing the transport intact). And it violated clause 5 twice
over: the same expression is duplicated at resource_ops.rs:411 and machine_wave.rs:180, the
hook form is triplicated at output_verify.rs:94-98, output_verify.rs:111-115 and
machine_wave.rs:84-88, and the transport form is duplicated at resource_ops.rs:435 and
machine_wave.rs:202.

IN SCOPE — the invariant, plus exactly what is required to make the new message TRUE:

  * The choke point. New `src/core/executor/failure_report.rs`. Six duplicated message
    sites collapse into `streams()` / `exec_failure()` / `hook_failure()` / `hook_error()` /
 […4018 chars elided]

**Hostile critique — fatal:** MISSING HUNK — the crate does not compile as written. `src/core/executor/machine_wave.rs:121` declares `exec_results: Vec<(usize, f64, Result<transport::ExecOutput, String>)>` on `record_wave_outcomes`. The proposal changes `execute_wave_io`'s return type to `Vec<WaveExecution>` (its machine_wave hunk) and rewrites the loop body as `for exec in exec_results` (its second machine_wave hunk, whose quoted current_code starts at line 139), but NO hunk in `changes` touches line 121. So `machine_b.rs:181` binds a `Vec<WaveExecution>` and `machine_b.rs:188` passes it into a parameter still typed as a Vec of tuples, and the loop destructures a tuple type as a struct. Two type errors. It is a one-line fix, but the proposal presents its hunk list as complete and its machine_wave line-count delta (237 -> ~302) as itemised, and neither accounts for it.

### [parallel-gap] Refs #390 — one verdict site for both schedulers: print the task's stdout, and stop `--parallel` from silently passing

IN SCOPE (one PR, three commits): the console fix + D1 (parallel writes no run log) + D2 (parallel skips post_apply_failure) + D6 (transport error leaves no log), plus two free correctness wins found at the sites being replaced.

WHY D1 IS NOT ADJACENT — IT IS LOAD-BEARING FOR THE FIX ITSELF.
The console fix's own new affordance is a pointer: `full output: state/gx10/runs/r-<id>/llama-cpp-build.create.log`. On the parallel path that file does not exist — `capture_exec_output` has exactly ONE call site, src/core/executor/resource_ops.rs:357, and `grep -c capture_exec_output src/core/executor/machine_wave.rs` returns 0 across all 237 lines. Measured A/B on one config with only `policy.parallel_resources` flipped: `true` produced no `runs/` directory at all; `false` produced `probe_a.create.log` and `probe_b.create.log` with full `=== STDOUT ===` sections. So shipping the console fix alone means forjar prints a path to a file it never wrote. A fix that lies about where the evidence is, to the very reporter who said "I could not find the diagnostics anywhere in the full raw apply log", is worse than the silence it replaces. D1 must land in the same commit as the pointer.

WHY D2 CANNOT WAIT FOR A LATER RELEASE — THE FIX ITSELF WIDENS THE HOLE.
D2 is not needed to make the console fix correct. It is needed because the console fix changes operator behaviour in exactly the direction that makes D2 dangerous. Today `--parallel` is unattractive for debugging: nobody reaches for it while chasing a failure, because the failure output is equally useless either way. After this PR […4437 chars elided]

**Hostile critique — fatal:** THE HEADLINE DELIVERABLE IS IMPOSSIBLE UNDER THE PROPOSAL'S OWN CAPS — measured, not argued. I compiled the proposed failure_text.rs verbatim (probe at <SCRATCH>/<SESSION>/scratchpad/probe/ft.rs) and fed it the exact shape output_shape advertises: 1284 stdout lines totalling ~4.1 MB. The real header is `  stdout (3 of 1284 lines, 8192 of 4102553 bytes):` — THREE lines, not the `40 of 1284 lines` the proposal prints. tail() applies TAIL_LINES=40 and then TAIL_BYTES=8192 to the joined result, so on a multi-MB transcript the 8 KiB cut eats almost all 40 lines. Worse, the point fails even with short lines: the reporter's echo, `nvcc --version` and `grep GGML_CUDA` run at the TOP of the script, before cmake. A tail-only 40-line window structurally cannot show them in a 1284-line build log. caps_and_truncation concedes this in writing […1710 chars elided]
