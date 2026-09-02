# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

**`apply --canary-machine` converged the whole fleet past the operator gate, with a
`--yes` nobody typed (#374).**

`check_operator_auth` was the first line of `apply_execute` — the *last* stage of the
apply dispatcher — and every early exit returns above it. So with
`allowed_operators: [alice]` on two machines:

```text
forjar apply --operator mallory --yes                      -> not authorized  EXIT=1
forjar apply --canary-machine sandbox --operator mallory   -> 2 machines converged, EXIT=0
forjar apply --refresh-only --operator mallory             -> every lock rewritten, EXIT=0
```

`--pre-script` had the same shape: `apply_pre_checks` ran the operator's script and
*then* refused. #370 patched exactly one of these exits (`--plan-file`) at its own call
site; a gate each exit has to remember is not a gate, so the check now runs in
`dispatch_apply_cmd` before any exit, hook or backup.

Second, independent defect in the same command: `cmd_apply_canary_machine` hard-coded
`yes = true` into both legs. A flag whose whole promise is "one machine first, so you
can look" rolled the remaining fleet out with no confirmation prompt — for *authorized*
operators too, needing no misconfiguration at all. `--yes` is now threaded from the
command line and each leg asks in turn.

### Changed

**The read-only `apply` modes stay ungated, deliberately (#374).** `--check`,
`--diff-only`, `--output-scripts` and `--dry-run-{graph,cost,verbose}` change nothing
and print what the ungated `forjar check` / `plan` / `graph` verbs already print to
anyone — none of which accepts `--operator`. Gating them buys no confidentiality and
costs a real refusal: `check_operator_auth` iterates *every* machine regardless of
`--machine`, so an operator listed on one machine would lose `apply -m theirs --check`.
This is the same line #370 drew when it left `plan --out` ungated, and both directions
are pinned by tests in `tests/falsification_canary_apply_is_authorized.rs`.

A read is only a read if the *invocation* is. `--check`, `--diff-only` and
`--output-scripts` exit from `apply_mode_exits`, which sits *below* `apply_pre_checks`,
so `apply --check --pre-script deploy.sh --operator mallory` used to run `deploy.sh`
and then print check results with no refusal anywhere. An invocation carrying
`--pre-script`, `--pre-flight` or `--webhook-before` is therefore gated in every mode.

## [1.24.0] — 2026-09-01

### Added

**A pre-PR quorum gate — evidence for a claim, enforced (#390).**
`docs/specifications/quorum-spec.md` + `scripts/quorum-gate.sh` +
`scripts/quorum_evidence.py`. A branch cannot be pushed until a diff-bound receipt
shows its claims survived four mandatory lanes: CRUX (competitive survey, ≥3 named
systems), adversarial refutation, an independent `agy /teamwork` review, and a pmat
MCP pass that must include `analyze_vacuous_tests` by name.

The motivation is #390 itself: a reporter, a maintainer, and a merged fix all
operated for days on a false story while every test stayed green. CI checks the
code; nothing checked the story.

The receipt is bound to the diff via `git hash-object`, so it cannot be recycled or
outrun by an edit, and the evidence must be **committed** — a working-tree-only
digest passes locally and fails in CI, the worst failure mode a pre-push gate can
have. The check with teeth is citation anchoring: ≥33% of adjudicated claims must
cite a `path.rs:N` that resolves **at the merge-base**, the one tree the pusher did
not author.

`.quorum/enforce.json` scopes who is *blocked*; everyone else gets advisory mode —
all checks run and all findings print, but the push is never refused. A contributor
without an agent stack, or out of model credits, uses `QUORUM_SKIP="reason"`. That
skip is refused for an enforced author, who gets a committed `waived.reason`
instead: visible in the PR diff, unsettable from the environment. Bypass exists;
silent bypass does not.

Hardened by its own methodology. An independent review of the first draft returned
REDESIGN and found four bypasses, all verified before being fixed: `QUORUM_BASE=HEAD`
emptied the diff, a local branch named `main` took the exemption while pushing a
feature ref, `sha256sum` would exit 127 on macOS under `set -e`, and the
falsification could name any pre-existing green test. It also argued the kill rule
down from majority-vote to *any un-countered substantive objection* — IETF rough
consensus, and because LLM refuters do not fail independently.

Honest limit, stated in the spec: it proves scrubbed, diff-cited, unrecycled prose
exists and matches the tallies. It does **not** prove a quorum happened.

### Fixed

**`completion_check` was never folded into `hash_desired_state` (#391).** Editing
only the check left the lock hash unchanged, so a lock entry sealed against the OLD
check compared equal to the NEW one and `plan` reported `NoOp` over a resource whose
declared condition had genuinely changed. Same defect class as FJ-035's
`overlay_hosts` fix. This is a real gap and a different half of #390 — it could not
fix the reported symptom, because that defect was never in the hash.


**A failed task's STDOUT never reached the operator, and the failure was named
wrong (#390).** An operator building llama.cpp with CUDA on `gx10` ran
`forjar apply` six times, editing `command:` between runs to add `echo`,
`nvcc --version` and `grep GGML_CUDA` diagnostics. Not one of them ever
appeared. The two `CMake Warning` lines appeared every time, byte-identical.
They concluded — reasonably, and wrongly — that forjar was replaying a cached
transcript, and filed a caching defect.

Nothing was cached. Seven independent reproduction lanes, one over a real SSH
transport, proved with append-only counter files that the command re-ran on
every single apply. The whole symptom was stream routing. The operator's only
failure line was built as

```rust
format!("exit code {}: {}", out.exit_code, out.stderr.trim())
```

in `resource_ops.rs` and, duplicated, in `machine_wave.rs`. `out.stdout` is not
truncated there — it is absent from the expression. `echo`, `nvcc --version`
and `grep` write to STDOUT; cmake's warnings and llama.cpp's own bare
`message("CMAKE_BUILD_TYPE=...")` (CMake NOTICE mode) write to STDERR. So which
lines survived was decided purely by stream, and the message could not change
no matter what was edited — it is a pure function of the exit code and stderr.
"Byte-identical across six runs" was forced by construction, not by a cache.

The headline also named the wrong failure: the command exited 0 every time.
What exited 1 was the `completion_check` GH-254 re-asserts at the end of the
generated script. Six builds were spent hunting a compiler error that never
happened, under a line reading `exit code 1`.

Every failure message on the apply path is now built in one place,
`core::executor::failure_text`, which:

- prints BOTH streams, labelled, with the true byte count, excerpted head AND
  tail (~2 KB per stream). Head as well as tail is load-bearing: #390's
  diagnostics ran *before* the build, so a conventional tail-only window would
  have hidden them a seventh time. This is also the first ceiling this string
  has ever had — it previously went unbounded into an append-only event log.
- distinguishes `NOT CONVERGED` from `FAILED` and quotes the resolved
  `completion_check` verbatim, so the thing that is actually false is named.
- names the absolute path of the run log and the `forjar logs` command that
  renders it — but only when the writer reports it actually wrote one — and
  warns when `--state-dir` is relative (the default), which is how a stateless
  CI runner deletes the evidence with the checkout.
- refuses to claim "the command itself exited 0" for a task declaring
  `timeout:` or `sudo:`, whose nested `bash` does not inherit
  `set -euo pipefail` (tracked as #390-E).

This also closes the mirror-image defect: `output_verify::verify_against_host`
reported stdout and DESTROYED stderr — the branch every `type: task` with no
`completion_check` reaches. Five constructors across the executor disagreed
about which half of a failure to discard; a ratchet test now holds each file to
a budget so a sixth cannot be added quietly. Run logs also stop recording
`type: unknown` for every resource.

Follow-ups filed rather than folded in: #390-A (the parallel wave path writes no
run log at all), #390-B (that path also skips post-apply verification),
#390-C (`--json` still reports `error: null` for a failed resource),
#390-E (the nested-shell `set -euo pipefail` hole).

## [1.23.1] — 2026-08-30

Two fixes shipped as a patch because the first one is the difference between
a CI lane that measures a fleet and one that measures nothing.

### Fixed

**`forjar drift` refused to run without a state dir, so a CI lane could never
measure anything (#385).** `state/` is gitignored in paiml/infra — the lock
lives on whichever box ran `apply` — so every checkout of the fleet's nightly
drift lane hit this, and had since the lane was written:

```
FAIL gx10         forjar drift exited 1: error: cannot read state dir .../infra/state
drift-tripwire: 0 of 2 requested machine(s) measured
FAIL: no machine was measured — this run measured NOTHING
```

#380's own reasoning says why the refusal was wrong. For a `type: task` the
observable is an ASSERTION, not a baseline: a `completion_check` that fails
right now is drift whether or not anything was ever written down about it. A
run with no lock can still execute every task check and give a TRUE answer
about the host — it simply cannot hash-compare `File`/`Image` resources, which
is a *smaller* answer, not an invalid one.

An **absent** state dir now walks the config instead of the lock, runs the
assertions, and reports the census with the remainder attributed to
`no lock (never applied from here)`:

```
Checking gx10 (no lock — assertions only)...
  inspected 1 of 2 resource(s) in scope: task 1
  skipped 1: no lock (never applied from here) 1
  DRIFTED: runner-scope-and-labels (completion_check fails on gx10: task=pending)
```

`--tripwire` exits on the findings rather than on the missing directory, and
`--json` carries the same census (`skipped_by_reason`), which is the surface
the infra lane parses. `--dry-run` previews the same population through the
same predicate, so the preview cannot promise work the run will not do.

Two neighbouring faults deliberately stay fatal, because collapsing them into
"never applied from here" would be the same reported-not-measured defect in a
new place:

- a state path that is **present** and unreadable — wrong mode, not a
  directory, a dead mount — still exits 1 with `cannot read state dir`;
- **no state dir and no config** leaves nothing to assert and nothing to
  compare, and is refused rather than answered.

**The `Coverage` lane cached a 70 GiB tree onto a runner that size, and
`actions/cache` called the resulting ENOSPC a warning (#386).** `cargo
llvm-cov` here builds 242 integration test binaries plus the lib and bin
unit-test binaries, all instrumented, all carrying full DWARF — measured at
70.70 GiB in 19,070 files. The lane then asked `actions/cache` to write a
second, compressed copy of that tree onto the same filesystem:

```
zstd: error 70 : Write error : cannot write block : No space left on device
##[warning]Failed to save: "/usr/bin/tar" failed with error: ... exit code 2
```

A failed SAVE is downgraded to a warning, so every *green* run of this lane had
already filled the runner's disk and said so where nobody reads — and since the
save never completed, the cache never existed: consecutive runs on an identical
key both reported `Cache not found for input keys`. Eventually the runner's own
Worker process died writing its own diag log, which is why the failing run had
no logs at all. The `target` cache is gone from hosted jobs, and
`tests/falsification_hosted_jobs_do_not_cache_target.rs` fails if it comes back.

## [1.23.0] — 2026-08-30

Three defects, each of which cost a machine or an artifact: drift could not see
the guards a fleet declares, forjar could not replace its own running binary,
and the release lane shipped archives from previous tags.

### Fixed

**`apply --plan-file` ignored `--dry-run` and `-m`.** `cmd_apply_from_plan` took
neither argument, and the `ApplyConfig` it built hard-coded `dry_run: false,
machine_filter: None`, so:

```
$ forjar apply -f forjar.yaml --plan-file p.json --dry-run
Plan applied: 1 converged, 0 unchanged, 0 failed
$ echo $?; test -f alpha.txt && echo CREATED
0
CREATED
```

A two-phase plan/review/apply feature whose `--dry-run` converges the machine
instead of previewing it is the worst available default. `--dry-run` now prints
the reviewed plan and applies nothing.

`-m` **intersects** the reviewed scope — a selector on `--plan-file` may only
narrow the reviewed delta, never widen it — and an EMPTY intersection is now an
error naming what the plan does cover, rather than an apply of nothing that
exits 0. The other three selectors join it below.

**A re-sealed plan could still claim "no changes", and the first fix closed
only the empty case.** The seal is an unkeyed BLAKE3 hash, so anyone who can run
`forjar` can compute one: copy `config_hash` and `state_hash` verbatim out of an
honest plan (neither leg moves), rewrite the body, recompute the diff leg and
the composition through the public `plan_seal::digest` API, and `apply
--plan-file` printed `Plan has no changes to apply.` and exited 0 with a create
still pending.

The 1.20.1-era claim that `check_body_partition` "still refuses a
zero-the-counters edit whose author ALSO recomputed the seal" was **false** and
has been deleted from the module docs. `0/0/0/0` over an EMPTY change list
partitions perfectly well; that check catches a plan claiming zero *while
listing several*, and the attack empties the list.

The repair for that — `apply --plan-file` re-plans and compares — was right, but
the clause covering an empty body keyed off `plan.changes.is_empty()`, a
syntactic accident. What decides whether anything executes is
`PlanScope::from_plan`, which skips `NoOp`. So on a PARTIALLY converged stack
— every real deployment — the same attack works without emptying anything:
delete the one pending line and keep an honest `no_op` line beside it. Counters
still partition (0/0/0/1), the list is not empty, the scope is:

```
$ forjar apply --plan-file forged_delete.json --yes
Plan has no changes to apply.
exit=0                                    # alpha STILL PENDING
```

The obvious repair — use `scope.is_empty()`, the predicate three lines below —
is **wrong**, and the reason is the interesting part of this fix. `forjar plan
-r bravo --out` over an already-converged `bravo` writes `changes: [bravo
no_op]`, counters `0/0/0/1`, empty scope: the SAME DOCUMENT, byte for byte.
That plan applies cleanly today and must keep doing so, or every idempotent CI
loop over a filtered plan starts failing. No predicate over the document can
separate a narrow plan from an edited one, because the format could not say
which it was.

So the format says it now. `forjar-plan-v2` carries a `selectors` record —
the `-m`/`-r`/`-t`/`-g` the plan was written under — sealed into the diff leg,
and `apply --plan-file` re-plans **through those selectors** and requires
agreement in BOTH directions:

- every pair the body NAMES must carry the action the planner gives it (catches
  relabelling a create as `no_op`);
- every non-`NoOp` change the planner PRODUCES must be named by the body
  (catches deleting the line instead, and catches emptying the list).

Two directions, one predicate, and no special case for emptiness — which is what
the two evaded checks both were. `plan` and `apply --plan-file` now compute
their plans through ONE function (`cli::plan_compute::plan_filtered`), because a
second spelling of "the planner plus the four selectors" would make the
comparison fire on plans nobody edited.

A forger can still re-seal a document that DECLARES itself narrow, and the
planner will honestly agree with it. What they can no longer do is that
invisibly: the claim has to be in the file, and an empty-scope apply of a
filtered plan now prints the work it is not doing —

```
Plan has no changes to apply.
note: this plan is filtered (-r bravo) and asks for nothing. 1 change(s)
      OUTSIDE its filter are still pending: alpha on web (CREATE).
```

The seal remains what `core::plan_seal` always said it was — integrity, not
authentication. Two-phase *authorization* would need a keyed hash or
`cli::pq_signing`, and is a different feature.

**A `forjar-plan-v1` document can no longer be narrow.** v1 has no `selectors`
record and never will, so "written with `-r alpha`" and "someone deleted the
bravo line" are the same document — and a v1 forgery needs no skill at all,
since there is no seal to recompute. Exempting v1 from the completeness check
would have left the whole defect open behind a one-word `"format"` edit, so v1
gets the strict reading: its body must name every change the planner finds
pending. The remedy is one `forjar plan --out`, which writes v2.

**`apply --plan-file` silently dropped every apply flag except `-m`.** The
`ApplyConfig` it built took exactly one field from the invocation; the other
fifteen were hard-coded:

```
$ forjar apply -f forjar.yaml --plan-file p.json --yes -r alpha
Plan applied: 2 converged, 0 unchanged, 0 failed      # bravo converged too
```

and the same for `-t`, `-g`, `--progress`, `--force`, `--timeout`, `--retry`,
`--parallel`, `--max-parallel`, `--resource-timeout`, `--rollback-on-failure`,
`--trace`, `--refresh`, `--force-unlock` and `--force-tag`. An operator who
believed `--rollback-on-failure` was armed on a plan apply was wrong, silently.
Worse, the doc comment written to fix an earlier FALSE-comment defect supplied a
rationale for it — that the selectors "were already applied when the plan body
was written". True of how the plan was produced; no reason at all to ignore a
flag the operator is passing NOW.

Each flag is now decided rather than dropped:

- **Selectors** — `-m`, `-r`, `-t`, `-g` INTERSECT the reviewed scope. A
  selector may only narrow a reviewed delta, never widen it, and the executor
  already intersects all four with the scope. An EMPTY intersection is an error
  naming what the plan covers, because converging nothing at exit 0 is the
  silent green this whole issue is about. `--dry-run` previews the same
  narrowed set the real run would converge.
- **Knobs** — `--progress`, `--timeout`, `--retry`, `--parallel`,
  `--max-parallel`, `--resource-timeout`, `--rollback-on-failure`,
  `--force-unlock` and `--trace` say HOW the reviewed delta executes and are
  passed straight through. There was never a reason not to.
- **Re-planners** — `--force`, `--force-tag` and `--refresh` are now REFUSED
  with `--plan-file`. They clear the lock entries the planner reads, so they
  change what the delta IS: `--force-tag`/`--refresh` can make a resource
  reviewed as `update` execute as `create`, and `--force` defeats the scope
  outright, because `PlanScope` demotes out-of-scope changes to `NoOp` so
  `triggers` still fire and `should_skip_single` skips a `NoOp` only
  `if !cfg.force`. Refusing costs one re-run; ignoring cost the belief that a
  reviewed plan executed.

GH-208 rides along: the plan path was handed `args.dry_run` alone, so
`--plan-file --dry-run-json` converged for real. It takes the whole dry-run
family now.
### Added

**One quality gate, in core, shared by `forjar lint`, `forjar_lint` and the
pre-apply check.** (Refs #356)

`core::quality_gate::evaluate` runs four checks — bashrs over the generated
shell, plaintext secrets in the config and in the scripts, an opt-in cyclomatic
ceiling, and compliance packs plus in-config `policies:` — and returns one
verdict that every surface renders. `forjar lint` gains `--sarif`,
`--policy-dir` and `--max-cyclomatic`; `forjar_lint` gains `gate_passed`,
`error_code`, `findings[]` and a SARIF 2.1.0 `sarif` object, all additive.

A value sealed as `ENC[age,<ciphertext>]` is ciphertext, not a plaintext
secret, and is not reported — the discrimination that separates this from a
grep for the word `password`.

SARIF results now carry `region.startLine` for the resource's declaration when
that key is in the file that was linted, and the artifact uri is the real path
rather than the literal `forjar.yaml`. `parser::policy_check_to_sarif` is a
projection onto the same emitter, so there is one SARIF emitter in the tree
instead of two.

### Changed

**`forjar lint` and `forjar_lint` gave different answers for the same file.**
(Refs #356)

`cli/lint.rs` dropped every `SC1*` diagnostic and every line inside a heredoc
body; `mcp/handlers.rs` dropped neither, and listed advisory diagnostics the
CLI only tallied. Same verb, two answers, and `tests_parity.rs` never compared
lint. Both now route through `core::quality_gate` and render identically:

- the `SC1*` exclusion is gone. Its comment cited false positives that
  `core::purifier` removed in forjar#285 after linting 1,311 generated scripts
  and measuring ZERO SC1 hits. SC1 is the syntax-error family.
- diagnostics below `Severity::Warning` are dropped on BOTH surfaces. They are
  style advice about shell forjar emits, not shell the operator wrote.
- the gate lints `transport::strip_data_payloads(script)` — the exact text the
  executor validates — so it can no longer refuse an apply over a base64 blob
  or a `content:` heredoc the transport runs without comment.
- lint findings are now rendered as `FJQ-<rule> <resource>[/<phase>]: <message>`.

**`forjar apply --policy-check` widened from compliance packs to the whole
gate.** (Refs #356) It evaluated packs and nothing else, so a config could ship
a plaintext password, or emit shell bashrs rejects outright, and still apply.
The refusal now names `FORJAR_QUALITY_GATE_VIOLATION` and lists every blocking
finding. `validate` and `plan` are deliberately NOT gated: both are read-only
and answer questions, and a gate in front of a question takes away the
operator's route to a fix.

**`lint --policy-dir` runs shell, so it is a CLI flag and NOT a verb
parameter.** (Refs #356) A compliance pack rule of `type: script` is evaluated
by `sh -c`, so pointing the gate at a pack directory executes what the pack
author wrote — measured, not inferred: a pack whose script is `touch <path>`
creates that file under `forjar verb call lint`.

An earlier revision of this entry kept the field on every surface and justified
it by saying `Effects::ReadOnly` meant read-only *with respect to the fleet*,
citing 1.21.0's `ambient_inputs`. That reading is withdrawn. `ReadOnly` means
the invocation writes nothing anywhere — the machine running it included — and
runs nothing somebody else chose, which is the conclusion 1.21.1 reached
independently from the config's side (forjar#372). `policy_dir` is gone from
`mcp::types::LintInput`; `--policy-dir` survives on `forjar lint` and
`forjar apply --policy-check`, where an operator typed a flag whose help text
says it runs shell. Opting in and reading a schema are not the same act.

It is off by default on every surface; in-config `policies:` are declarative
and never execute anything.

`compliance_pack::RuleEvalResult` gained a `severity` field. The gate needs it
to level a per-rule finding, and `compliance_gate::count_severity_failures` was
recovering it by searching `pack.rules` for a matching id — answering "warning"
for any result whose id was not found, so a pack whose ids drifted counted zero
errors and passed the gate.

- **`forjar drift` was blind to task guards, and never said what it had
  inspected** (#380, paiml/infra#380). A `type: task` with a `completion_check`
  is an assertion about the host — the check is the claim, `command` reports the
  violation — and drift only consulted it when the lock happened to carry an
  observed digest, as a hash comparison whose failure printed
  `state query failed:` with an empty message. Where the lock recorded
  convergence but no observation (`state reconstruct`, an apply whose post-apply
  state query failed or timed out), the assertion was never executed at all.
  forjar's own dogfood ledger has carried this since 1.12.3 as
  `drift-and-plan-blind-to-failing-task-completion-check`.

  Drift now executes each converged task's `completion_check` over the same
  transport `apply` uses, under the same 60s bound, and a non-zero exit is
  drift regardless of what the lock recorded. `--no-task-checks` opts out per
  run.

- **`No drift detected.` read identically over sixty-two resources and over
  none.** Every drift run now prints its denominator — inspected versus skipped,
  by type and by reason — in text and in `--json` (`resources_inspected`,
  `resources_skipped`, and a per-machine `census`). Measured on paiml/infra's
  gx10: 62 resources declared, 30 in the lock, and the runner guard that
  prompted the issue in neither number the operator saw.


- **`cp` could not replace a binary that was RUNNING, and `install(1)` left a
  window where it did not exist** (PMAT-136, paiml/infra#386). `cp` opens the
  destination in place, so the kernel refuses it for a file being executed
  (ETXTBSY) and coreutils refuses it for a dangling symlink — the two states
  forjar most needs to repair. Three code paths placed executables with `cp`.
  The cost was a machine: paiml/infra's lambda-labs left forjar undeclared on
  the strength of that failure and drifted to 1.20.1 while the fleet ran 1.21.x,
  which made its own YAML guard NO-GO over 200 files.

  `install(1)` clears both refusals but unlinks then creates: measured at 10611
  absent observations in 396132 stats (2.7%), against 0 in 741725 for
  temp+rename. On a host where 16 runners share one `$CARGO_HOME/bin` that is a
  live ENOENT hazard. All three sites now stage a sibling and `mv -f` —
  `rename(2)` neither opens the destination nor follows it. The
  `cp: not writing through dangling symlink` failure shares the cause and is
  retired by the same change.

- **The release staging directory was a fixed path on runners that are not
  ephemeral** (#324). `/tmp/release-staging` persists between jobs on
  `[self-hosted, clean-room]`, so each release's tarballs were re-uploaded by
  the NEXT one: v1.21.0 staged 6 archives, v1.21.1 10, v1.22.0 14 — exactly +4
  per release, the four self-hosted Linux legs. #324 diagnosed this class and
  fixed only the download side.

  It is also cross-LEG: all four Linux legs share one `/tmp` on one box, and
  each leg's upload step reported 9 and 11 files while packaging exactly one.
  `merge-multiple: true` deduplicates by NAME, which is why v1.21.0 passed while
  already contaminated — only version-stale files changed the union. Every Linux
  artifact has been shipping sibling architectures' archives. Now `RUNNER_TEMP`,
  which is per-job and runner-cleaned; adding `rm -rf` to the shared path would
  have turned a duplicative race into a destructive one.

## [1.22.0] — 2026-08-29

Epic #356: the unified verb surface grows from nine verbs to twelve, the quality
gate becomes one calculation instead of three, and a plan file becomes a sealed
artifact rather than a suggestion. Everything here was built against the real
binary and every fix ships with a test that was verified to fail before it.

### Added

- **`forjar_remediate`** (#356) — a verb that proposes corrections to a config
  and writes nothing. It earns `Effects::ReadOnly` the hard way: the fix set is
  returned as data, and applying it is the caller's separate, explicit act.
- **`forjar_audit` and `forjar_workspace`** (#356). Both existed as CLI leaves
  whose results could only be printed; the provenance trail and the workspace
  identity list are now structured output an agent can consume.
- **Plan sealing — `forjar-plan-v2`** (#356, #358). A plan is now bound to the
  state it was computed against AND to its own body, so a hand-edited plan file
  is rejected rather than silently honoured. The seal covers the `selectors`
  that produced it, which closes the case below.
- **A TOTAL partition of the CLI surface** (`src/verb/partition.rs`). All 193
  CLI leaves are classified `Unified`, `CliOnly(reason)`, or `Pending(issue)`,
  and a leaf that names no bucket **fails the build**. The test walks the live
  clap tree, so a new command cannot quietly skip the question of whether it
  belongs on the verb surface.

### Fixed

- **A `ReadOnly` verb ran shell, and the gate that should have caught it was
  blind** (#356). `forjar_lint --policy-dir` reached `sh -c`. Found by driving
  real MCP stdio rather than by reading the code — a `policy_dir` pointing at a
  directory of policies created a sentinel file on disk through a verb
  published as read-only.
- **`--plan-file` executed the whole config, not the plan** (#356, #358). The
  flag read the plan for its metadata and then re-planned from scratch, so the
  resources actually applied were whatever the config said at apply time, not
  what the operator reviewed and approved.
- **An unsealed v1 plan chose the filters it was checked under** (#358). A plan
  produced with `-r bravo` could be applied without the selector, widening the
  blast radius past what was reviewed. The v2 seal now covers selectors. The
  first proposed fix here — rejecting an empty scope — was measured and
  *withdrawn*: `forjar plan -r bravo --out` over an already-converged resource
  legitimately yields an empty scope, so that rule would have broken every
  idempotent CI loop over a filtered plan.
- **`--plan-file` ignored `--dry-run`** and dropped every flag but `-m` (#358).
- **`forjar lint --fix` deleted every comment in the config it was fixing**
  (#359). The fixer round-tripped through a parse that does not preserve
  comments, so running it cost the user their annotations.
- **The same verb gave two answers depending on the transport** (#356). The
  quality gate was three separate calculations (CLI, verb, pre-apply); it is now
  one implementation in core, so the surfaces cannot disagree.
- **`policy-coverage` was withdrawn from the verb surface** (#369). The unified
  answer was wrong, and shipping a wrong number under a stable name is worse
  than shipping no verb. `forjar policy-coverage` remains a CLI command.

- **`forjar undo` did not undo** (#376). It rolled the lock back to the target
  generation and then re-applied the CURRENT config, which immediately
  re-converged the host to the state it had just walked away from. Three applies
  of `v1 → v2 → v3` followed by `undo --yes` exited 0, printed `1 converged`,
  and left the file holding `v3`. It could not have worked: a generation stored
  only a BLAKE3 hash of the config, never its body, so undo had nothing to
  replay. Present since at least 1.20.1.

  A generation is now recorded AFTER the apply it describes, carrying the
  EXPANDED config beside the lock — expanded because `includes:` bodies and
  `-p` overrides are resolved in the config *value* and absent from the file, so
  recording the raw file left both to be re-read live at replay and converged
  the host forwards again. `-p ver=v3` then undone used to land the host on the
  param's DEFAULT: bytes no generation ever held.

  Where faithful replay is impossible it **refuses** rather than guessing. A
  `file` resource with `source:` takes its bytes from a path the generation
  never captured, so replaying an old config against a newer payload would
  converge forward while stamping the lock with the old hash — after which
  `drift` reports clean over the wrong bytes. Undo now exits non-zero, names the
  resource, and leaves host and lock untouched.

- **`undo` applied the cwd `forjar.yaml` against an unrelated `--state-dir`**
  (#377). `-f` defaults to `forjar.yaml` in the current directory while
  `--state-dir` is separate, so the two could name different stacks and nothing
  noticed: it diffed one stack's generations, applied another's resources, and
  re-stamped the state dir — erasing the evidence they had ever differed. Undo
  now refuses on a `GlobalLock.name` mismatch. It accepts a name the state dir
  has applied under before, read from its own generations, so undoing across a
  legitimate stack rename is never refused.

- **`includes:` silently erased the base config's entire policy block** (#379).
  The merge replaced `policy` wholesale and unconditionally; `Policy` derives
  `Default`, so an include that said nothing about policy handed over a default
  block. Any config using `includes:` therefore lost `tripwire` (drift detection
  quietly off) and `snapshot_generations` (no generation recorded, so `undo` and
  `rollback` were dead) — silently, and without the overwrite warning every
  sibling section already had. Policy is now replaced only by an include that
  declares one.

### Known limitations

- **The running MCP server does not send `readOnlyHint`** (#375). All twelve
  verbs are read-only and `forjar mcp --schema` reports the annotation per tool,
  but `serve()` builds its tool list through a different function, and
  `pforge_config::ToolDef::Native` has no annotations field — so the property
  cannot reach a live client without an upstream change. Prose that claimed
  otherwise has been corrected rather than left to mislead.
- **`forjar apply` authorization is checked below several early-return exits**
  (#370, #374). 1.21.1 closed `--plan-file`; `--canary-machine` still reaches a
  converging apply without passing `check_operator_auth`, and `--refresh-only`
  and `--check` exit above the gate. Tracked with a full ledger in #370.
## [1.21.1] — 2026-08-29

Two defects in 1.21.0 where a promise the call graph does not keep. Both were
found while building epic #356 against the real binary, and both are fixed here
rather than in the next minor because a user could be relying on either today.

### Fixed

- **`forjar_plan` published `readOnlyHint: true` and executed config-declared
  subprocesses** (#372). `src/verb/spec.rs` says `ReadOnly` means "Safe for an
  agent to call unattended" and `src/verb/registry.rs` says an agent "may call
  any forjar verb unattended without risking a change to a machine". Three paths
  made both false, all reachable over `forjar mcp` stdio with nothing but a
  config path:

  | key | reaches |
  |---|---|
  | `ambient_inputs` | `Command::new("bash").arg("-c")` (`core/task/ambient.rs:90`) |
  | `secrets.provider: sops` | `Command::new("sops")` / `op` (`core/resolver/template.rs:56,71`) |
  | `output_equivalence: !command` | `bash -c` (`core/task/output_hash.rs`) |

  Measured against the 1.21.0 binary, one verb per fresh fixture: only
  `forjar_plan` fired them, and it fired all three. **An agent asked to inspect
  an untrusted repository executed whatever that repository declared** — no flag,
  nothing to opt into.

  `core::unattended::sanitize_config` now strips those three keys before the MCP
  path plans, and `plan` stays `Effects::ReadOnly` because it genuinely does not
  change the fleet. The skip is disclosed, not silent: `PlanOutput` gains a TOTAL
  `unattended_skipped` (always present, possibly empty — an absent field and an
  empty list read the same to a careless consumer), and the existing `disclosure`
  prose composes both blind spots.

  The secrets path is fail-closed rather than merely non-executing: without an
  explicit refusal arm, the fallthrough would have read `FORJAR_SECRET_<KEY>` and
  resolved a DIFFERENT value under the same name — a plan computed against a
  secret nobody configured, reported as successful.

  **The CLI is unchanged.** `forjar plan` still probes, still runs
  `ambient_inputs`, still shells out to sops. That capability (#244) is why
  `plan` can tell stale from fresh; the defect was the unattended surface
  offering it to a caller who had not asked.

- **`apply --plan-file` never reached `check_operator_auth`** (#370).
  `apply_mode_exits` returns for `--plan-file` before `apply_execute`, whose
  first line is the authorization check. With `allowed_operators: [alice]` and a
  non-alice operator, measured on the 1.21.0 binary:

  ```
  $ forjar apply --yes                                        exit 1  not authorized
  $ forjar plan --out p.json                                  exit 0
  $ forjar apply --plan-file p.json --operator mallory --yes  exit 0  2 converged
  ```

  A plan file is unauthenticated — any user can write one — so the bypass needed
  no privilege and no forgery. The plan path now performs the same authorization
  the ordinary path does.

  Four other modes share the shape (`--check`, `--diff-only`, `--output-scripts`,
  `--refresh-only` also return before `apply_execute`). They are read-shaped and
  are NOT fixed here; the systematic gate-parity treatment is tracked on #370 for
  the next minor, because patching five modes by hand will miss the sixth.


## [1.21.0] — 2026-08-28

### Added

- **A task could not declare an input that is not a path, so an ambient change
  was reported as `unchanged` by every read verb** (#244). `staleness_reason`
  decides entirely from the DECLARED set, and until now the only declarable
  input was a PATH: `probe_resource` built `input_hash` from
  `hash_inputs(task_inputs)` alone, and `Resource` had no field that could
  carry anything else — an ambient input was not merely undetected, it was
  undeclarable. Measured on 1.13.2, a task reading an undeclared
  `ambient/fonts.txt` was applied, only that file was changed, and every read
  verb reported clean:

      plan   -> Plan: 0 to add, 0 to change, 0 to destroy, 1 unchanged.
      check  -> Check: 1 pass, 0 fail, 0 skip
      drift  -> No drift detected.
      apply  -> Apply complete: 0 converged, 1 unchanged.

  while `--force` changed the artifact's bytes. The motivating case is not
  exotic: a rasterizer calling `fontdb.load_system_fonts()`. There is no honest
  glob for the system font set, it changes when somebody runs
  `apt install fonts-*` or the CI runner AMI rolls, and every frame then renders
  with different glyph metrics while forjar reports `N pass, 0 fail`.
  Under-declaration converts "no build system" into "a build system that lies",
  which is harder to detect than having no cache at all.

  `ambient_inputs: [<shell command>]` folds each command's stdout into the SAME
  `input_hash` the probe and the lock already agree on, so `staleness_reason`
  needs no change and plan/check/drift/apply all become correct at once — they
  route through one probe. ONE function, `hash_declared_inputs`, is called by
  the probe AND by `record_io_hashes` AND by the executor's cache-skip; two
  compositions is how you get an eternal "inputs changed" pump, which is worse
  than the bug. With no `ambient_inputs` the hash is byte-identical to
  `hash_inputs`, so upgrading rebuilds nothing. A FAILING fingerprint command
  contributes a failure marker rather than being dropped — dropping it collapses
  the hash back to the file-only value and reports clean over a stale artifact,
  which is this exact bug reintroduced the moment the fingerprint breaks. stdout
  only: stderr routinely carries a pid or a timestamp, and hashing it would
  report "inputs changed" on every plan.

  Two costs, stated rather than hidden. One subprocess per ambient input per
  probe, on every plan/check/drift/apply — a cached fingerprint is a fingerprint
  that lies, so there is no cache. And `plan`, `check` and `drift` become able
  to run a user-declared command: they are read-only with respect to the FLEET,
  not with respect to the machine running them. This remains a DECLARATION and
  detects nothing nobody thought of; the three ways to catch an UNNAMED ambient
  read (fanotify, ptrace, LD_PRELOAD) are out of scope.

- **`apply` printed the same word, `converged`, for a change the operator asked
  for and for drift it silently repaired on the host** (#336). Those are
  different events: the second means something outside forjar modified a managed
  resource — the difference between a deploy and an intrusion, or between a
  deploy and a unit that keeps resetting itself. The information was never
  missing from the process; it was discarded at a function boundary one call
  frame above the printer. `check_pre_apply_drift` computed a `Vec<DriftFinding>`
  per machine, spent each finding on exactly two side effects (an `eprintln!`
  and a `ResourceStatus::Drifted` write) and returned `Result<(), String>`, so
  by the time `cmd_apply_scoped` reached `print_apply_summary` the only
  surviving facts about the run were three integers. Now:

      Apply complete: 3 converged (1 repaired drift), 12 unchanged.
        drift-repaired: [intel] dnsmasq-fleet-hosts — file state changed

  The `--json` half matters more. The `drift:` lines go to STDERR and the report
  to stdout, so `forjar apply --json` gave a machine consumer ZERO drift signal,
  and this fleet's nightly lanes are machine consumers. `summary` gains
  `drift_repaired_count` (always present, so a parser can branch on `> 0`) and
  `drift_repaired[]` with machine, resource and detail.

  The count is intersected with what the run actually converged: the gate leaves
  a resource excluded by `-r` / `--only-machine` / a tag filter, or one that
  failed, as `drifted` in the post-apply lock, and a claimed repair that did not
  happen is worse than silence because the operator then does not go and fix it.
  It counts RESOURCES, not findings — `detect_drift_full` emits one finding per
  observable, so a single tampered file yields both `content changed` and `file
  state changed`. At zero the summary line is byte-identical to 1.20.1's, which
  is load-bearing: two existing falsification tests assert on its exact text.

  Two deliberate blind spots, now written into
  `apply-summary-distinguishability-v1`. Under `--force` the drift gate returns
  early, so `drift_repaired` is always empty — running the detector there would
  add a transport round-trip per resource to the one path that exists to skip
  observation. And `src/cli/observe.rs::run_watch_apply` prints its own
  `Apply complete:` line from a path that never invokes the gate, so
  `--watch --auto-apply` words its summary differently; that divergence
  pre-dates this change.

- **`forjar store verify` — nothing could answer whether a store entry still
  held the bytes it recorded** (#236). Write `GOOD ARTIFACT BYTES` into
  `<entry>/content/out.mp4`, write meta, then overwrite the file with
  `CORRUPTED BYTES!!!!` leaving the recipe and inputs untouched. On 1.20.1
  `read_meta` returns a byte-identical struct, `store_path` returns the same
  address, and `store gc` and `store list` both report the entry present and
  valid. Bit rot, a partial write, an interrupted `atomic_move_to_store` or a
  manual edit were all invisible, because there was no recorded digest to
  compare against. `path::store_path` answers a different question — "has this
  recipe with these inputs already been built?" — and is deliberately untouched,
  since re-addressing it would move every entry already on disk. `StoreMeta` now
  carries `output_hash`, BLAKE3 over the entry's `content/` tree computed after
  the artifact lands, which answers "are these the bytes we produced?"; the new
  verb re-hashes and compares:

      forjar store verify
      forjar store verify --repair

  It exits non-zero on any failure, so it can be a cron or CI gate. Four
  verdicts: `ok`; `MISMATCH`, the bytes are not the recorded bytes; `unsealed`,
  written before schema 1.1 and so carrying no digest to be wrong about —
  reported, never counted as a failure; and `MALFORMED`, no readable `meta.yaml`
  or no `content/`, which IS a failure, because `write_meta` is part of every
  entry's creation and its absence means the entry was never finished.
  `--repair` removes only `MISMATCH` entries so the next build or cache pull
  re-creates them, and never touches an `unsealed` one — deleting data on the
  evidence of an older schema is the failure mode a repair flag has to avoid.
  `--json` carries `verified`, `unsealed`, `failed`, `repaired` and `results[]`.

  **What is sealed is narrower than what can be verified.** `execute_import` is
  the only production `write_meta` call site, so an import is the only path that
  seals an entry today; everything already on disk reports `unsealed` until it
  is rewritten. `cache_exec::verify_pulled_content` still checks a staging tree
  BEFORE the entry exists, so it would need the SENDER's `output_hash` — a
  cache-protocol change, not in this release, and neither is the
  `output_hash -> store_hash` dedup index the field also makes possible.
  `cmd_archive_unpack` writes a `FarManifest` where a `StoreMeta` belongs, so an
  unpacked archive verifies as `MALFORMED`: honest, in that it cannot be
  checked, and not yet right.

### Fixed

- **`FORJAR_BUDGET_DRY_RUN=1` did not prevent deletion; a `disk_budget` apply
  reclaimed ~1.5 TB during what was believed to be a preview (#334).** That
  variable is a variable of the GENERATED REAPER, evaluated on the target at the
  far end of a chain that strips it — `sudo bash <<'FORJAR_SUDO'` resets the
  environment and `ssh host bash` carries no `SendEnv`. forjar's own process
  never read it, so it could neither honour it nor report that it was ignoring
  it, and the reaper's `${FORJAR_BUDGET_DRY_RUN:-0}` fell through to its
  fail-dangerous default of deleting. Four changes:

  - The reaper now previews by default. Deleting requires
    `FORJAR_BUDGET_EXECUTE=1`, granted in exactly two places — the generated
    systemd unit and the pass `forjar apply` runs — mirroring the inversion
    `nas_archive` was given in #284. `FORJAR_BUDGET_DRY_RUN` still works, and
    still wins, so the documented variable stops being a lie.
  - Every pass names its mode (`mode=dry-run` / `mode=execute`) on its start and
    completion lines, the apply-time pass announces EXECUTE before invoking, and
    a preview no longer counts un-freed bytes into `reclaimed_bytes`, rewrites
    the drift-hashed heartbeat, or trips the anti-inertness `exit 1`.
  - `forjar apply` REFUSES when `FORJAR_BUDGET_DRY_RUN` is set and the scope
    holds a `disk_budget`, naming the two previews that do work. An ignored
    request is worse than a rejected one.
  - `forjar codegen --phase reaper` emits the reclaim pass alone — a preview
    that previews. `--phase apply` emits the INSTALLER, which grants the opt-in
    and re-elevates; the recipe documented in the CLI appendix has been
    corrected.

  ONE-TIME RE-CONVERGE: the reaper text changed, so `disk_budget_script_sha` and
  `hash_desired_state` change and every machine with a `disk_budget` re-applies
  exactly once. That is the property FALSIFY-DBG-013 pins, and is expected.

- **`default-features = false` was a no-op** (#237). `Cargo.toml` opened a
  `[features]` table that never defined a `default` key, so Cargo had nothing to
  subtract: `cargo tree --no-default-features` was byte-identical to the default
  tree. A consumer wanting `forjar::api` paid for an MCP server, two TLS stacks,
  a bundled SQLite and a multi-thread tokio runtime. There is now a real
  `default = ["cli"]` with `cli`, `db` and `tls` cuts, `[[bin]] forjar` carries
  `required-features = ["cli"]`, and the library tree drops from 370 crates to
  213. The default build is unchanged — `Cargo.lock` does not move and
  `cargo tree -e normal` on the default features is identical.

  Measured on the release as a whole, not on #237 in isolation: #237 alone cut
  360 to 191, and #228's `ureq` — which the registry transport needs and which
  is not feature-gated, because `src/core/store/registry_*.rs` is not — adds 22
  crates back to the trimmed tree. Both numbers are re-measured here rather than
  carried over from the branch, because a dependency count quoted from before a
  sibling change landed is the kind of figure this changelog exists to not
  publish.
- **CI ran none of the crate's 87 doctests** (#318). sovereign-ci's test job is
  hard-scoped to `cargo test --lib` and, with `use_nextest: true`, is executed by
  cargo-nextest, which cannot run doctests at all; `lockfile` compiles nothing and
  `examples-validate` selects one integration target. An uncompilable doctest was
  therefore invisible until the clean-room release gate, a release cycle later —
  which is exactly what happened to #315. A `doctests` job now runs
  `cargo test --locked --doc` on every PR and is wired into the required `gate`.
- **Nothing ever read a published release object back** (#325). v1.18.0 shipped
  carrying four `forjar-1.17.0-*.tar.gz` assets and a 10-line `SHA256SUMS` for six
  archives; it was repaired by hand and the same defect then reached v1.19.0,
  v1.20.0 and v1.20.1, because `nightly.yml` asserts only that every `v*` tag HAS
  a release, never that the release describes itself. New
  `scripts/release-object-audit.sh` checks four invariants over a release object —
  no assets from another version, a non-zero archive count, `SHA256SUMS` naming
  exactly this release's archives, and a `.sha256` sidecar per archive that agrees
  with `SHA256SUMS` — and a daily `release-audit` workflow runs it over every
  published release. `binary-release.yml`'s `checksums` job gained the
  version guard #324 added to the other producer, and now refreshes the sidecars
  it writes instead of leaving them naming clobbered bytes.

- **Three `.rs` files under `src/` were compiled by nothing, and one was
  compiled twice** (#292). rustc compiles only what a `mod`, a `#[path]` or an
  `include!` names, and nothing in this build or this test suite asserted that a
  file checked into `src/` is reachable from the crate root. A file that loses —
  or never gains — its declaration stops being type-checked, stops being linted,
  and if it holds tests they stop running, while still reading as source to
  every human and every external tool. The three, and what happened to each:

  - `src/cli/commands/status_args_ext.rs` survived 303 lines of not parsing as
    Rust at all — it begins mid-struct-body, and rustfmt rejects it with
    "visibility `pub` is not followed by an item". Orphaned output from a
    mechanical split; all 101 of its field names already exist verbatim in
    `status_args.rs`. Deleted.
  - `src/core/planner/tests_sat_deps_b.rs` is the one that cost something. Its
    `mod` line was never written, so its 10 assertions on unsat conflict-clause
    extraction, negative unit clauses, redundant clauses and serde — the
    SAT-solver share of the "134 tests" its commit message claimed — had never
    executed. Wired in, all 10 pass, so `sat_deps.rs` was right; that was luck,
    not evidence.
  - `src/core/planner/tests_proof_obligation.rs` was redundant rather than lost:
    its `classify` assertions are a subset of the 28 exhaustive ones in
    `tests_proof_cov.rs`, and its `label`/`is_safe` assertions are duplicated in
    `tests/falsification_proof_security.rs`. Deleted rather than wired in.

  `src/transport/tests_container_b.rs` and `tests_container_c.rs` were
  byte-identical and both declared, so eleven container-transport tests built
  and ran twice per `cargo test` while the count read as twenty-two. `_c` and
  its declaration are gone; `_b` keeps every test name.

  `tests/falsification_no_orphaned_source_files.rs` pins both properties by
  walking the tree rather than by listing the three files found today: every
  `.rs` under `src/` is named by a `mod`, a `#[path]` or an `include!` in its
  own directory (or in `<dir>.rs`, the 2018-edition parent form), and no two
  `.rs` files under `src/` or `tests/` are byte-identical. Over all 1354 tracked
  `.rs` files under `src/` it finds exactly those three, with zero false
  positives; a third test asserts the walk found over a thousand files, so a
  broken walk fails loudly instead of passing by measuring nothing. It is named
  in ci.yml's hand-listed integration targets, because nothing in this repo's CI
  runs `tests/*.rs` otherwise — a falsification test left unnamed there is green
  once on a developer's machine and never executed again, which is the same kind
  of artifact it checks for.

  **#292 is a sweep report and item 2 of it is deliberately left.** The 98.5%
  near-clones under `src/cli/` are a different problem — a ~200-field
  `StatusArgs { .. }` literal repeated across ~60 test files, which wants a
  `Default` impl and a rewrite of all of them, so the issue stays open. Its item
  4 needed no change: `.gitignore:26` has been `.claude/*` since before the
  commit that was analysed.

- **The contract-citation guard measured 77 of 211 citations and reported
  success** (#298). The 1.16.0 fix repaired the four contracts an audit had
  named by hand and added a CI resolver shaped around those same four cases, so
  the defect survived one level up — in the guard. `pv audit
  contracts/verb-surface-v1.yaml` printed `Falsification tests: 8` and `No audit
  findings` and exited 0 while seven of those eight cited code that does not
  exist: four named FILES that were never created, three named functions that
  exist nowhere. The corpus writes citations in four shapes (`path.rs`,
  `path.rs::fn`, `path.rs::mod::fn`, `path.rs mod::fn`) and
  `re.fullmatch(r"([\w./-]+\.rs)::(\w+)", ...)` accepts the second; the other
  three fell through a `continue` and were reported as "not resolvable" rather
  than "not resolved". Two further narrowings compounded it: the grep ran over
  `src tests benches` GLOBALLY, so a citation naming the WRONG FILE always
  passed, and only `falsification_tests[].test` was read, so 104 `enforced_by`
  and 12 `discharged_by` citations were never resolved at all.

  `tests/falsification_contract_citations_resolve.rs` states the invariant in
  Rust: every citation resolves to the exact item it names, IN THE FILE IT
  NAMES. Thirty-nine offenders on arrival — 14 whose cited file does not exist,
  25 whose function is not in the file cited — plus `verb-surface-v1.yaml`'s
  `qa_gate.check` naming three `--test` targets that had never existed, so the
  documented way to check that contract was `error: no test target named ...`.
  The boundary is written down because it is what will be argued about: only
  `falsification_tests[].test` and the `enforced_by` and `discharged_by` keys of
  `proof_obligations[]` are resolved, because those are the keys whose VALUE is
  a citation — a resolver that also walked the free prose in `description:`,
  `notes:` and `if_fails:` would land red on arrival and get weakened back into
  the vacuous pass it replaces. Citations were retargeted rather than deleted
  where the property they assert is genuinely enforced. The Python heredoc in
  `proofs.yml` is deleted rather than reimplemented — one resolver, in one
  dialect, in a place that runs it — and the Rust guard is named in ci.yml.

  A second pass then found one more shape the new resolver could not read: where
  a contract names several items under one path, `item_after` returned the first
  and stopped at the comma, so ten function names written as continuations were
  resolved by nothing. `items_after` continues over comma-separated bare
  identifiers under the same path. Verified by POISONING the corpus rather than
  by asserting the parser should cope — replacing a cited function with a name
  present nowhere in the tree gives `7 passed; 0 failed` before and `6 passed; 1
  FAILED` after.

  **Building from source: `cargo check` can now fail where it did not.**
  `build.rs` called `verify_bindings`, which reads `status:` and opens no
  contract file — which is how a binding for an equation no contract defines
  counted toward "43/43 bound". `verify_binding_equations` resolves the other
  half and fails the build naming the binding and the contract that does not
  define it. One binding was in that state: it claimed `receipt_deletion`, an
  equation `apply-receipt-v1.yaml` has never declared.

  **Still open, and stated here because over-reporting is this issue's own
  subject.** The issue's fix item 1 — make `pv audit` resolve its citations, so
  a falsifier naming a nonexistent function is a finding — is NOT done here, and
  is not a change to this repo: it is a change to aprender-contracts. `pv audit`
  still counts declarations without resolving one, so it would report `No audit
  findings` over the corpus this entry opens with exactly as it did then. The
  corpus is repaired and the invariant is now held by a Rust test run from
  `cargo test`; the instrument the issue named is not. Second, the measured
  citation set went from 77 to 211, but
  `the_parser_reads_every_citation_shape_the_corpus_uses` claims a totality it
  does not have: its ten cases are hand-written literals, not derived from the
  corpus, and none of them was a comma list — which is why it stayed green
  throughout while the parser was blind to a shape the corpus uses on ten lines.
  Deriving those cases from the corpus is the honest fix and is its own change.

- **Narrowing `lifecycle.ignore_drift` widened it to everything** (#335).
  `ignore_drift` is a FIELD LIST in the schema; the engine read it as
  `!lifecycle.ignore_drift.is_empty()`. So `ignore_drift: ["mode"]` — written to
  tolerate a mode change while still catching content tampering — silently
  disabled content, owner, group, existence and image drift as well, across
  `forjar drift`, `apply --tripwire` and the pull agent. The narrowest thing an
  operator could write was the broadest exemption forjar can express, and a typo
  (`["modes"]`) was that same skip-all by the same mechanism. Nothing rejected
  either form: `known_fields.rs` knew the KEY and no validator ever looked at
  the values, so `forjar validate` printed a clean verdict over a declaration
  that meant the opposite of what it said. Reproduced end to end on the real
  binary: apply converges a file carrying `ignore_drift: [content]`, the bytes
  are changed on disk, and `forjar drift` prints "No drift detected." The
  example we ship, `examples/cookbook/33-lifecycle.yaml`, taught exactly that
  shape under a comment promising it only ignored content. #333 made tasks
  convergeable, which widens the population of resources reaching for this
  opt-out, so the cost of it meaning more than it says was growing.

  `["*"]` is now the only honoured value and the only accepted one. A narrowed
  list is REFUSED at config validation with a message naming the offending
  tokens, and the engine asks `LifecycleRules::suppresses_all_drift` instead of
  collapsing a list to a boolean, so a narrowed list that still reaches the
  engine by any route means "keep looking", which is the safe direction for a
  tripwire.

  **This ships two of the three things the issue asked for.** The narrowed form
  is refused, and a typo falls out of that refusal rather than becoming a silent
  skip-all. The field list is NOT honoured, and that is not a deferral for
  convenience: `ResourceLock.observed` is one opaque digest of the state query's
  output, so there is no representation in which `mode` changed and `content`
  did not, and nothing for a field list to select over. Honouring it needs a
  per-field observation in the lock (a schema change, with the migration
  treatment `StoreMeta` just got) and state queries that emit parseable fields
  rather than a digest — which each resource type decides for itself, so it is a
  change per resource type, not one central change. Split to **#360**, which is
  where the remaining work is tracked — but the validation error quotes
  `forjar#335`, not #360, and explains what forjar would otherwise have done
  rather than claiming the declaration is merely illegal. #360 appears nowhere
  in the binary; #335 is the number to search for if you hit the error.

  **Breaking, deliberately.** A config carrying `ignore_drift: [content]`
  validated yesterday and hard-fails today, on validate, plan and apply alike,
  and the error names the one-token edit. That includes a list supplied by a
  RECIPE, which this change on its own would have let through: recipes expanded
  after `validate_config`, so the refusal could not see them. #357, in this same
  release, validates the expanded config too, so a recipe-supplied narrowed list
  is refused at load — naming the expanded id (`recipe_id/foo`) — rather than
  reaching apply. `the_narrowed_form_is_refused_when_supplied_by_a_recipe` is
  the test that pins it. So for a recipe the upgrade symptom is a config that
  will not load, not a one-time drift report.

- **Only the human was told the plan had a blind spot** (#342). The disclosure
  1.20.0 added landed for the TTY rendering ONLY; `plan --json` and the
  MCP/HTTP/verb `plan` kept presenting a lock diff as the state of the world. It
  had been implemented as a side-effecting printer rather than as a value:
  `print_scope_disclosure` formatted the sentence and immediately `println!`d
  it, returning `()`, so the only way to consume it was to be INSIDE
  `print_plan` — the JSON arm structurally could not wire it, and
  `mcp::handlers::PlanHandler` had nothing to attach to `PlanOutput`.
  `plan-declares-its-quantifier-v1`'s equation `discloses(plan_output) ⟺
  unconsulted(locks) > 0` is not qualified to the TTY rendering, so on two of
  three shipped surfaces the left side was false while `unconsulted > 0`. And it
  inverted the issue's own threat model: #342's motivating incident is
  machine-driven — a nightly lane parsing forjar output, quoting a "52 changes"
  figure from the blind command — so the consumers that cannot NOTICE a missing
  disclosure, a CI parser or an MCP agent reading `to_update: 0`, were exactly
  the ones still being handed the undisclosed diff.

  `print_helpers::scope_disclosure` now returns `Option<String>` and
  `print_scope_disclosure` is three lines over it, so `print_plan`'s signature
  and the TTY text are byte-identical. `plan --json` and `PlanOutput` — shared
  by `forjar verb call plan`, MCP stdio and HTTP through `verb/registry.rs`, so
  this is one missing value across three surfaces, not three bugs — now carry
  `"lock_relative": true` and `"unconsulted_observations": N` unconditionally,
  and `"disclosure"` only when `N > 0`. The split is deliberate: suppressing the
  prose at zero is about OPERATOR ATTENTION, since an unconditional banner is
  noise and noise is how a warning stops being read, while the COUNT is total so
  that a parser can tell `unconsulted_observations: 0` ("nothing observed") from
  an absent key ("older binary"). The MCP handler counts over the locks it
  planned over, the same convention `load_machine_locks` uses on the CLI, and
  reaches the counter through named shims rather than a second implementation
  that could drift. `docs/mcp-schema.json` is regenerated — it was stale at
  `"version": "0.1.0"`, so the diff is much larger than the three new fields;
  leaving a published schema that far behind reproduces this issue one level up.
  RFC steps 2-5 are out of scope and the issue stays open for them.

- **A `.crates.toml` that cargo already rejects was merged into, and forjar
  reported converged** (#345). 1.20.1 made `_fj_register` entry-aware so forjar
  no longer SCRAMBLES `$CARGO_HOME/.crates.toml`. That closed the half forjar
  caused; it did not close the half the issue said mattered most — the
  read-back. A correct merge INTO wreckage is still wreckage, and every host
  that ran a pre-1.20.1 forjar has a file cargo rejects in whole for one bad
  entry. `mv -f` cannot fail on content, so `_fj_register` returned 0, the
  package resource reported CONVERGED, `cargo install --list` went on naming
  nothing, and `package_check` read that empty list and said `missing:<crate>` —
  the exact symptom on intel, where sixteen CI runners share one `$HOME`.
  `_fj_register` now gates the commit on `_fj_crates_ok`, which copies the
  candidate into a throwaway `CARGO_HOME` and runs `cargo install --list`
  against it. Ask CARGO, not a TOML library: cargo is the only consumer that
  matters and it is the parser that rejected the file. On refusal the temp file
  is removed, the destination is left byte-identical, the operator is told which
  file and whether it was already broken before this run, and `return 1` FAILS
  the resource instead of lying about it. The probe costs 0.015s per registered
  crate, needs no network, and is fail-open on absent cargo, failed `mktemp` and
  failed `cp`, so a broken `/tmp` cannot wedge every install on a host.

  **Behaviour change, deliberately.** A machine whose `.crates.toml` is already
  wreckage now fails its cargo package resources loudly instead of appending to
  a file cargo cannot read. The fix is to repair or move that file aside, not to
  revert this. On the cache-hit path the binaries are installed before
  registration is refused, so such a host ends up with working binaries cargo
  does not know about — and the check then honestly reports them missing.
  Concurrency is untouched and out of scope: sixteen runners sharing one `$HOME`
  can still lose an entry through read-merge-mv (#331, #320).

  **The first version of that gate made every `provider: cargo` apply script
  unrunnable, and every gate this repo runs was green over it.** Nothing
  shipped: it was introduced and removed inside this release, both commits on
  the 1.21.0 integration branch, so there is no published version that cannot
  install a cargo package. What is worth stating is how it got that far.
  `_fj_crates_ok` cleaned up its throwaway `CARGO_HOME` with `rm -rf "$_vh"`.
  bashrs rates SEC011 — missing validation before `rm -rf` — at Error severity,
  `transport::validate_before_exec` refuses any script carrying an Error
  diagnostic, and `strip_data_payloads` whitelists only `$_STAGING`,
  `$_CACHE_DIR` and `$_CARGO_BIN`, so nothing exempted the probe's temp
  variable. Every `provider: cargo, state: present` apply script was REJECTED
  before execution: `forjar apply` on a cargo package died with an I8 violation
  and installed nothing, while the same config on origin/main ran normally — and
  cargo is the provider forjar dogfoods for its whole stack-tool fleet. The
  guard now satisfies the rule rather than suppressing it, chosen against the
  real linter (bashrs 6.67.0) rather than by guesswork:

  | form | bashrs |
  |---|---|
  | `rm -rf "$_vh"` | SEC011 error |
  | `_fj_rmtmp() { ... rm -rf "$1"; }` | SEC011 error (positional) |
  | `[ -n "$_vh" ] && rm -rf "$_vh"` | 0 errors |
  | `if [ -n "$_vh" ]; then rm -rf "$_vh"; fi` | 0 errors — taken |

  The `if` form over the `&&` form because `&&` yields a non-zero status when
  the guard is false, which would trip `set -e` at a site whose whole purpose is
  to fail open. Widening the strip whitelist was the other option and is worse:
  it would hide the check instead of answering it.

  **And the reason every gate was green: nothing in this repo pushed a GENERATED
  script through the gate that runs before execution.** `cargo test`, `cargo
  clippy` and #345's own falsification test all passed while the resource could
  not run at all. `src/transport/tests_generated_scripts_lint.rs` closes that —
  it walks a table of resources and asserts that every generated apply, check
  and state_query script survives `validate_before_exec`. It calls
  `validate_before_exec`, not `purifier::validate_script`, because the property
  that must hold is the COMPOSITION: `strip_data_payloads` runs first, and this
  regression was precisely that the strip did not cover the probe's variable. It
  carries a denominator test so a corpus that shrinks to nothing cannot read as
  a pass. With the guard reverted and the test kept,
  `every_generated_apply_script_survives_the_i8_gate` fails naming the one
  script that broke while check and state_query stay green. Found by adversarial
  review of the first commit, not by the test suite. The same pre-execution gate
  is the subject of #350 below, which is the other way forjar rejected its own
  shell in this release; that one is not covered by this corpus, because no row
  in it carries a config value with an apostrophe.

- **A `cron` resource with no `owner:` installs into ROOT's crontab, and its
  check read the invoking user's** (#348). The resource was correctly installed
  and permanently unconvergeable, and every dependent was skipped. Measured on
  paiml's intel:

      $ crontab -l | grep -c ci-image-rebuild
      0                                   <- what the check looked at
      $ sudo crontab -l | grep rebuild.sh
      30 3 * * * bash .../rebuild.sh      <- where the apply had put it

      JIDOKA: intel/ci-image-rebuild failed - dependents will be skipped:
        apply exited 0 but the host does not report the declared state
        (check exit 1)

  `check_script`, `apply_script` and `state_query_script` each re-derived their
  crontab command independently, and only the apply carried the `SUDO=""` /
  `[ "$(id -u)" -ne 0 ] && SUDO="sudo"` preamble. They agreed on the owner
  (`root`); they disagreed on the PRIVILEGE. Reading another user's crontab is
  exactly as privileged as writing it — `crontab -u <user>` refuses EVERY
  non-root caller, even for the caller's own username — and `2>/dev/null`
  swallowed that refusal, so `grep -qF` read an empty stream and exited 1. The
  check could not distinguish "the job is not installed" from "I was not allowed
  to look", and asserted the first. The same omission in `state_query_script` is
  the more expensive half: the observable recorded `cron=MISSING:<name>` for a
  job that exists, so the lock stored "absent" as the OBSERVED state and drift
  was wrong in the same direction.

  One function decides now — `crontab_user()` for the identity, `SUDO_PREAMBLE`
  for the privilege — and all three call sites delegate. The apply's emitted
  bytes are unchanged, so no apply behaviour and no apply-side script hash
  moves. **One-time re-converge**, on the read side: the state query now carries
  the preamble too, so on a host where the read was previously refused the
  recorded observable moves from `cron=MISSING:<name>` to what `grep -A1`
  actually captures — the `# forjar:<name>` marker and the `# forjar-cmd:<name>`
  line after it. Not the schedule line itself: `apply_script` writes marker,
  cmd_marker, then the entry, so one line of trailing context stops short of the
  job — so the observable still does not contain the schedule or the command,
  and cron drift is blind to a job being edited in place. That is **#362**, filed
  from this paragraph: both captured lines are constant functions of the resource
  name, so the digest is identical for every possible schedule under a given
  name. #348 made the read reach the right crontab; it did not make it look at
  the job. Every cron resource that was being observed as absent
  while installed therefore reports drift once and then settles — the same shape
  #349 records below. `CRONTAB_CHECK_GUARD` is the second half and is not
  optional: `crontab -l` exits 1 for BOTH "no crontab for user" and EPERM, so on
  a host with no passwordless sudo the false `missing:` would simply move one
  step later. The guard takes the honest signal BEFORE the read and exits 2,
  which `cli::check` maps to SKIP and `output_verify` treats as neither
  converged nor diverged. **Reporting change:** a host without passwordless sudo
  moves from a false `missing:`/FAIL to SKIP, so anyone counting cron resources
  as failing will see that count move. It cannot hang — `stdin_isolation` gives
  the whole script `< /dev/null`, ssh uses `BatchMode=yes` with no `-t`, and
  `sudo -n` never prompts. The docs had already promised this and the code never
  did: 03-resources.md's lint table said "`$SUDO` in crontab read/write" while
  the read half did not exist.

- **`sudo: true` governed the apply and neither of the two read paths, so a
  file on any root-only path reported `missing:` forever** (#349). Measured on
  paiml intel: `toolchain-audit-rule` wrote
  `/etc/audit/rules.d/50-cargo-bin.rules` correctly, apply exited 0, and the
  check then failed with `missing:file`. `/etc/audit` is `drwxr-x--- root root`
  on stock Debian/Ubuntu, so the unprivileged `test -f` could not TRAVERSE to
  it — DAC denied the directory, not the file. `sudo` is a property of the
  RESOURCE and `dispatch.rs` treated it as a property of the APPLY PHASE: there
  is one privilege resolver and exactly one of its three sibling entry points
  called it. So the check did not answer a weaker version of the apply's
  question, it answered a DIFFERENT one — the apply asked "is there a file at P,
  as root?" and the check asked "is there a file at P, as noah?", and under a
  mode-0750 root-owned parent those answers differ permanently. The failure is
  then fed forward: `post_apply_failure` records the resource Failed and jidoka
  skips every dependent, which in the reported case were the readback and the
  `augenrules --load` that arm kernel auditing — a privilege bug in the READ
  path disabled a security control by refusing to run the steps that enable it.
  `state_query_script` was the same defect with a quieter symptom: `live_hash`
  and `observed` recorded the digest of the literal string `MISSING` for a file
  that was there.

  The resolver is renamed `in_declared_privilege_context` and called by all
  three entry points; its body is byte-identical, so no existing `sudo: true`
  apply re-converges. **One-time re-converge** in two places: `live_hash` for a
  `sudo: true` resource on a root-only path changes from the digest of `MISSING`
  to the real one, and `disk_budget` folds all three scripts into its
  desired-state hash.

  **This changes a failure mode on real fleets, and not only where apply was
  already broken.** The wrapper is `sudo bash <<'FORJAR_SUDO'` with no `-n`
  (`src/core/codegen/dispatch.rs`), and `sudo` overloads exit 1 for its own auth
  failures. So on a host where sudo needs a password, an operator who runs
  `apply` interactively, where sudo can prompt on the terminal, but runs `drift`
  or `check` from a systemd timer with no TTY, now gets a hard failure on every
  `sudo: true` resource whose path an unprivileged check could previously read
  perfectly well — and the check reports "diverged" rather than "could not
  observe", because sudo's exit code does not distinguish them. That is wider
  than the scope recorded when the fix landed, which put the regressing set at
  exactly the hosts where apply was already broken for that resource; that holds
  only where apply and the read verbs run in the same context, and the
  interactive-apply / TTY-less-timer split is the case it misses. Papering over
  it with `sudo -n` and a fallback to the unprivileged probe would reintroduce
  the two-contexts ambiguity this removes. Not a complete fix for the class
  either: resources that decide privilege internally and ignore the field are
  untouched — `network.rs` elevates its apply unconditionally while its check
  runs `ufw status` unelevated regardless — which is the same shape one level
  down and belongs with #348's sweep of per-resource defaults.

- **forjar's own I8 gate rejected the shell forjar generates: any generated
  script carrying a config value with an apostrophe was refused before it
  reached a host** (#350). The everyday way to hit it was the
  `unobservable:no-completion-check:` sentinel a task with no `completion_check`
  gets. Measured:

      DRIFTED: ci-budget-activation (transport error: I8 violation —
        script failed bashrs validation: bashrs lint errors:
        [error] SC2075: Escaping a single quote in single quotes won't work.

  So instead of a clean "this resource is unobservable" report, the resource
  reported ERROR and the whole drift run degraded. The issue's diagnosis —
  "`sh_squote` is not applied here" — is wrong; it IS applied, and the `'\''` in
  the pasted script is the escaper's output. `sh_squote` rendered an embedded
  quote as `'\''`, the familiar POSIX close/escape/reopen and correct shell, but
  forjar lints every script it generates with bashrs before executing it, and
  bashrs' SC2075 is a line-scoped regex (`'[^']*\'[^']*'`) with no quote-state
  tracking: it matches the CORRECT idiom because it cannot tell it from the
  genuine error `echo 'can\'t'`. This was never about the sentinel — output
  artifact paths, package names, mount labels and cron commands all carry
  apostrophes, and the FJ-154 injection-hardening tests construct exactly such
  values and pinned the `'\''` output, so the hardening and the I8 gate were in
  direct contradiction. Fixed at the one escaper: `sh_squote` now emits
  `'"'"'`. `'a'\''b'` and `'a'"'"'b'` are the same POSIX word — the shell cannot
  tell them apart, a line-scoped linter can, and `'"'"'` is the form SC2075's
  own message recommends. That immunises all 262 call sites at once instead of
  adding a per-call-site dodge. This is the second defect at that gate in this
  release — #345 above is the other, from the opposite side — and the corpus
  test that one added, `src/transport/tests_generated_scripts_lint.rs`, does not
  cover this one: no row in it carries a config value with an apostrophe, so
  nothing there generates the idiom SC2075 rejects.

  **The second half: the sentinel named a command that was never run.**
  `sh_squote` STRIPS control characters — right for a shell word, wrong for a
  message whose whole job is to name a command back to a human. `set -eu` plus
  `sudo systemctl daemon-reload` was welded into `set -eusudo systemctl
  daemon-reload`. `render_command_inline` now renders the line breaks as `\n`
  instead of dropping them, and the sentinel is emitted with `printf '%s\n'`
  rather than `echo`. That second part is NOT cosmetic: dash, the default
  `/bin/sh` on Debian, has the XSI `echo` that expands backslash escapes, and
  the sentinel's stdout is what drift HASHES — with `echo`, the observable's
  bytes would differ between a bash target and a dash target and manufacture
  drift from nothing. Verified in the test: the same script under `sh` and
  `bash` produces byte-identical output with `printf` and diverges with `echo`.

  Blast radius. 25 inline assertions pinned the literal `'\''`; all were
  assertion text, no logic. Derivation script text is hashed, so a derivation
  embedding a value with an apostrophe rebuilds once. State and drift hashes are
  of query STDOUT, not script text, and both idioms print identical bytes, so
  the escaper change causes no drift churn. The sentinel change does alter
  stdout for multi-line commands, so those tasks report drift once; today they
  report ERROR and no drift verdict at all. Three hand-rolled copies of the old
  idiom were folded in so the invariant cannot drift back: `copia::shell_quote`
  delegates to `sh_squote` outright, while `wasm_bundle`'s inline content and
  `sandbox_exec`'s plan text switch idiom in place and deliberately do NOT call
  it, because they embed multi-line text whose newlines must survive.

- **A recipe's resources were validated by nothing** (#357). `load_config`
  validated, and only then expanded. `expand_recipes` runs after the
  `validate_config` call, so every resource a recipe supplied reached `plan` and
  `apply` having been checked by nothing at all. `includes` were given the
  opposite order deliberately — FJ-254 moved `merge_includes` ABOVE the
  validation call for exactly this reason — and recipes were simply never moved
  with them:

      includes:   merged at :284, validated at :289   OK
      recipes:    validated at :289, expanded at :300  <- never seen

  It is not a narrow hole. `validate_config` is where forjar's whole config-time
  contract lives, so the contract held only for authors who did not use recipes,
  and recipes are the mechanism forjar documents for fleet reuse — which makes
  the most widely deployed resources the least checked. The recipes chapter of
  the book tells the reader expansion begins with "Config YAML is parsed and
  validated", which was true of the config and false of the recipe. Fixed by
  validating AGAIN over the expanded config rather than by moving the call: the
  first pass reports errors in the file the user is editing, in the ids the user
  typed, and the second reports what the machine would actually converge, in the
  expanded ids (`recipe_id/foo`), so the id in the message is the id in the
  plan. A shared `render_validation_errors` keeps the two from drifting in how
  they report. The clean-recipe control test guards the obvious over-correction:
  a second pass that rejected legitimate expansion output — namespaced ids,
  resolved `{{inputs.*}}` templates — would make every recipe unusable, which is
  worse than the hole it closes.

  **Closing that hole immediately failed two examples we ship, and the examples
  were right.** `examples/dogfood-renacer.yaml` and
  `dogfood-sovereign-stack.yaml` were refused with `resource
  'observability/obs-grafana-data': invalid owner '472' (expected Unix username
  like 'root' or 'www-data')`. Grafana runs as uid 472 and that account has no
  passwd entry on the host, which is the normal case for a directory
  bind-mounted into a container, so `owner: 472` is the only way to express the
  ownership that makes the mount usable — and forjar already emits `chown 472
  /path` for it (`src/resources/file.rs:59`), which works. `is_valid_unix_name`
  required `^[a-z_][a-z0-9_-]*$`, so the correct config was unwritable. Both
  examples have carried `owner: 472` since they were written and both passed
  `forjar validate` every time, because their resources come from a recipe and
  recipes were never validated: the over-strict rule and the thing hiding it
  were the same defect, which is why they are fixed together.
  `is_valid_unix_name` now also accepts a bare numeric id, and the relaxation is
  bounded and pinned as such — `owner: "4x7;rm -rf /"` is still a validation
  error, because "accept a number" and "accept anything" are indistinguishable
  from a green suite otherwise. No example YAML was edited to make a test pass.

  **Breaking, and the one to read before upgrading a fleet.** Every rule in
  `validate_config` applies to a recipe's resources for the first time, and
  `load_config` returns `Err`, so a recipe that violates any of them turns a
  config that loaded yesterday into a hard failure on `validate`, `plan` AND
  `apply`. Two of the examples this project ships were in that state and had
  passed `forjar validate` every time. #335, in this same release, is the
  amplifier: a narrowed `lifecycle.ignore_drift` became a validation error, and
  a recipe-supplied one was previously seen by nothing, so a config can break on
  the pair where neither alone would have touched it. `forjar validate` is the
  cheap preview and reports the same errors the plan would; ids in the
  post-expansion pass are the expanded ones (`recipe_id/foo`), which is what the
  plan carries rather than what the recipe file says.

### Changed

- **`forjar cache verify` was comparing against the wrong thing, and has been
  for as long as it has shipped** (Refs #236). It re-hashed `<entry>/content`
  with `tripwire::hash_directory` and compared the result to the entry's
  DIRECTORY NAME. But an entry written by `forjar store-import` is addressed
  with `provider_exec::hash_staging_dir` — a different preimage under a
  different domain tag. So `cache verify` reported **100% failure on any store
  built by an import**, while a conda entry (also `hash_directory`) passed.
  Three addressing schemes coexisted with no field saying which one an entry
  carried.

  It now compares against `meta.output_hash`, the digest the entry itself
  recorded. **This is a visible change to an existing exit code**: a CI job that
  has been red-always against an import-built store goes green, and entries
  written before schema 1.1 report `unsealed` rather than a false mismatch. The
  JSON keys (`verified`, `failed`, `results[].hash|valid|expected|actual`) are
  unchanged.

- **`meta.yaml` schema 1.0 → 1.1** (Refs #236), adding `output_hash` and
  `addressing`. Both carry `#[serde(default)]`, so **every schema-1.0
  `meta.yaml` already on disk still loads** — pinned by
  `a_schema_1_0_entry_still_loads_and_reports_unsealed`, which is measurably red
  without the default on `addressing`. Such entries report `unsealed`: there is
  no recorded digest for them to be wrong about, and calling that corruption
  would make `--repair` delete good data.

  Note what was NOT done. The issue proposed promoting
  `provider_exec::hash_staging_dir` to the canonical content hasher; that is
  declined. Its walker does `std::fs::read(&path)` — it slurps each whole file
  into RAM, which would OOM on the 149.9 GiB mp4 store this issue exists for.
  `tripwire::hash_directory` streams, already skips symlinks, already sorts
  children, and is already what every verification site calls.

- **`forjar build --push` no longer shells out to `curl`** (Refs #228). Every
  registry verb — HEAD, POST, PUT and the chunked PATCH — is now an in-process
  `ureq` call through the new `core::store::registry_http` transport.

  `curl` was an **undeclared runtime dependency**: nothing in `Cargo.toml` or
  the docs said you needed it, and on a host without it the first HEAD died as
  `No such file or directory (os error 2)`. 1.12.6 (#224) made that message
  actionable; this removes the dependency it was reporting.

  **ureq, not reqwest.** `src/core` contains zero `async fn` while the MCP path
  runs a live tokio runtime, and `reqwest::blocking` panics when called from
  inside an async context. The 1.12.6 note suggesting reqwest because the crate
  "already compiles" it has been corrected in place.

  **Operators with a private-CA registry, read this.** curl validated TLS
  against the OS trust store. ureq's default is a bundled Mozilla root set, so
  the agent is built with the `platform-verifier` feature and
  `RootCerts::PlatformVerifier` to keep validating against the platform store.
  A push to a registry fronted by a corporate or internal CA behaves as it did
  on 1.20.1.

  The two load-bearing gates were carried across as explicit checks on the
  response rather than as curl flags, and are now pinned behaviourally against
  a live loopback registry in
  `tests/falsification_registry_push_needs_no_curl.rs`:

  - **Refs #154** — a push is judged by the registry's status, not by whether
    the request completed. The guard used to be `--fail-with-body`; it is now a
    2xx gate plus the registry's own error body quoted into the message.
  - **Refs #210** — only `202 Accepted` opens an upload session. ureq follows up
    to 10 redirects by default, so the agent sets `max_redirects(0)`. Without
    it, `docker.io`'s 301 to its marketing site is followed, the client sees a
    2xx, and the blob is PUT at a web page — measured: with redirects left on,
    that test reports a successful push against a decoy.

  Two side effects worth knowing about:

  - The four in-process-registry tests in `tests_registry_push_net.rs` were
    permanently `#[ignore]`d because curl honors an ambient `HTTP(S)_PROXY` even
    for `127.0.0.1`. The proxy is configured per agent now and disabled for
    loopback, so all four run. One of them,
    `chunked_push_succeeds_even_on_http_500_because_curl_silent`, asserted that
    an HTTP 500 was a **successful** push — the exact inverse of #154. It had
    been false since `--fail-with-body` landed; nothing ran it. It is now
    `chunked_push_fails_on_http_500`.
  - The chunked PATCH used `curl -r <range> --data-binary @file`. `-r` is a
    download-side flag, so every chunk uploaded the **whole blob**. Each PATCH
    now streams its own byte range and declares its own `Content-Length`.

  The `head_check_command` / `upload_initiate_command` /
  `upload_complete_command` / `manifest_put_command` doc-string builders are
  gone: they returned curl command lines that describe nothing this code does
  any more.

- **`forjar build` no longer pulls the artifact back with `scp`** (Refs #290).
  The Sovereign AI Stack ships copia as the rsync replacement, and pulling one
  cross-compiled binary back from a build host is squarely copia's domain —
  `copia sync host:path dest` works today and requires NOTHING on the remote,
  because it streams over `ssh host "cat ..."`. Unlike rclone (cloud backends)
  and curl (HTTP), there was no out-of-domain argument here; scp was simply the
  tool reached for first when FJ-33 was written, before the sovereignty policy
  was anything but prose. #291 turned that prose into
  `src/resources/sync_tools.rs` and recorded this call site as
  `Justification::Debt("paiml/forjar#290")`, which made the debt visible without
  paying it — and a standing exception nobody has to remove is how a debt
  becomes permanent. The ledger row goes with the call site, and the partition
  tests make the two edits indivisible: delete the row alone and
  `every_external_sync_binary_is_justified` fails, rewrite the invocation alone
  and `the_partition_has_no_stale_entries` fails.

  **Behaviour change.** copia becomes a runtime dependency of the `build`
  resource on the DEPLOY machine. scp ships with essentially every openssh
  install; copia does not. Hosts already carrying `stack-tool-copia` are fine,
  anything else must add it before this lands. The preflight refuses with
  `cargo install copia --features cli` before touching the filesystem —
  deliberately before `mkdir -p`, so an operator does not get a half-made
  destination and then a message about a missing binary, in that order. Same
  shape as `nas_archive`'s mover preflight.

  Two properties are transitively rather than locally guaranteed now, and are
  worth knowing. copia's remote pull invokes plain `ssh <host> "cat ..."` with
  no `-o BatchMode=yes -o ConnectTimeout=10`, where scp had both — Phase 1
  already ssh'es with them and would fail first, so a run that reaches Phase 2
  has proven key auth, but the guarantee now lives in Phase 1. And copia has no
  delta transfer for remote paths and buffers the artifact in memory, which is
  correct for ONE freshly-built binary and must not become the pattern for
  trees; the call site says so.

  The remote spec is unchanged — copia's `FileLocation::parse` splits
  `host:path` on the first colon exactly as scp does, so FJ-154's escaping story
  (validated host from `is_valid_host`, `sh_squote`d artifact path) is preserved
  as a property. Its emitted bytes are not: #350, in this same release, changed
  what `sh_squote` renders an embedded quote as, from `'\''` to `'"'"'`. Same
  POSIX word, different script text.
  `chmod +x` is now REQUIRED rather than defensive, because copia writes with
  default permissions. The falsification test EXECUTES the generated shell
  against stub `ssh`, `copia` and `scp` on PATH, where the scp stub touches a
  sentinel: a `script.contains("copia")` assertion cannot tell calling copia
  apart from calling scp with the word copia in a comment.


## [1.20.1] — 2026-08-26

**The `.crates.toml` merge corrupted multi-line entries, and cargo rejects the
whole file for one bad entry.**

`_fj_register` merged the staging `.crates.toml` into `$CARGO_HOME`'s line by
line. cargo writes MULTI-LINE arrays for any crate installing more than one
binary, and the merge did not know that. It broke in both directions:

| | |
|---|---|
| `head -1` on the **source** | took only `"kani-verifier ..." = [` and dropped the array body and its `]` |
| `grep -v "^\"$_key "` on the **destination** | removed only the KEY line of the entry being replaced, orphaning its body mid-file |

The comment above it stated the false premise outright — *".crates.toml is
`[v1]` followed by one line per install"* — so the bug was written down as the
design.

Both fired on the paiml fleet from a single apply, leaving:

```toml
"cross 0.2.5 (...)" = [
    "cross",
    "cross-util",
]
    "cargo-kani",      <- orphaned body, no key
    "kani",
]
...
"kani-verifier 0.67.0 (...)" = [      <- key 16 lines later, never closed
```

cargo refuses the entire file for one malformed entry, so `cargo install
--list` returned **nothing** on a host whose `$HOME` is shared by sixteen CI
runners. Every `package` resource then failed its check with `missing:<tool>`
while every binary was present and runnable — silent in the worst way, because
the binaries keep working until something asks cargo. It also re-corrupted on
**every** apply, so the machine could not be converged at all.

awk now tracks the array, so an entry is dropped and appended whole. Still no
TOML parser — this is generated POSIX shell for hosts that may lack python —
but awk is entry-aware, which is the property that was missing.

The test **executes the generated shell** over a fixture rather than asserting
on its text, and is falsified by restoring the old merge.

### If you were affected

A `.crates.toml` corrupted by an earlier version is not repaired automatically.
`cargo install --list` failing with `invalid TOML found for metadata` is the
symptom; the file needs its orphaned array fragments removed and any truncated
entry closed. Binaries in `$CARGO_HOME/bin` are unaffected throughout.


## [1.20.0] — 2026-08-26

**`plan` now states the quantifier its report ranges over.**

`plan` compares the config to the LOCK. `drift` compares the lock to the HOST.
Both are correct for the question they answer, and they disagree about the same
machine — measured on the paiml fleet:

```
intel  plan  (lock-relative):  0 to add, 52 to change, 0 to destroy, 83 unchanged
intel  drift (host-relative):  Drift detected: 28 resource(s)
```

Neither number contains the other. In a sandbox the gap is starker: mutate a
managed file on the target and `drift` reports two findings while `plan` prints
`no changes / 1 unchanged`.

The defect was never that plan is lock-relative — that is its job, and making it
contact machines would make it slow and network-dependent. The defect was that
plan presented a lock diff **as the state of the world**. `0 to change` reads as
"nothing is wrong" when it means "nothing in the lock disagrees with the config".

Plans now carry:

```
This plan is lock-relative: it compares the config to the lock, and did not
contact any machine. 134 locked resource(s) carry state observed on a
target that this plan did not consult — run `forjar drift` for what the machines
actually hold.
```

Three properties, each deliberate:

- **Printed on a clean plan too.** The dangerous case is the one where plan has
  nothing to report; a disclosure that only appeared alongside pending changes
  would be missing exactly when it is needed.
- **Not printed when there is nothing to be blind to.** With no lock, no
  observation exists — an unconditional banner is noise, and noise is how a
  warning stops being read.
- **The count is what plan is BLIND TO, never what drifted.** Establishing drift
  requires reaching the machine. A count presented as "N drifted" would be this
  same defect one level up.

### Contract

`plan-declares-its-quantifier-v1` — three equations, three bindings. **40/40 →
43/43 bound.** Its precedent is `apply-converges-observed-drift-v1`, written
after #305 showed every stated equation holding while the machine was never
looked at. That closed the quantifier for `apply`; this closes it for `plan`, by
disclosure rather than by widening the domain.

`plan_ranges_over_config_and_lock_only` is stated explicitly so that making plan
host-aware must contradict a written equation rather than quietly grow.

Both stated failure modes were injected and both go red: removing the disclosure
fails two of three tests; making it unconditional fails the no-lock test.

### Note for operators

If you have been reading `plan` as "is this machine converged", it was answering
a narrower question. `forjar drift --tripwire` is the host-truthful gate and
exits non-zero on real drift — it has existed for some time and is easy to miss.


## [1.19.0] — 2026-08-26

**`state: absent` did nothing for any file forjar did not create**, and the lock
stopped being able to confuse a spec for an observation.

### `state: absent` never removed anything forjar had not made (#339)

`determine_absent_action` returned `NoOp` when a resource had no lock entry. The
reasoning was written down in `why.rs`:

> `state: absent — resource not in lock, nothing to destroy`

That is a claim about the **lock**, not about the machine, and it is backwards
for the ordinary case. The reason to declare a file absent is normally that it
*exists* and forjar did **not** create it — a legacy file, a leftover, a stale
drop-in. Those are exactly the resources with no lock entry. So `absent` worked
only for files forjar had made itself, which is the case where you would simply
delete the declaration instead.

Every surface reported success. Reproduced in a clean sandbox with **no lock
file at all**, so this was never lock staleness:

| command | reported | file |
|---|---|---|
| `plan` | `no changes` — `0 to destroy, 1 unchanged` | present |
| `apply --yes` | `0 converged, 1 unchanged` — *Apply complete* | **survives** |
| `drift` | `Checking sandbox (0 resources)... No drift detected` | present |
| `apply --yes --force` | `1 converged` | removed |

`drift` did not merely miss them — it reported **zero resources**, so
absent-state resources were outside its accounting entirely.

It surfaced removing `/etc/sudoers.d/noahgift` from a fleet controller: a
dormant `NOPASSWD: ALL` grant for a user that does not exist, mode 0644 so sudo
skipped it, blocking every sudoers change on the host. A plain apply printed
`Apply complete` and left it there. **Declaring a file absent for a security
reason got you a green report and a live file.**

**Why the fix does not resurrect GH-229.** That bug was the mirror image:
`Destroy` was returned for anything in the lock, and since a successful destroy
writes the resource back as `converged`, the plan re-emitted `Destroy` forever.
Its fix added the hash check separating *converged as present, now redeclared
absent* from *the destroy already ran* — and that check is what makes `Destroy`
safe here. `rm -rf` is idempotent, the first apply records the absent-form hash,
the second takes the already-converged branch. Fixed point after one apply,
reached by observing the target rather than inferring from the declaration's
history.

**A divergence that predated the bug.** `explain_absent` decided for itself, and
differently: it returned `Destroy` for *any* locked resource — the pre-GH-229
rule, without the hash check — so `forjar why` said "will be removed" for a
resource `forjar plan` correctly no-ops. An explanation that derives its own
answer is not an explanation of the decision; it is a second decision that
happens to be printed. It now delegates.

Eight tests asserted the old behaviour as the requirement. One carried the
comment `// no lock — resource never existed`, which is precisely the assumption
that made this a defect.

### The lock now separates SPEC from STATUS (#337, step 1)

#305 — the defect 1.18.0 was released to fix — was a spec/status conflation bug.
Two different digests lived in the same untyped `details` map under string keys,
and every decision path read the wrong one:

| key | computed | read by |
|---|---|---|
| `live_hash` | on the **target**, through the transport | nothing, for five months |
| `content_hash` | on the **controller**, only when `content.is_some()` | drift detection |

The doc comment was the bug, written down: `hash` holds
`hash_desired_state(resource)` — the config, which never touches a machine — and
was documented as *"BLAKE3 hash of the resource's observable state"*.

`ResourceLock` now carries a typed `observed: Option<String>`, read through
`observed_state()` and written through `set_observed_state()`. A typed field
cannot be reached for by the wrong string key. `None` means **not observed** — a
third state, distinct from "observed and unchanged" — so drift skips rather than
inferring agreement.

Adding the field made all 159 construction sites declare which one they meant,
and two were wrong in a way review would not have caught: `record_success` was
passing `None` where the live digest belongs, and `--refresh` wrote only the
`details` copy, which would have left every reader on a stale digest. Two stores
with readers split between them is #305 rebuilt inside its own fix.

**Backward compatible.** `observed_state()` falls back to `details["live_hash"]`
on the read path only, so every existing lock keeps working and the fallback
cannot reintroduce two writers.

### Also

- `detail_str` replaces four hand-written `Some(Value::String(s))` matches.
- `.pmat/baseline.json` refreshed; a stale baseline from #333 had been failing
  the TDG gate on *every* commit in the repo, citing a file the committer had
  not touched.


## [1.18.0] — 2026-08-24

**`apply` converges what drifted on the target.** 1.17.0 and every version before
it reported `unchanged` for a file changed behind forjar's back, and `drift` did
not report it either — so a drifted file was neither detected nor corrected by
normal operation. On the paiml fleet, **320 of 329 locked file resources were
invisible to drift detection.**

This release is unusual in that most of it is fixing the first attempt. The
convergence fix (#307) shipped five defects of its own, an adversarial release
gate returned **3/3 NO-GO**, and 1.18.0 is what came out the other side. The
NO-GO reasoning is worth keeping: 1.18.0-as-first-written was *strictly weaker*
than 1.17.0, because one `--dry-run` silenced drift for every resource type on a
machine — converting a file-only blindness into whole-machine, operator-triggerable
silence.

### The root cause (#305, #297)

forjar recorded **two** observables per file resource on every apply:

| | covers | computed |
|---|---|---|
| `live_hash` | content + owner + group + mode + existence | **on the target**, through the transport |
| `content_hash` | bytes only, and only when `content.is_some()` | **on the controller** |

`detect_nonfile_drift` excluded `ResourceType::File`, so the complete oracle was
written every run and **read by nothing**. The exclusion arrived with the comment
"already handled by detect_drift_impl", which was false when written — `source:`
support had landed 3h49m earlier without extending `build_resource_details`. A
later refactor deleted the comment, so the false premise stopped being visible.

`ResourceStatus::Drifted` was a defined enum variant that **nothing ever wrote**.

### Fixed

- **`apply` converges observed drift** rather than refusing it. `--force` was
  never a repair: it empties the lock map so every resource re-applies.
- **A managed directory does not drift because files were added inside it.**
  `stat` size tracks entry count (4096 → 12288 at 400 entries); folding that into
  the live hash would have marked every managed directory permanently drifted and
  permanently un-appliable.
- **`Drifted` means "needs work", not "stop looking"** (#310). The detectors
  re-check it. Without this the CI tripwire fired once and then reported clean
  forever over a still-tampered file.
- **`apply --dry-run` no longer writes the lock**, and a **refused** apply no
  longer writes before being refused.
- **Templates are resolved before comparison** — a regression of PMAT-197, which
  `cli/drift.rs` had carried the fix and the reason for all along. 112 templated
  paths and 23 templated task commands were re-applied on every run.
- **Drift asks the machine, not the controller.** `check_file_resource_drift`
  routed through the transport only for *container* transports; every other
  machine — including plain SSH — hashed the controller's filesystem and reported
  it as the remote host's state. Measured: a file present locally and absent on
  the target produced "No drift detected".
- **A docker resource's identity is declared intent**, not `docker inspect`'s
  full output, which carries `StartedAt` and changes on its own every few
  seconds. Every apply was tearing down and recreating every container.
- **A symlink at a managed path is replaced, not written through.** `>` and
  `chmod` follow links, so forjar wrote declared content and mode onto arbitrary
  paths with its own privileges; the dangling-link variant was a create primitive.
- **`group:` without `owner:`** emitted no ownership command at all, then reported
  converged.
- **The cargo package observable checks the binaries**, not just `.crates.toml`.
  Metadata survives a pruned `$CARGO_HOME/bin`, which is why six fleet-wide
  toolchain deletions produced zero drift findings.
- **Every transport call on the apply path is bounded.** One host that connects
  and then stalls used to hang the whole run with zero output.

### Contracts

`apply-converges-observed-drift-v1` is new. Its `apply_reads_what_drift_reads`
equation initially **encoded the raw-resources defect as its specification** —
directly above two invariants saying the opposite. A guard that specifies the bug
cannot fail on it. Corrected, plus `observation_is_bounded`.

`idempotent-apply-v1` gains a `scope_boundary` saying out loud what its equations
do **not** range over: they observe no machine, so a deleted file still plans NoOp
and correctly so. The defect violated no contract, which is the part worth fixing
in the corpus.

### Known limits of the verification

The release gate ran 166 agents across 8 competitive-research lanes and 10
adversarial falsification lanes — and **every lane used `addr: 127.0.0.1`**, where
controller and target are the same box. It structurally could not test the
distinction this release exists for. The transport defect above was found
afterwards, by hand, with two real hosts.

## [1.17.0] — 2026-08-23

Everything in the unpublished 1.16.0 internal build — six release blockers,
including arbitrary command execution on target machines via a `file` resource
whose content contained the heredoc delimiter — plus all 14 ledger regressions.

### The regressions

Defects already recorded in `docs/cli-defects.json` under `confirmed`, verified
against 1.12.3, still reproducing at 1.16.0. They survived 1.13, 1.14, 1.15 and
1.16 — not because they were hard, but because nothing ever re-ran the ledger's
own repros.

**Fixed**

- **`state-decrypt` accepted any passphrase.** It counted errors and returned
  `Ok(())` regardless, so a wrong passphrase exited 0. In the shipped build —
  which has no `encryption` feature — it reported skips and success for state it
  could never have read. It now refuses up front and names the missing feature.
- **`status --hash-verify` counted hashes instead of comparing them**
  ("1/1 resources *have* BLAKE3 hashes"). It now recomputes, reports
  match/MISMATCHED/unverifiable separately, and exits non-zero on a mismatch.
- **`drift --tripwire -m <unknown-machine>`** reported no drift over ZERO
  machines. A typo in a cron'd `drift --tripwire -m intel` silently stopped
  checking anything and reported healthy forever.
- **`lock-prune --yes`** announced a prune and left the lock byte-identical.
- **`stack-diff` was blind to twelve config fields**, including `name` and
  `policy`. The comparison enumerated a hand-written field list; it now
  destructures `ForjarConfig` exhaustively, so a new field is a compile error in
  the differ.
- **`validate --json` emitted zero bytes on failure** — the one case a machine
  consumer needs the structured errors.
- **`audit --json` Debug-printed the event into a string**, making `run_id`,
  `operator` and `config_hash` unreadable without re-parsing Rust syntax.
- **`run` executed unexpanded `{{params.*}}`** and ignored `--param`;
  **`run --json` never executed** and masked a failing task's exit code;
  **`trigger` reported actions fired and ran nothing.**
- **`agent --auto-apply` never remediated.** Two bugs: the effect was a boolean
  computed from the flag, and the agent carried a parallel drift detector that
  read a lock path forjar has never written.
- **`retry-failed` never saw a recorded failure.** It string-matched
  `"ApplyCompleted"` against a log serialised `"apply_completed"`.

**Rejected, deliberately**

- `force-and-rollback-report-zero-actual-changes` describes **intended,
  contract-specified behaviour**. `apply-summary-distinguishability-v1` defines
  `forced_noop_count` as lock-based, its invariant holds, and a previous fix at
  that site was reverted as a regression. Moved to `rejected`. Its genuinely
  misleading *wording* is fixed: the note now says "N differed from the lock"
  and names `forjar drift` for the live question.

### Added

- **`ledger-replay`**, a required proofs job that re-runs the ledger's own repros
  and fails when one still reproduces. It prints its denominator every run, fails
  when it replays zero entries, and treats a timeout or broken sandbox as an
  error rather than a skip.
- A **`contracts`** CI job that validates every contract and resolves every
  falsifier citation. `pv audit` counts declarations without resolving them, and
  no CI job ran `pv validate` at all — which is how an invalid contract shipped
  on the 1.16.0 branch.


## [1.16.0] - 2026-08-21 — INTERNAL BUILD, NOT PUBLISHED

**This version was deliberately never published to crates.io and never tagged.**
It was built and deployed to our own infrastructure only.

Six release blockers were found by the mandatory pre-release dogfood and fixed —
including arbitrary command execution on target machines driven by managed file
content. A follow-up audit then confirmed **14 regressions**: defects already in
`docs/cli-defects.json` under `confirmed`, verified against 1.12.3, still
reproducing. They had survived 1.13, 1.14, 1.15 and 1.16, because nothing ever
re-ran the ledger's own repros.

The judgement call was whether to publish anyway, since those 14 are equally
present in the published 1.15.0 and so are not a regression against what users
have. The decision was **no**: "not worse than last time" is not a release
standard, and putting known-defective behaviour in front of users is not excused
by the previous release having had it too. So 1.16.0 took the security fixes
internally, and 1.17.0 carries them to crates.io with the 14 cleared.

See the `1.17.0` entry for the full list of what shipped in both.


### Fixed

- **`--yes` no longer disables the BLAKE3 state-integrity gate** (FJ-1270).
  The pre-apply check was written `has_errors(&issues) && !yes`, and `--yes` is
  documented as "skip confirmation prompt (CI mode)" and is *mandatory* for any
  non-interactive apply. So every scheduled apply, every CI apply and every
  `ExecStart=… forjar apply --yes` unit ran with tamper detection off: over a
  lock file that did not match its `.b3` sidecar, apply printed
  `ERROR: integrity check failed`, converged anyway, and exited 0.

  Prompting and integrity are now separate concerns. `--yes` means "do not
  prompt" and nothing else; an integrity failure exits non-zero with or without
  it, and no host is touched. No apply flag lifts the gate — the check takes no
  override argument at all, so there is nothing to reach for.

  The refusal names the recovery instead: restore the state
  (`forjar snapshot restore` / `forjar generation`), or bless a known-good lock
  with `forjar reseal --all`, which was already the documented command for a
  lock and sidecar that have legitimately diverged. That path is now covered by
  a test, so the advice cannot rot into a gate with no way out. A replacement
  `--ignore-state-integrity` flag was considered and rejected: it asserts
  nothing about the state it waves through, and it is the kind of flag a CI job
  acquires permanently after one bad night — which is how this gate was lost the
  first time. It would also buy nothing for a corrupt-YAML lock, which fails to
  parse at load whether or not the check ran.

- **The published `install.sh` exited 127 on `--help`, and `dist --verify` said
  PASS.** The generator emitted the argument parser *above* the block defining
  `info`/`warn`/`die`/`usage`, so the very first thing a user ran hit
  `usage: not found`, and every error path before the definitions printed
  `die: not found` instead of its message. Generator output, the committed
  file, and the copy served from raw.githubusercontent.com were byte-identical,
  so this was live.

  `sh -n` cannot catch this by construction: a forward call is valid POSIX
  syntax that fails only at runtime. Every existing guard was a text or byte
  assertion, so nothing ever executed the script. `dist --verify` now **runs**
  it — `sh install.sh --help` must exit 0 and print usage, and an unknown flag
  must reach `die()` with exit 1 — inside a PATH sandbox whose first entry
  shims curl/wget/sudo/tar/install/cp/chmod/mktemp to refuse, so verifying can
  never install anything.


### Added

- **`nas_archive` resource type** ([#282](https://github.com/paiml/forjar/issues/282)).
  Disk *reclaim* has been a first-class resource since FJ-036; disk *archival* —
  its mirror, and the operation that **deletes originals** — was a shell script
  with its policy in a string literal. Five directories on lambda-labs were never
  archived because their names were not typed into that string, invisible to
  `plan` and `drift` because "a file exists with this sha" was perfectly
  converged over a wrong list.

  The type forbids the dangerous shapes: a destination inside or equal to the
  source (the move-onto-itself shape), an empty dir list, and a path where a
  directory name belongs. Containment is component-wise, so `/mnt/unas-old`
  beside `/mnt/unas` is still accepted.

  Six defects found reviewing the predecessor script are encoded once instead of
  per-copy: the verify fails **closed**, caches are dropped before comparing,
  inventories are compared as well as rsync output, `lsof` cannot abort the run,
  the source inventory is captured before the copy, and CIFS-hostile trees are
  refused.

  Admission is measured in **bytes held in small files**, not file count and not
  the small-file share of the count. Both proxies were tried and both misfire:
  `/home/noah/data/courses` is 755 G in 7,426 files, 46% under 64 KiB — but
  23.4 MB in small files against 754.9 GB in large ones, so the small files cost
  ~3 seconds inside a ~36-minute move. A file-count ceiling refused it outright;
  a 50% share threshold passed it for the wrong reason.

  `apply` installs the script and arms the timer; it does not move data. Running
  the script by hand is a dry run unless `ARCHIVE_EXECUTE=1`.

  Contract: `contracts/nas-archive-v1.yaml` — 6 equations, 14 proof obligations,
  14 falsification tests, 3 Kani harnesses, quorum-validated against git-annex,
  Nix gc roots and Terraform plan/apply.

### Fixed

- **forjar's own `if` collided with the condition it was given**
  ([#281](https://github.com/paiml/forjar/issues/281)). A `completion_check`
  written as a YAML folded scalar (`>-`) arrives collapsed onto one line, and
  `verdict::assert_that` inlined it into `if {condition}; then`, putting
  forjar's `if`/`then` on the same line as the condition's `do`/`done`. bashrs
  SC2136 and SC2135 read that as a malformed `if`; both are `SC2*`, which
  `validate_script` does not filter, and both are Error severity — so
  `forjar apply` aborted the resource and JIDOKA cascaded to its eight
  dependents, on a check whose source contains no `if` at all. The condition now
  gets a line of its own, verified running under both `dash` and `bash`.

- **An I8 rejection never showed what it rejected** (also #281). The sanitised
  script was built, linted and dropped, so the error named a rule and a line
  number for text nobody could see — and the script is *generated*, then
  rewritten again by `strip_data_payloads`. The rejection now prints that exact
  text, line-numbered.

### Changed

- bashrs 6.66.3 → **6.68.0**. 6.66.3 raised two SEC010 false positives on
  forjar's own generated scripts, so the I8 gate rejected output forjar
  produced. 6.68.0 also fixes SC1028/SC2104/SC1078 quote-blindness
  ([paiml/bashrs#243](https://github.com/paiml/bashrs/issues/243), #244, #245).
  Related: [#285](https://github.com/paiml/forjar/issues/285) — with 6.68.0
  linked, `purifier.rs`'s blanket `SC1*` exclusion can likely be retired.

## [1.15.0] - 2026-08-19

Same failure shape as 1.14.0 — **reporting a result that was never measured** —
found this time from the consuming side, during a live fleet outage.

Note: 1.14.0 was prepared but never published to crates.io, so this release
also carries everything listed under 1.14.0 below.

### Fixed

- **A check that could not run reported the config as clean.**
  `check_unknown_fields` re-parses raw YAML as a second pass and swallowed a
  parse failure as `Vec::new()` — indistinguishable from a clean config.
  paiml/infra's `machines/lambda-labs/forjar.yaml` reported `OK: 82 resources`
  while carrying `fs_type:` (the field is `fstype`), and the identical typo in
  a sibling config was correctly rejected. Same binary, opposite answers. The
  cause was a duplicate resource key 1200 lines away, which disabled
  unknown-field checking for the entire file.

- **Unresolved secret placeholders were emitted as credentials.**
  `resolve_or_fallback` returns the unresolved resource by design; nothing
  bounded the consequence. A `file` with `content: "API_KEY={{secrets.NAME}}"`
  wrote that literal string to disk. Guarded at codegen dispatch, so one
  unresolvable secret fails one resource rather than the whole machine.

- **A multi-line `completion_check` emitted invalid bash.** A YAML `|` block
  scalar keeps its trailing newline, so the appended `; }; then` landed on its
  own line — an empty statement, `syntax error near unexpected token ';'`.
  Fourteen resources on one host failed identically on a single apply.

- **The cargo check trusted cargo's install record, not the binary.**
  `cargo install --list` is a *record*; it is a different file from the
  binaries and nothing keeps them in agreement. After a CI cache-prune emptied
  a shared `~/.cargo/bin`, `apply -t stack-tools` reported 5 of 5 resources
  converged on a host with no rustup, no cargo and no rustc. The record is now
  used only for *which binaries a crate owns*; each one is then run.
  Includes a cargo-subcommand fallback (`cargo-X X --version`), without which
  `cargo-mutants` and `cargo-llvm-cov` are reported missing forever, and the
  version is matched against what the **binary** reports — a path-installed
  crate records its source dir in the header, and a record can be stale while
  the binary is correct.

- **`--refresh` was a dead flag.** Documented as "Re-run check scripts, only
  re-apply what fails" and read by nothing in production code. It returned in
  0.1s without contacting the host and reported `1 unchanged` over a binary
  that had been reduced to a dangling symlink. This is why a broken host could
  report converged: `apply` compares config to its lock and never looks at the
  machine. It now runs each in-scope resource's check on its host and evicts
  failing lock entries. Plain `apply` deliberately stays lock-based.

- **Repair could not overwrite a dangling symlink.** `cp` refuses
  ("not writing through dangling symlink"), and so does `cp -f` — which is the
  exact state a cache-prune leaves behind, so the one thing most needing
  repair was the one thing that could not be repaired. Placement now uses
  `install`, which is also portable to macOS unlike `cp --remove-destination`.

### Security

- **h2 0.4.13 → 0.4.16** for RUSTSEC-2026-0258 (unbounded empty DATA frames;
  unbounded memory growth or a panic on overflow). Transitive via
  hyper/reqwest; lockfile-only.

## [1.14.0] - 2026-08-17

A release about one failure shape: **a check that reported confidently on
something it had not measured.** Sixteen issues, all of that family.

### Fixed

- **`composite_hash` was not injective** (#235). NUL-separated components with
  no length framing collide: `["a\0b"]` and `["a", "b"]` hashed identically.
  This is the store's address function, so two different derivations could
  claim one store path. Now domain-separated and length-prefixed.

- **`prove` reported hash-determinism PASS for non-deterministic builds**
  (#248). It compared one pure function against itself — a tautology that
  could not fail. It now checks the codegen phases that can actually differ.

- **`execute_sync` reported the planned replay count as the executed count**
  (#249). A partial replay reported as complete.

- **Purity was reported but never enforced** (#241), and its monotonicity
  invariant was dead in production because `dep_levels` was always empty.

- **The store root was a hardcoded root-owned const** (#239) — unprivileged
  users could not use the store at all. Now resolved with a writability probe
  and a `FORJAR_STORE` override.

- **A task reported converged without reaching its declared state** (#254).
  `completion_check` gated whether the command *ran* and was never
  re-evaluated after, so "converged" meant "the command exited 0". On
  paiml/infra's `lean-toolchain`, `sudo: true` made `$HOME=/root`; every
  command succeeded, forjar reported `1 converged, 0 failed`, and
  `command -v lean` failed immediately afterwards.

- **The cargo package check asked the PATH, not cargo** (#257). `command -v`
  finds a binary whose name matches; it cannot tell you the crate is
  installed. Now `cargo install --list`.

- **`apply -r` prompted with a count it would not act on** (#253). The
  confirmation counted the *unscoped* plan: `plan -r X` promised 1 while apply
  offered 69. Execution was correctly scoped throughout — apply acted on 1 —
  but the prompt is the number an operator approves on, and
  `plan-apply-equivalence-v1` obliges the two to agree.

- **Two container tests asserted a wall-clock budget** (#259) — 4s in
  isolation, over 30s under the full suite, same commit. They measured how
  busy the machine was. They now assert dispatch provenance.

- **Five contracts had never validated** (#251), for two layers of reasons:
  proof-obligation types outside the schema vocabulary, then a flat
  `enforcement` block where the schema wants named rule structs, a unit
  smuggled into a u32 `bound`, and an unknown kani `strategy`.

### Added

- **A named library surface** (#240, #245) — `forjar::api` re-exports the 7
  functions and 4 types a consumer needs for content-hash staleness, instead
  of 1844 public items across 195 modules with no supported subset.

- **A CI job that runs the proofs, and proofs that can actually run** (#242).
  Kani harnesses and Lean proofs sat in the tree, cited by name as evidence in
  `contracts/*.yaml`, and nothing executed them — they did not even compile.
  Fixing the compilation only revealed the real state: of 21 harnesses, 7
  failed, 2 were never reached, and 2 were intractable (117 min, and 48 GB of
  RSS at 48 min).

  Two root causes, both now rules rather than one-off fixes:

  1. **Symbolic input reaching allocating code.** `format!` on a validator's
     error path drags `core::fmt` into the model, and CBMC models every path —
     not merely the one the property asserts on. Validators a harness drives
     now return a verdict (`classify_remote`, `hysteresis_holds`) and render
     the message in the caller. 117 min → 104 s; 48 min → 35 s.
  2. **A model checker cannot verify through a cryptographic hash.** Measured:
     blake3's default build fails outright (`foreign "C" function syscall`),
     and its portable `pure` build reached 29.1 GB of RSS still running at 36
     minutes. Nine harnesses reached a hash. Three were tautologies about the
     `blake3` crate; the rest are discharged executably, two of them by tests
     written to replace them because they had no coverage at all.

  A contract citing a proof that does not exist is the same defect one level
  up, so `every_harness_a_contract_names_actually_exists` now guards it — and
  found two pre-existing cases on its first run, where Lean theorems were
  declared under `kani_harnesses:`.

  Result: **20 harnesses, 20 verified, 0 failed.**

- `LICENSE-APACHE`, which `Cargo.toml` had claimed for some time (#243).

### Changed

- The README no longer claims "Pure Rust with zero C dependencies" (#238);
  there are 8 C-backed crates in the default tree.

- `dist-output/` is removed and gitignored (#256). Release generates into
  `/tmp/dist-output`; the copy committed at the repo root had drifted from the
  maintained `install.sh` and carried all 7 of the repo's bashrs SEC findings.

### Known limitations

Six issues remain open and are **features, not defects** — deliberately not
rushed into this release:

| # | |
|---|---|
| #236 | store has no output-content digest (no dedup, no corruption detection) |
| #244 | undeclared task inputs are undetectable |
| #246 | byte-identity is the only equivalence predicate |
| #247 | no regenerate-and-compare verification mode |
| #237 | no default feature set — `default-features = false` is a no-op |
| #228 | registry push still shells out to curl |


## [1.13.2] - 2026-08-16

### Added

- **`forjar dogfood` — exercise generated artifacts against reality** (FJ-038).

  Three releases in two days each fixed the previous one, and every one had
  passed 12,904 unit tests, a five-gate clean room and a 19-check CI run. The
  common cause was not missing tests: it was that the fixtures and the code
  shared an author, so each fixture confirmed the assumption it was meant to
  test. The rclone stub emitted whichever status characters the author believed
  in. The cargo fixture carried both marker files because the author believed
  both were present.

  **A test you author cannot falsify a premise you hold.** `forjar dogfood`
  invokes the real external tool and builds the on-disk shapes that really
  occur, then asserts reality agrees with what the code assumes:

  - `backup_sync` runs `rclone check --combined` against a four-case fixture and
    confirms `= * + -` mean what the coverage counters assume;
  - `disk_budget` builds all four real cargo layouts — repo target root, per-arch
    subdirectory, cargo registry, and a `cc`-style source dir named `target` —
    and confirms the first two are detected and the last two are not;
  - `file` and `cron` execute their emitted shell under **bash**, the interpreter
    every forjar transport actually uses.

  A missing external tool is a FAILURE, not a skip: dogfooding a resource built
  on a tool's output format, without that tool, proves nothing.

  Coverage is declared by an **exhaustive match** over `ResourceType` with no
  wildcard arm, so a new resource type fails to compile until its dogfood status
  is stated, and `NotApplicable` requires a written reason that is printed on
  every run. The previous `scripts/dogfood-use.sh` covered only `file` resources
  and still reported success while two new resource types shipped broken; a gate
  that can silently stop covering things is worse than none, because it reports
  GO with authority.

  Verified by mutation — reintroducing each shipped bug turns the gate RED:

  ```
  both-markers cargo rule (1.13.1) -> FAIL disk_budget: repo target root NOT detected
  inverted rclone +/-     (1.13.0) -> FAIL backup_sync: counter keyed on wrong character
  ```

- **`forjar codegen -r <resource> --phase apply|check|state-query`** — emit the
  shell a resource generates, resolved as `apply` would resolve it. A resource
  whose real payload is synthesised shell cannot be dogfooded, or debugged, if
  the artifact cannot be got at.

### Fixed

- **`disk_budget` matched no cargo target directory on a real fleet machine.**
  Detection required BOTH `CACHEDIR.TAG` and `.rustc_info.json`. Measured on
  lambda-labs across a 4.6 TB `targets/` tree: **zero of 16** marker-bearing
  directories carried the pair.

  ```
  targets/<repo>                 .rustc_info.json,  NO CACHEDIR.TAG
  targets/<repo>/<arch-triple>   CACHEDIR.TAG,      NO .rustc_info.json
  ```

  cargo writes `.rustc_info.json` at the target root and `CACHEDIR.TAG` in the
  per-arch subdirectories, so the conjunction is satisfied by neither. The
  reaper triggered at 94% used, enumerated nothing, reclaimed 0 bytes and
  reported `health=inert` — the exact silent-inertness the resource exists to
  prevent, reached by a different route.

  A directory is now a cargo target dir when it has `.rustc_info.json`
  (definitive), **or** `CACHEDIR.TAG` together with a `debug/` or `release/`
  subdirectory. That second clause is what still keeps the reaper out of
  `~/.cargo/registry`, which carries `CACHEDIR.TAG`, lacks `.rustc_info.json`,
  and whose children are `src/`, `cache/`, `index/` rather than build output.

  Both directions are pinned by falsification tests built from the measured
  layouts, and both were verified to turn RED under mutation: restoring the
  conjunction fails the per-arch test, and dropping the build-output
  requirement fails the registry-protection test.


## [1.13.1] - 2026-08-15

### Fixed

- **`backup_sync` read rclone's `--combined` status characters backwards, and
  the error inflated coverage.** They are not the intuitive way round:

  ```
  + path   missing on the DESTINATION  -> present locally, NOT backed up
  - path   missing on the SOURCE       -> only in the remote (stale)
  ```

  1.13.0 counted `-` as "missing from the remote". Files that were genuinely
  not backed up produce `+`, which nothing counted — so they fell out of the
  coverage denominator entirely and a backup **missing data reported higher
  coverage than one that had all of it**. That is the precise class of
  overstated-health failure the resource exists to prevent, shipped inside it.

  `+` now feeds the missing count. `-` is tracked separately as
  `stale_in_remote` and deliberately excluded from the denominator: a file
  present only in the remote means the local copy was deleted, not that
  anything is unprotected, and counting it would make every local deletion read
  as a backup fault until the next sync.

  Caught by running `rclone check --combined` against a real fixture rather
  than trusting the flag's name. The regression test now asserts the mapping
  explicitly, and the falsification stub emits rclone's real characters.

- **`a_missing_rclone_binary_stops_the_run` was host-dependent.** It relied on
  rclone not being installed on the machine running the tests, and broke the
  moment rclone was deployed. It now builds a hermetic PATH containing only the
  utilities the preflight needs.


## [1.13.0] - 2026-08-15

Two new resource types, both born from the same failure mode on one machine:
a guard that was deployed, enabled, reporting success, and doing nothing.

### Added

- **`disk_budget` — free space as declared machine state** (FJ-036).

  lambda-labs reached 100% on `/` (1.2 G free) while a reaper ran nightly on
  schedule and exited 0 every time. Over the preceding month it reclaimed 1.6 G
  total, across a slide from 370 G free to 1.2 G and through an earlier
  100%-full event. It was deployed, enabled, and `systemctl` reported it active
  throughout. Three independent defects, each individually sufficient:

  - a fixed 7-day idle TTL on a box whose build trees turn over in two days, so
    every candidate was legitimately "recent" and it correctly declined to
    delete anything, all the way to full;
  - build directories matched by **name** (`target|target-local|target-private`),
    so the 189 G living in `.target` was never even enumerated;
  - it never read `df`, so a run that reclaimed nothing at 100% pressure was
    indistinguishable from a healthy no-op.

  A `disk_budget` declares watermarks per filesystem. A high watermark triggers
  a reclaim pass; the pass runs until a low watermark is restored, oldest-first,
  and halts there rather than exhausting its candidates. The two thresholds
  cannot be collapsed into one — hysteresis is enforced at parse time, because a
  pass that stops while still above its trigger re-fires on every tick.

  Candidates are found **behaviourally, never by name**: cargo build directories
  by the markers cargo itself writes (`CACHEDIR.TAG` *and* `.rustc_info.json`),
  git worktrees by asking git. Requiring both cargo markers is what keeps the
  reaper out of `~/.cargo/registry`, which carries the tag and not the info file.

  Crucially, **a triggered pass that misses its target exits non-zero**, so an
  inert reaper becomes a failed unit and is visible to `forjar drift` instead of
  silently green. `state_query` publishes health *classes* on stdout and raw
  byte counts on stderr, so volatile values never enter the drift hash.

- **`backup_sync` — an offsite copy that must prove it exists** (FJ-037).

  The same machine held ~2.1 TB of irreplaceable media on a 4-wide RAID0 with no
  parity, and zero bytes of it anywhere off that array, while an hourly job
  reported `Backup complete` for months. It rsynced a directory to a symlink
  pointing back at that same directory, and its success metric ran `find` on
  that symlink without `-L` — printing `Files: 0` while 77 matching files sat
  there. Structurally zero on every input, not merely on an empty one.

  `backup_sync` rejects a destination that is not an rclone `remote:path`, and
  proves at runtime that the remote is *configured and reachable* before a byte
  moves — an unconfigured rclone remote silently degrades to a local path, which
  is the same self-referential failure by another route.

  Health is a count of files verified present in the remote **by checksum**
  (`rclone check --combined`), compared against the source. Zero examined or zero
  matched is a failure, not a pass. A run below the declared coverage threshold
  exits non-zero.

  `apply` deliberately does *not* run the sync, unlike `disk_budget`: seeding
  terabytes takes days under provider upload caps, and a deployer that runs the
  job also writes the status file that is supposed to be evidence the *service*
  ran. `apply` arms the timer; `state_query` reads the journal, which the
  deployer cannot forge.

  forjar owns the generated `rclone.conf` so the remote definition is declared
  state rather than a manual step that can go missing. Backend and options live
  in the repo; the OAuth token arrives through the secrets provider. A literal
  token is refused at parse time, an unresolved `{{secrets.x}}` at codegen, and
  the file is written under `umask 077` at 0600 with an atomic rename.

### Fixed

- **Removing a systemd resource no longer leaves an orphaned unit.** `state:
  absent` deleted unit files while the unit was still loaded, leaving it
  `Active: failed` with *"Unit to trigger vanished"* — invisible to an apply
  that reported converged. Teardown now stops, disables, removes, reloads, and
  clears the failed state, in that order.

- **`hash_desired_state` now covers scripts a handler generates.** A resource
  whose payload is synthesised is not fully described by its declaration: two
  forjar versions can emit different scripts from identical YAML. The planner
  compared only the declaration, reported `unchanged`, and left machines running
  the previous generated artifact. All three emitted scripts are hashed — folding
  in only `apply` would pin `apply`=unchanged against `drift`=drifted forever,
  with nothing re-recording state.

- **`RESOURCE_FIELDS` completeness is now enforced by reflection.** A field added
  to `Resource` but missing from the parser's hand-maintained allow-list was
  accepted by serde and then rejected by validation as `unknown field`, so a
  fully-implemented, fully-tested feature was undeclarable in YAML. A test walks
  the serialised struct, so the next omission fails a test instead of shipping.

- Templated values in reclaim roots and backup sources are resolved. An
  unexpanded `{{params.home}}` silently matches nothing, which for a reaper or a
  backup means "protects nothing" while reporting success.

### Notes for handler authors

Both handlers ship a test that runs every emitted script through
`purifier::validate_script` — the same call `forjar apply` makes. Its absence let
six I8 violations ship in `disk_budget`: the resource passed 12,000+ tests and
was rejected on every machine, because nothing in the suite exercised the
purification path production uses. If you add a resource type, add that test.

## [1.12.6] - 2026-08-12

### Fixed

- **`build --push` now names the missing binary instead of reporting an errno**
  ([#224](https://github.com/paiml/forjar/issues/224)). Every OCI registry
  request shells out to `curl`, an undeclared runtime dependency. On a host
  without it the first HEAD died as `curl HEAD: No such file or directory (os
  error 2)` — which names neither curl nor the fact that a required external
  binary is absent, and reads like a network or registry fault.

  A preflight at `push_image()` — the single funnel every push goes through —
  now fails early with an actionable message, before any partial upload begins.

  Found by infra's clean-room gate (a container holding only what the crate
  declares) while GitHub CI was green on the same commit, because the CI image
  happens to ship curl. A user running `cargo install forjar` on a minimal host
  would have hit the original error.

  This makes the dependency legible; it does not remove it. Removing it is
  #228, done in Unreleased. (The suggestion recorded here — "the crate already
  compiles `reqwest`" — was wrong: `reqwest` is async-first and
  `reqwest::blocking` panics when called from inside an async context, which is
  exactly what the MCP path is.)

## [1.12.5] - 2026-08-12

### Changed

- **bashrs floor raised `6.64.0` → `6.66.3`** (lockfile moves 6.66.0 → 6.66.3).
  forjar surfaces bashrs' linter directly to users — `bashrs::linter::lint_shell`
  in the purifier, plus `forjar lint` and the MCP handlers — so bashrs' lint
  fixes are forjar's user-visible behaviour.

  This is a **floor** bump, not just a lockfile refresh, and deliberately so:
  bashrs 6.66.3 retires MAKE016, whose autofix **corrupted Makefiles**. A caret
  range still admitting 6.64.0 would let a consumer resolve back into a
  Makefile-corrupting autofix, which matters because v1.12's headline feature is
  Makefile ingest.

  What forjar users gain: no more spurious diagnostics inside quoted heredocs
  (`<<'EOF'` bodies are literal text, not shell — provisioning scripts and
  `command:` blocks are full of them), MAKE003 no longer trips over Make's `$$`
  escape, and MAKE010 recognises `|| exit` / `|| return` / `|| die` tails as
  error handling.

  No API changes were required: the full suite passes unmodified against 6.66.3
  (12776 passed, 0 failed).

## [1.12.4] - 2026-08-11

### Fixed

- **Dogfooding sweep of the published 1.12.3 across CLI / MCP / LSP / HTTP**
  ([#208](https://github.com/paiml/forjar/issues/208)): 101 confirmed defects,
  4 of them BLOCKERs. Resolved in
  [#216](https://github.com/paiml/forjar/pull/216) together with
  [#211](https://github.com/paiml/forjar/issues/211),
  [#212](https://github.com/paiml/forjar/issues/212),
  [#213](https://github.com/paiml/forjar/issues/213),
  [#214](https://github.com/paiml/forjar/issues/214) and
  [#215](https://github.com/paiml/forjar/issues/215):

  - **[A]** 15 flags were declared on the clap struct and never consumed —
    accepted on the command line and silently ignored. Notably every one of the
    eight dry-run spellings is now ORed fail-safe, so `--dry-run` cannot be
    dropped by a code path that only checked one of them.
  - **[C]** 26 defects where machine-readable output was malformed or leaked
    Rust `Debug` formatting into what callers parse as JSON.
  - **[D]** 5 selectors that filtered part of the output but not the rest,
    leaving precomputed counters disagreeing with the rows they summarise.
  - **[E]** 8 cases where invalid input was accepted or crashed instead of
    being rejected — including workspace names containing `..`, which are now
    refused before any filesystem access rather than after.
  - **[Z]** 19 assorted correctness defects, including MCP state-directory
    resolution, which resolved relative to the process working directory
    instead of the config file's directory.

  These were found by `cargo install`-ing the published crate and driving every
  interface, not by the test suite: two MCP handlers had been wrong since they
  were written and had passing tests that only asserted `Ok`.


- **`build --push` fabricated a push** ([#210](https://github.com/paiml/forjar/issues/210)).

  With a network it printed `Push complete: 3 uploaded` and exited 0 having
  uploaded nothing; with no network at all it printed
  `push skipped: registry unreachable` and **still exited 0**. The target was
  `docker.io/app:latest` whatever the resource declared:

  ```console
  $ unshare -rn forjar build --resource img --push   # no network whatsoever
    registry: docker.io
    name: app
    tag: latest
    push skipped: registry unreachable (no Location header in upload response)
  $ echo $?
  0
  ```

  Five defects, each sufficient on its own to make the success line false:

  1. Transport failures were swallowed into a "skipped" line and `Ok(())`.
  2. The push target was re-derived from `name`/`version` with a different
     default (`app`/`latest`) than the build used, and split at the first `/`,
     so `myorg/app` parsed as registry `myorg`. `tag:` on an image resource was
     parsed and dropped entirely. The push now reuses the exact reference the
     build stamped into the image, parsed by
     `core::store::image_ref::parse_image_ref`.
  3. Success was gated on the presence of a `Location:` header. `docker.io` is
     a website: it answers the upload POST with a 301 to the marketing site,
     whose `Location` was taken as an upload session — the blob was PUT at a
     web page, which returned 200. The status code is now the gate (202
     Accepted), and Docker Hub resolves to `registry-1.docker.io`.
  4. The manifest was uploaded as a *blob* and never PUT to the tag, so even a
     fully successful run created no pullable tag. It is now PUT to the tag.
  5. `?digest=` was concatenated onto session URLs that already carry a query
     string (`?_state=…`), which every real registry rejects with
     `BLOB_UPLOAD_INVALID`.

  `Push complete` is now printed only after forjar re-reads the tag from the
  registry and confirms it resolves to the manifest just pushed; every other
  outcome is a non-zero exit. HTTP 401 is reported as what it is — forjar
  implements no registry credentials, so an authenticated registry is refused
  with a pointer to `--load` + `docker push` or `--far`. Anonymous-write
  registries push and verify for real (`docker pull` of the pushed digest
  succeeds).


## [1.12.3] - 2026-08-10

### Fixed

- **`apply` deployed stale content while reporting "unchanged"** when a file
  resource's `source:` file changed ([#206](https://github.com/paiml/forjar/issues/206)).

  `hash_desired_state` hashes resource *field strings*. For `content:` that is
  correct — the content **is** the field. For `source:` the field is a **path**, so
  editing the referenced file left the hash identical, `determine_present_action`
  planned `NoOp`, and apply skipped the resource:

  ```console
  $ echo VERSION-ONE > payload.txt && forjar apply -f repro.yaml --yes
  Apply complete: 1 converged, 0 unchanged.
  $ echo VERSION-TWO > payload.txt && forjar apply -f repro.yaml --yes
  Apply complete: 0 converged, 1 unchanged.
  $ cat /tmp/deployed          # -> VERSION-ONE   (stale)
  ```

  `--force` was the only workaround. For a tool whose contract is "converge to
  declared state", silently not converging while printing success is the worst
  available failure mode. Observed live in paiml/infra PMAT-204, where an edited
  reconciler script reported "converged" three times while the machine kept
  executing the previous copy.

  The planner now folds the **content hash** of the `source:` file into the
  desired state. The component is **appended**, and only for resources that
  declare `source:`, so no recorded hash for any other resource on any machine is
  invalidated. Path identity is preserved (same bytes at different paths still
  hash differently), and a source that appears or disappears now changes the hash
  rather than staying pinned at "unchanged".

  Note: source-based file resources will show one `Update` on the first apply
  after upgrading, as their hash gains the new component. That re-apply is
  convergent and expected.

### Added

- `contracts/source-content-identity-v1.yaml` — the **completeness** leg of
  `idempotent-apply-v1`. That contract asserts "differing hash always plans
  Update", which is sound but vacuous if the hash cannot differ when the deployed
  artifact differs. 7 falsification tests, including an end-to-end reproduction.
- `src/core/planner/tests_hash_source.rs` — 5 tests covering content change,
  determinism, path identity, source appearance, and non-regression of
  source-less resources.

## [1.12.2] - 2026-07-29

Four design defects, each one a place where forjar reported on something other
than the thing that would actually run. They were found by a design review of
1.12.1 rather than by a failing test, because in every case the test suite and
the CLI output agreed with each other and both were wrong.

### Fixed

- **`apply` treated a zero exit code as proof the work happened.** A task
  declaring `output_artifacts` whose command exited 0 without producing them was
  recorded as converged, and the state lock then asserted an artifact that was
  not on disk. Apply now verifies declared outputs exist on local machines
  before recording success, and names the missing ones when they don't.
- **The script was readable as its own stdin.** The transport feeds the
  generated script to `bash` on stdin, so a task command that itself reads stdin
  (`cat > f`, `read`, `xargs`) consumed the remaining script lines. The task
  half-executed and reported success, having silently eaten its own tail. Every
  script is now wrapped `{ ... } < /dev/null` at the one point where scripts are
  executed, so a command's stdin can never be the script.
- **`forjar prove` proved the unresolved config.** It read `config.resources`
  before template expansion, so a proof about conflict-freedom examined
  `${var}/out` rather than the path two targets would really write. Two targets
  that genuinely collide after expansion were reported as
  `[PASS] I3 conflict-freedom: [CHECKED] 2 targets disjoint`. `prove` now
  resolves first.
- **`forjar lock` hashed the unresolved config.** The lockfile therefore
  fingerprinted the template text rather than the values, so two configs that
  expand to different infrastructure could share a hash. `lock` now resolves
  before hashing.

## [1.12.1] - 2026-07-28

Three interface defects found by installing 1.12.0 from crates.io and driving
every CLI, MCP and LSP surface against a real project. None was visible from a
schema, a `tools/list`, or a handler test that only asserted "returns Ok".

### Fixed

- **MCP `forjar_plan` reported every resource as a pending change.**
  `ExecutionPlan::changes` carries EVERY resource with its action, NoOp
  included; `cli::plan` filters those before counting and the MCP handler did
  not. A fully converged project reported all 6 of its resources as pending
  while the CLI reported `0 to change`. It also included phony resources, which
  are goal-only.
- **MCP `forjar_status` returned no machines, ever.** It scanned the state
  directory for files with a `.json` extension; a machine's state is a
  DIRECTORY, `state/<machine>/state.lock.yaml`. The CLI printed
  `Machine: local (localhost)` for the same project.
- **`forjar lsp` was an unrecognized subcommand.** The language server is
  complete and has 80 passing tests, but no `Commands` variant dispatched it,
  so no editor could start it. Now wired and documented.

### Known

`core::webhook_server` implements an HTTP endpoint with 16 tests including live
socket accept/reject, but nothing starts it — there is no `forjar webhook`
command. Exposing a listening socket needs bind-address and authentication
decisions, so it is a design question rather than a wiring fix; tracked in
PMAT-200 rather than added unilaterally in a patch release.

## [1.12.0] - 2026-07-28

forjar can now **replace and ingest** a trivial Makefile. Getting there required
fixing five defects, four of which shared one shape: a signal that reported
success without ever consulting the world.

### Added

**`forjar make [GOALS...]`** — builds each goal and its transitive
prerequisites, and nothing else. `resolver::goal_closure` walks `depends_on`
upward; the result is downward-closed by construction, so a pruned config can
never execute against an unconverged prerequisite. That is the property that
makes goal selection safe where `--subset`/`--exclude` pattern filters are not,
and it is what `-r` never had: `-r` is exact-match with NO closure, so
`apply -r link` runs link and silently skips the compile step it depends on,
linking whatever objects happen to be on disk. That is `make -o`, not `make`.

**`phony: true`** — a make-style target that names an ACTION, not a file. It is
excluded from bulk apply and plan entirely, and runs unconditionally when named
as a goal. Goal-only is the only reading that preserves idempotency: "runs on
every apply" would propagate dirtiness through its whole transitive closure and
stop `plan` ever reaching "0 to change"; "phony prerequisites auto-run when
reached" is not convergent, because a `clean` that `build` depends on deletes
the outputs that make `build` stale, forever.

**`forjar import-makefile`** — ingests a single-makefile, non-recursive build by
joining two streams from one `make` invocation: `-p` gives structure with
UNEXPANDED recipes, `--trace` gives the expanded commands. The join key is
`(recipe file, recipe line, target)` — the target name is load-bearing, because
`build/main.o` and `build/util.o` both trace as `Makefile:14` when they share a
pattern rule.

Two measured hazards shaped the invocation more than the parser. An up-to-date
tree emits NO commands (`Nothing to be done`), so `-B` is mandatory or the
import yields structure with no commands for exactly the targets that matter.
And pattern rules only instantiate during goal resolution, so import is two
passes: enumerate names, then ask for them all by name.

Recipes emit one **subshell per logical line** (after folding backslash
continuations), reproducing make's per-line shell isolation exactly — so
`cd build && ./app`, an idiom far too common to refuse, imports faithfully.
Order-only prerequisites become `depends_on` edges and never `task_inputs`;
hashing a directory as an input is what made 1.11.0 an idempotency pump.

Recursive make, `.ONESHELL`, double-colon rules, VPATH and GNU make < 4.0 are
**refused with reasons, writing nothing**. An importer that silently
mistranslates is worse than none: its output looks like your build and is not
one.

### Fixed

**`forjar check` reported `pass` for every resource, unconditionally** — for
every resource type, since at least 2026-02-27. Verified on the published
1.11.1 binary against a config that had never been applied, in an empty
directory: `2 pass, 0 fail, exit 0`. The cause was a protocol mismatch, not a
missing comparison: generators emitted their verdict as a stdout marker
(`<test> && echo exists || echo missing` — a branch whose arms are both `echo`
always exits 0) while the consumer read the exit code, and nothing anywhere
parsed the markers. `apply --check` shares the path, so its documented
"exit 2 = changes needed" was unreachable too.

The fix cannot live at the codegen boundary: the same marker means opposite
things depending on desired state — for `state: absent`, `missing:` IS
convergence. All 17 generators now report through `resources::verdict`. Also
corrected while converting: `service` asserted a fixed "active AND enabled", so
a `state: stopped` service would have become a permanent failure; gpu's rocm
path did `echo missing; exit 0`; a model checksum MISMATCH reported pass — the
most dangerous case, since the file exists and looks fine.

**24 templatable `Resource` fields were never resolved**, including
`task_inputs` — the field 1.11's entire incremental-build release is about,
while its sibling `output_artifacts` was resolved. A config that templated its
inputs got `Apply complete: 0 converged, 1 unchanged` over a stale artifact:
precisely the failure 1.11 shipped to eliminate. Also `scatter`/`gather` (spliced
into executed shell), `state` (selects the absent/directory/symlink branch), the
six `overlay_*` fields that configure the fleet's overlay IPs, and `stages`.

**`forjar destroy` executed unresolved templates** — generating
`rm -rf '{{params.x}}/...'` against a literal path, reporting success while the
real resource survived and its lock entry was removed. Third code path to make
this mistake. **And it RAN builds instead of removing them**: task, build and
wasm_bundle ignore `state`, so converging them to `absent` executed the command
— running a build or a deploy as the way of "removing" it.

**A selector matching nothing is now an error.** `apply -r <typo>` printed
`0 converged, 0 unchanged` and exited 0; in CI, where the exit code is often the
only signal read, a typo'd targeted apply looked like a completed deploy.

### Guards against recurrence

Both guards are constructed so that a future addition is covered without anyone
remembering to update them:

- `no_resource_type_generates_an_unfailable_check_script` EXECUTES each
  generated script against a real filesystem. Asserting on script TEXT is what
  let the check defect live for months — the text was always plausible.
- `every_string_field_on_resource_is_template_resolved` REFLECTS over the
  serialised `Resource` to discover which fields accept a string. A hand-written
  list of fields to check has the same failure mode as the hand-written list of
  fields to resolve.

Contract: `contracts/build-semantics-v1.yaml` (L3, 9 falsification tests).

### Known differences from make

- **Shell options.** forjar wraps a `command:` in `set -euo pipefail`; make sets
  none. Imported recipes restore make's semantics per line. An earlier draft of
  this entry called the difference "strictly stricter — it surfaces errors make
  swallows"; dogfooding the built binary falsified that. Under pipefail
  `seq 1 100000 | head -1` exits 141 on SIGPIPE where make returns 0, and
  `cmd | head` is a stock Makefile idiom, so the claim described a working build
  becoming a failing one as a safety improvement. A hand-written forjar
  `command:` still runs under `set -euo pipefail`.
- A `-` prefixed recipe line (ignore errors) imports as `... || true`. `--trace`
  strips the prefix, so it is read from the make database instead.
- A real file target that depends on a `.PHONY` target is **refused**: make runs
  the phony prerequisite when it reaches it, goal-only phony cannot, and the
  imported config would build without the action ever running.
- `include`, `$(shell …)` and `$(wildcard …)` are resolved by make before the
  import sees them, so their values are frozen at import time.
- Staleness is BLAKE3 content, not mtime: `touch` does not trigger a rebuild,
  and recompiling to identical bytes correctly does not relink.
- The staleness probe runs on the controller and skips remote resources rather
  than hashing the wrong host, so `forjar make` is a build system for LOCAL
  targets.

### What this release does NOT fix

v1.12 does not make the three read paths agree; it makes each one honest about
the level it observes. `check` is existence/state level — `rm build/demo` now
fails it, but editing a file's content in place does not. `plan` is config-hash
plus the build probe. `drift` remains the only content-level comparison.
Measured: tamper with a file resource's content and check says `1 pass`, plan
says `0 to change`, drift says `Drift detected`. Raising check to content level
means every generator hashing its artifact — a feature, not a bug fix.

Also outstanding, now recorded as `known_gaps` in the contract rather than left
implicit: the transport writes the script to bash's stdin, so a task that reads
stdin consumes the rest of its own script (pre-existing); the bashrs determinism
gate rejects idioms like `date +%s` at apply time, so such a Makefile imports
cleanly and then cannot run; and order-only edges participate in propagation
because there is no `order_only` field.

## [1.11.1] - 2026-07-27

### Fixed — two defects in 1.11.0, found by dogfooding it

**Directory `output_artifacts` created an idempotency pump.** Hashing a
directory's *contents* meant the canonical translation of make's `| build`
order-only prerequisite — `output_artifacts: ["build"]` — went stale the moment
the next rule wrote into it. Observed: apply #1 converged, apply #2 reported
`stale — output artifact modified` and **re-ran the entire graph**, and only
apply #3 settled. That violates `f(f(x)) = f(x)`, forjar's core idempotency
contract.

A directory artifact is now identified by **existence**, never by contents. Its
contents are the products of *other* rules; they are not the identity of the
rule that created it. Files declared alongside a directory are still hashed.

**The read paths were blind to the staleness `apply` acts on.** `planner::plan`
forwarded an EMPTY probe map, so after `rm build/demo`:

```
forjar plan  -> Plan: 0 to add, 0 to change, 0 to destroy, 3 unchanged
forjar check -> Check: 3 pass, 0 fail, 0 skip
forjar drift -> No drift detected
forjar apply -> stale — output artifact missing ... rebuilt
```

A planner that cannot predict its own apply is worse than one that is merely
conservative. `plan` now probes, so `plan`/`drift`/`observe` and `apply` give
one answer. `plan_with_probes` remains pure for unit tests.

### Known limitation

`forjar check` still over-reports. Its generated scripts echo a marker
(`task=completed` / `task=pending`) but exit 0 either way, and the CLI grades on
process success — so `check` currently means "a shell ran", not "the resource is
converged". Fixing it touches all 14 resource generators and changes what
`check` means for non-build resources, so it is deferred rather than rushed into
a patch release.


## [1.11.0] - 2026-07-27

### Added — incremental builds: forjar now plans from the world, not just the config

Until now forjar decided what to do by hashing the *desired config*. A task whose
`task_inputs` had changed on disk hashed identically as a desired state, so the
planner returned `NoOp`, the executor never ran, and forjar printed
`Apply complete: 0 converged, N unchanged` over a **stale artifact**. That is the
worst failure mode a build tool has: a wrong binary under a green summary.

The pre-existing `cache:` / `task_inputs` machinery could not fix it.
`check_task_input_cache` ran inside `apply_one_resource`, i.e. *downstream* of a
planner that had already said `NoOp` — structurally able to *suppress* work,
never to *schedule* it.

**What changed**

- New `core::task::probe`: before planning, forjar hashes each resource's
  declared `task_inputs` and `output_artifacts` and hands the result to the
  planner. The planner stays **pure** — it never touches the filesystem.
- A converged resource is now re-run when its inputs changed, an output artifact
  is missing, or an output was modified out of band. `rm build/demo` rebuilds;
  previously it reported `unchanged`.
- **Change propagation**: a rebuilt prerequisite now invalidates its dependents.
  `depends_on` was ordering-only, so a rebuilt object file left the link step
  converged and the binary stale. One forward sweep over the topological order.
- `output_hash` is now recorded alongside `input_hash`.

**Fixed along the way**

- I/O hashing resolved paths against `state_dir.parent()`, not `working_dir`.
  Since builds declare paths relative to their project root, every relative path
  hashed as absent — which silently disabled caching whenever `--state-dir` was
  relative. Added `hash_outputs_in(artifacts, base_dir)`.
- Hash tracking no longer requires `cache: true`. Tracking is what makes
  correctness possible; `cache` remains the switch for *skipping* work.

**Verified against `make`** on a 3-target C project: clean build, true no-op
re-apply, minimal rebuild on edit (2 of 3 targets — not rebuild-everything),
artifact restore after `rm`, and identical program output to `make` across
multiple source mutations.

### Fixed — `forjar drift` reported permanent false positives on templated resources

`drift` compared **raw** config resources against state the executor had stored
from **resolved** ones, so anything containing `{{params.*}}` drifted forever.
For `type: task` the state query is `echo "command=<command>"`, so the lock held
the resolved command while the probe regenerated the literal template.

This was not cosmetic: the apply-time drift gate is **global**, so one
self-drifting resource blocked every *targeted* apply on that machine. Downstream
fleets were running `forjar apply -t <tag> --no-tripwire` because of it.

This class had already appeared twice (planner FJ-154; drift), and `destroy`
still had it — so resolution now lives in exactly one place,
`resolver::resolve_or_fallback` / `resolve_all`, rather than being re-derived per
call site.

### Security

- `cargo update -p crossbeam-epoch` 0.9.18 → 0.9.20, clearing **RUSTSEC-2026-0204**
  (invalid pointer dereference in `fmt::Pointer` for `Atomic`/`Shared` when the
  underlying pointer is invalid). Transitive via `rayon` → `sysinfo`/`bashrs`.
  Lockfile-only; pre-existing, not introduced by this release, but a release
  should not ship a known advisory.

### Known limitations

forjar is **not** a general-purpose build system and this release does not claim
to be one. Specifically:

- Staleness is **content-hash** based, not mtime. `touch` alone does not trigger
  a rebuild — timestamp-only stamp-file idioms do not carry over, and
  round-trip tests against `make` must mutate content.
- Probing is **controller-local**. Resources targeting remote machines are not
  probed and keep the previous config-hash behaviour, rather than hashing the
  wrong host's files.
- No pattern rules, no `$@`/`$<` automatic variables, no `make <goal>` target
  invocation, no `.PHONY`. Deferred deliberately.


### Changed — copia delta is now TRUE ROLLING delta (the real fix)

forjar's large-file provisioning delta was fixed-block: a 1-byte insertion made
every later block differ, forcing a full re-transfer. It now computes a **true
rolling delta** via the copia crate (`copia::CopiaSync::delta` matches blocks at
ANY byte offset), so an insertion/deletion reuses the unchanged bulk of the file.

The elegant part: **no copia binary is staged on the receiver.** The receiver
reports a `copia::Signature` computed with `od`+`awk` (the Adler weak checksum,
verified bit-for-bit against `copia::RollingChecksum`) and `b3sum` (blake3 strong
hash) — all universal tools; forjar does the rolling match locally with the library.
A checksum disagreement (e.g. busybox) can only degrade to all-literal (correct
output), never corrupt — the strong hash and the whole-file blake3 are re-verified
before the atomic rename. `copia-provisioning-v1` gains FALSIFY-COPIA-005..007.


### Security — copia provisioning hardening (copia-provisioning-v1)
Fixes 5 defects a 6-lens quorum found in the generated remote delta-sync shell
(`src/copia`): perms (chown/chmod) now run on the temp file **before** the atomic
rename (closing a world-readable window for 0600 secrets); literal payloads stream
via a base64 **heredoc** (no `echo` ARG_MAX blowup); the reconstructed file's
**blake3 is verified** before commit; every interpolated path/owner/group/mode is
**shell-quoted** (injection guard); and a **cleanup trap** prevents temp-file litter.

### Added — 33 verified Lean 4 proofs (6 contracts to L4)
forjar's first Lean proofs: `blake3-state`, `dag-ordering`, `recipe-determinism`,
`execution-safety`, `codegen-dispatch`, `overlay-interface` each gain a machine-checked
proof of their decidable core (0 `sorry`), with honest verification_summary
(l4_lean_proved for the pure core, l4_not_applicable for genuine I/O).


### Added — Provable IaC: `forjar prove` invariant ladder (provable-iac-v1)

`forjar prove` is enhanced from a convergence proof into a **pv-style L1–L5
provability report over the forjar.yaml itself** — the honest analog of
`terraform validate` with machine-checked validators (NOT "safe to apply": the
remote shell that mutates the machine is outside the trusted computing base).

- **Three-state assurance** (never collapsed): `PROVED` (a theorem transfers to
  this config), `CHECKED` (a Kani-verified decision procedure ran and found
  nothing), `UNKNOWN` (an opaque/imperative resource the analysis can't see through).
- **Structural invariants** added alongside the existing convergence proofs:
  I2 dependency-completeness, I3 conflict-freedom (target-namespace disjointness),
  I6 protected/blast-radius, I9 input-purity (advisory) — plus a stable,
  order-independent `plan-hash` (the artifact `apply` will bind to).
- **No vacuous green**: any `command`/`cron`/`script`/`recipe` resource downgrades
  the invariants it touches to `UNKNOWN` — the gate never reports safe where it
  cannot see (a real risk this fleet's exec-heavy configs surface).
- **HARD invariants** (I1/I2/I3/I6) block apply on falsification; advisory ones warn.
- Backed by `contracts/provable-iac-v1.yaml` (6 falsification tests + a Kani-verified
  conflict-detector, `provable-iac-kani-001`); quorum-validated design in
  `docs/specifications/provable-iac.md` (6 world-class lenses).


### Added — `overlay_interface` resource type (FJ-035)

A new first-class resource type that provides a DNS/DHCP-independent fleet
overlay natively, replacing the `machines/fleet-hosts` shell installer. A
machine declares `type: overlay_interface` with `overlay_ip: "10.42.0.11/24"`
and forjar idempotently binds the static secondary IP on the default-route NIC
(auto-detected, or an explicit `overlay_iface`) and installs a self-healing
systemd `service` + `timer` — plus a NetworkManager dispatcher hook where NM
owns the NIC. Optional `overlay_hosts` (managed `/etc/hosts` block) and
`overlay_firewall` (ufw allow the /24) sub-features.

The implementation reproduces the validated self-heal mechanism exactly,
including its three hard-won anti-regressions: the service is a plain
`Type=oneshot` (never `RemainAfterExit=yes`), the timer uses
`OnCalendar=minutely` (never `OnUnitActiveSec`), and apply `daemon-reload`s then
**restarts** both units so a unit-content upgrade takes effect without a reboot.
`overlay_ip` / `overlay_iface` are strictly validated and every interpolation is
shell-injection hardened.

## [1.6.2] - 2026-06-13

### Fixed — dogfood QA findings (#177, #178, #179)

The new `dogfood` QA skill (run against v1.6.1) surfaced three
low-severity issues, all fixed:

- **`forjar check` gains `--state-dir`** (#178, #182): `check` was the
  only state-reading subcommand without it (`apply`/`destroy`/`drift`/
  `status`/`undo`/`undo-destroy`/`lock-verify`/`history` all have it), so
  a stack applied to a custom state dir couldn't be checked against it.
  `check` now also reports whether each resource is recorded in state
  (`! <res> — not recorded in state`; `"in_state"` in `--json`).
- **4 stale example configs now validate** (#179, #181): `agent-deployment`,
  `dogfood-gpu-training`, `dogfood-outputs`, and `multi-agent-fleet`
  failed `forjar validate` (unknown fields, missing `sudo: true`); fixed,
  and a new `examples-validate` CI gate + `tests/examples_validate.rs`
  prevent regressions (45/45 examples now validate).
- **README C9 dependency claim corrected** (#177, #180): the falsifiable
  claim said "<20 deps (17 runtime + 1 build)" but its own verify command
  reported 30 runtime deps; restated to the accurate 30 (27 always-on +
  3 optional: age/wasmi/dhat) + 3 build, and the comparison-table cell
  updated to match.

### Added

- **`dogfood` Claude Code skill** (#176): contract-first, read-only
  exhaustive QA — rebuild+install, full command grid, prove claims
  C1–C10 and the 10 `contracts/` against a sandboxed `/tmp`-only local
  stack, safe apply→reapply→destroy→undo lifecycle, loud-failure
  injection, `dist --verify` self-test, GO/WARN/FAIL verdict.

## [1.6.1] - 2026-06-13

### Fixed — new-code audit of the v1.6.0 changes (#165)

A focused audit of the ~10.6k lines merged for v1.6.0 (the fix/feature
implementations themselves, which the original bug-hunt predated) found 9
regressions/gaps, each double-refuted; all fixed:

- **Lock-acquire livelock (regression, #169):** the v1.6.0 `O_EXCL`
  rewrite of `acquire_process_lock` could spin at 100% CPU when a
  stale-but-undeletable lock persisted (read-only mount / cross-UID
  sticky-bit dir). The loop is now bounded (5 attempts, 50ms backoff) and
  propagates a clear error instead of looping; the stale-lock reap re-reads
  and byte-matches the dead-PID content before unlinking, so it can't
  delete a concurrently-acquired fresh lock (TOCTOU); and the transport
  timeout now spawns children in their own process group and kills the
  group gated on a worker-done flag, eliminating a PID-reuse kill of an
  unrelated process.
- **Shell-escape gaps (#166):** the v1.6.0 escaping sweep missed
  `mount.rs` mounted/unmounted echo labels and `file.rs` source-read /
  unsupported-state error messages — config-derived values are now
  escaped there too.
- **Planner/executor (#167):** `moved:` collision checks now also run
  *after* recipe/resource expansion (a `to` colliding with an expanded
  recipe key was previously missed); a failing `pre_apply` hook is no
  longer re-executed under `--retry` (the v1.6.0 Skipped→Failed change had
  made it retryable) via a `retryable` flag on the failed outcome.
- **Coverage demotion (#168):** L3–L5 promotion is now recency-aware —
  a failing run at the same config hash demotes the resource (failures are
  now persisted), instead of an old passing record surviving forever.
- **Tier-2 verify (#168):** `--verify-containers` now honors a custom
  `dist.checksums` filename (the offline shim previously hardcoded
  `SHA256SUMS`, silently skipping verification for any other name).

## [1.6.0] - 2026-06-13

### Added

- **L3–L5 test-coverage persistence** (#155, FALSIFICATION-REPORT E9):
  `forjar test coverage` now reports beyond L2. Convergence/mutation/
  preservation runs append hash-stamped results to a `test-coverage.jsonl`
  log in the state dir; `cmd_test_coverage` reads them back and promotes
  each resource to the highest passing level whose `config_hash` still
  matches the resource's current desired-state hash — a changed resource
  falls back to its static L0–L2 level (no stale high-water marks). New
  `core::store::coverage_persist` + `cli::coverage_promote`.

### Security / Fixed — 28-defect deep audit (#154)

An adversarial bug-hunt (8 fault-dimension finders, every finding
double-refuted) surfaced 28 confirmed production defects; all are fixed:

- **apply no longer reports success on real failures** (#157): a failed
  `pre_apply:` hook now fails the resource (was reported `Skipped`, apply
  exited 0, dependents ran, rollback was skipped) on the sequential path;
  OCI registry push now checks HTTP status via `curl --fail-with-body`
  (401/404/413/5xx were reported as successful pushes); the coverage
  promotion gate distinguishes "llvm-cov absent" (advisory pass) from
  "llvm-cov ran and failed" (gate failure).
- **Shell-injection hardening** (#161): new `core::shell_escape::sh_squote`
  + identifier validators route every resource-handler data field through
  proper single-quote escaping. Fixes unescaped/unquoted interpolation in
  `build` (arbitrary command over ssh), `github_release` (repo/tag command
  substitution), `task`, `network` (ufw), the binary-cache rsync path, and
  the file/mount/package/cron/docker/model handlers.
- **Concurrency & resource leaks** (#159): mutation-runner sandbox dir now
  uses a process-wide atomic sequence (the #141 race, unpatched in the
  sibling); `acquire_process_lock` is now atomic via `O_EXCL` create_new
  (was a TOCTOU letting two `apply` runs both win); transport timeout now
  kills+reaps the child (was leaking a thread + orphan process); stdin-write
  failure reaps the child across all 7 transports (was leaking zombies);
  container build cleans up via RAII guards on every error path.
- **Storage integrity** (#158): OCI base-image blob paths validate the
  `sha256:<64-hex>` digest (a `..`-bearing digest allowed arbitrary
  read/write); state encryption writes atomically (temp+rename, was an
  in-place truncate that corrupted state on a mid-write crash); `lock
  defrag` refreshes the BLAKE3 `.b3` sidecar (was bricking the next apply's
  integrity check).
- **Planner/executor correctness** (#160): planner and `refresh-only` now
  resolve secrets with the same provider config the executor uses, ending
  a perpetual spurious-Update idempotency violation; the rolling-deploy
  `max_fail_percentage` gate uses integer math (was `as u8`-truncated at
  the boundary); `moved:` blocks reject collisions/chains at validate time
  (were silently overwriting lock state).
- **FAR archive + parser hardening** (#156): FAR decode bounds the
  manifest-length and chunk-count allocations from attacker-controlled
  headers (was unbounded → OOM/abort); the RFC-3339 and duration parsers
  validate ranges and use checked arithmetic (out-of-range month/day and
  multibyte/overflow inputs previously panicked).

## [1.5.0] - 2026-06-13

### Added

- **`forjar dist --verify` — Tier 1 static installer verification**
  (#146, FJ-3607/F-3609, Phase D of spec 25): generates artifacts to a
  temp dir and verifies `sh -n` parse, zero bashrs lint errors
  (in-process via the existing purifier wrapper), required
  checksum/arch-detection snippets, and download-URL structure.
  A deliberately broken asset template fails verification with a
  non-zero exit (F-3609). Tier 2 container execution is a follow-up.
- `forjar schema` now emits the `dist:` property block (mirrors
  DistConfig/targets/homebrew/nix, with a keep-in-sync pin test) —
  the spec claimed this existed; now it does (#146).
- `dist.source` validation: values other than `github_release`
  (local/url/s3) return a clear "not yet supported" error instead of
  generating artifacts with broken URLs (#146).

### Fixed

- **Coverage workflow de-flaked** (#147, closes #141): sandbox dirs in
  `run_convergence_test` were named from a wall-clock nanosecond read;
  concurrent threads on coarse-tick runners collided, and the first
  finisher's cleanup deleted the sibling's working dir mid-cycle
  (reproduced 32/240 trials). Names now include a process-wide atomic
  sequence — also hardens parallel `forjar check` in production.
- Release cargo cache keyed by runner image (#145): v1.4.4's
  x86_64-gnu leg failed linking 24.04-cached objects (glibc 2.38
  `__isoc23_*` symbols) on the new 22.04 baseline runner.

### Changed

- +69 lib tests closing the worst CLI coverage gaps (apply modes,
  canary/rolling fleet ops, infra dispatch, apply variants, check
  blocks, wave outcomes) — suite now 12,205 tests (#148).
- Spec 25 status reconciled from PROPOSED to per-phase reality
  (A-C implemented, D Tier 1 implemented, Tier 2 pending) (#146).

## [1.4.4] - 2026-06-12

### Fixed

- **`install.sh` works end-to-end** (#143) — verified live against the
  v1.4.3 release. The curl installer previously failed on every
  platform: wrong extraction path (archives contain a directory),
  unexpanded `~` fallback dir, hard dependency on a SHA256SUMS asset no
  release carried, `example.com` placeholder URLs, and a post-install
  version check that reported any pre-existing PATH binary. Generator
  now emits `$HOME`, directory-aware extraction, per-asset `.sha256`
  fallback, `"$DEST/$BINARY"`-anchored verification, `--prefix`
  traversal rejection, and lints clean (bashrs 16 errors → 0).
- **gnu binaries run on older distros again**: release builds moved to
  ubuntu-22.04 (glibc 2.35 baseline) — 24.04-built binaries demanded
  glibc ≥ 2.38.
- Regression tests pin the fixes for #85, #86, #88, #90 (#140); 15
  stale issues closed with evidence — open issues went from 18 to 1.
- Six F-grade files refactored to A-/B+ (#142, closes #116): zero
  F-grade files repo-wide; strict TDG pre-commit enforcement can be
  re-enabled via `pmat hooks refresh`.

### Added

- `binary-release.yml` uploads a combined **SHA256SUMS** asset per
  release (backfilled to v1.4.0/1/3); nightly **tag/release parity
  check**; all 8 historical tags now have GitHub releases with
  CHANGELOG-sourced notes, and v1.4.0/v1.4.1 received backfilled
  binaries.
- `tests/install_sh_parity.rs`: committed installer is byte-equal to
  generator output; workflow asset naming matches installer
  expectations.
- README documents the binary install path (installer one-liner,
  manual verify, cargo-binstall).
- `forjar dist` resolves **real checksums/versions** for Homebrew/Nix
  artifacts (#139, PMAT-080/F-3608/F-3610): `--version` + offline
  `--checksums-file`, hard errors instead of `PLACEHOLDER_CHECKSUM`.

## [1.4.3] - 2026-06-12

### Fixed

- **Three user-input-reachable panics** (#132, process aborts under
  `panic = "abort"`):
  - `unquote()` in when-expression evaluation sliced `&s[1..0]` for a
    lone quote character — `when: 'x == "'` panicked at plan time.
  - Secret-lint redaction byte-sliced `&matched[..12]` mid-UTF-8
    (e.g. `sshpass -p пароль`); now char-boundary-safe via the new
    shared `core::strutil` helpers, also adopted by `graph_svg`/`sbom`.
  - `forjar pin --check` sliced `&locked_hash[..16]` on hand-editable
    lockfile data; short/corrupt hashes no longer panic.
- **Auto-commit escaping its repository under git hooks** (#134, #137).
  forjar's git subprocesses inherited `GIT_DIR`/`GIT_WORK_TREE`/… which
  git exports to hook children, so `forjar apply` with `auto_commit`
  inside a git hook committed to the *hook's* repository. All git call
  sites now construct commands via the new `core::gitenv` module, which
  scrubs the nine repo-discovery variables.
- **Release pipeline repaired** (#131). The `Cargo.lock` version bump
  that every v1.4.x tag was missing is committed; a new CI
  `lockfile-preflight` job (`cargo package --locked --no-verify`) makes
  a stale lock fail PR CI instead of the tag; a committed dangling
  symlink (`.claude/worktrees/provable-contracts`) that broke
  `cargo package` on clean checkouts is removed; `.pmat-work/` and
  `.claude/` are excluded from the crate tarball.
- **Security Audit green again** (#131, closes #104, #98). Scoped
  cargo-deny `[[licenses.exceptions]]` for `libbz2-rs-sys` (SPDX
  `bzip2-1.0.6`); audit had been red on main since 2026-04-25.

### Added

- **Provable contracts for IaC convergence** (#136, closes #97):
  `idempotent-apply-v1` (converged lock ⇒ NoOp plan; f(f(x)) = f(x)),
  `plan-apply-equivalence-v1` (plan's predicted action set equals
  apply's executed set), `destroy-undo-roundtrip-v1` (undo restores the
  prior generation byte-for-byte) — 9 new `binding.yaml` entries, 22/22
  bound under `BindingPolicy::AllImplemented`, with `debug_assert!`
  call-sites and unit tests.
- **mdBook built in CI** (#135): new `docs.yml` builds the 102-chapter
  book with a SUMMARY.md resolution check; `tests/doc_cli_parity.rs`
  asserts all 154 CLI subcommands are documented; new CLI reference
  appendix documents 22 previously-undocumented commands; fixed
  `forjar completions` → `forjar completion` and stale MSRV/CLI counts.

### Changed

- **GitHub Releases are now created on hosted runners** (#131):
  `binary-release.yml` triggers on `v*` tag pushes, idempotently creates
  the release, and uploads the 4 Linux binaries — no self-hosted
  dependency in the critical path. Asset naming standardized to
  `forjar-<x.y.z>-<target>.tar.gz` (no `v` prefix) across both release
  workflows, matching homebrew patching and `install.sh`.
- Removed dead `provable-contracts` checkout/symlink/`pv codegen` steps
  from all 13 workflows (#131, #133) — `src/generated_contracts.rs` is
  git-tracked and `aprender-contracts` resolves from crates.io
  (closes #104's original cause; refs #112, #113).

## [1.4.2] - 2026-05-06

### Fixed

- `apply --force` now distinguishes "forced re-converge of an
  already-converged stack" from "legitimate re-converge after drift"
  in the apply summary (#129). Closes the gap that made claim **C3**
  (idempotency) unobservable through `--force`: a forced re-apply of
  a fully-converged stack used to print `N converged, 0 unchanged`,
  identical to a real drift-recovery run.

  When `--force` is used, the summary now adds:
  - **Text mode:** a yellow `note: --force re-ran N resource(s) the
    lock reported as unchanged (M actual change(s), N forced no-op(s))`
    line. `M = 0` is the C3-PASS demonstration.
  - **JSON mode:** new `forced_noop_count` and `actual_changes` fields
    in `summary{}`.
  - **Runtime contract:** `debug_assert!(forced_noop <= converged)` —
    aborts a debug build on planner / executor disagreement.

  Provable contract: `contracts/apply-summary-distinguishability-v1.yaml`.
  Integration test: `tests/test_fj129_force_distinguishability.rs`
  exercises all four step shapes from the contract's proof obligations.

  Surfaced downstream in [paiml/iac-from-zero](https://github.com/paiml/iac-from-zero)
  — the Coursera companion course had to drop `--force` from its
  C3 idempotency CI assertion to make the claim observable. With this
  fix, the demonstration works through `--force` too.

## [1.4.1] - 2026-05-02

### Fixed
- `apply_apt_latest` (`state: latest`) now tolerates `apt-get update` partial failures via `|| true` (Refs PMAT-161). Real hosts routinely have one or two unreachable third-party PPAs, masked PackageKit units, or stale arm64 entries on x86_64 boxes that make `apt-get update` exit non-zero even when the repos we actually care about refreshed cleanly. The subsequent `apt-get install` still fails loud if the requested package can't be resolved, so the `dpkg -l '^ii '` postcondition is preserved. Matches canonical Dockerfile / Ansible practice. Discovered when applying `state: latest` for `docker-ce` on lambda-labs (broken `mozillateam`, `obsproject`, etc. PPAs).

## [1.4.0] - 2026-05-02

### Added
- `state: latest` for the apt package provider (#125, Refs PMAT-161). Validation already accepted `latest` for Package types but `apply_script()` fell through to the unsupported arm, making it a no-op. The new `apply_apt_latest()` runs `apt-get update -qq` then `apt-get install -y -qq <pkgs>`, which installs missing packages or upgrades to the newest available version (no-op if already current). Closes the gap that made `apply_apt_present` (presence-only `dpkg -l` guard) unable to express "always latest" — adding `version:` to YAML was a no-op once a package was already installed at any older version. Postcondition verified via `dpkg -l "$pkg" | grep -q '^ii '` for each requested package.

## [1.3.0] - 2026-04-25

### Added
- `forjar reseal` recovery subcommand for re-creating sidecar BLAKE3 integrity files (#118, #119)
- Contract trait enforcement expanded from 7/13 to 13/13 implementations
- `pv codegen` contract macros for build-time generation (Refs PMAT-120)
- Contract call-site instrumentation for `hash_data` + `execute_isolated` (Refs PMAT-122)
- `vendored-openssl` feature flag for cross-compilation reliability (Refs PMAT-067)
- AllImplemented enforcement policy — build now fails on contract gaps
- Sovereign-CI self-hosted runner adoption with PR authorization gate
- Nightly Criterion benchmarks via reusable workflow

### Changed
- Migrated from archived `provable-contracts` → `aprender-contracts` (#117, Refs PMAT-163)
- Bumped major dependency versions: `aprender-contracts(-macros) 0.30 → 0.31`, `bzip2 0.5 → 0.6`, `toml 0.8 → 1.1`, `criterion 0.5 → 0.8` (dev-dep)
- `cargo update`: tokio 1.50 → 1.52, indexmap 2.7 → 2.14, regex 1.x → 1.12, openssl 0.10.x → 0.10.78, async-trait 0.1.x → 0.1.89, plus dozens of compatible bumps across the tree
- Replaced deprecated `criterion::black_box` with `std::hint::black_box` in benches
- `pmat repo-score` compliance improved from 79.0 → 91.5
- README: added Features section, docs.rs badge, cookbook link, CI/crates.io badges, MSRV badge corrected to 1.89.0
- `.gitignore`: added `.claude/` (Claude Code session state) so it doesn't block clean publishes

### Fixed
- Sidecar BLAKE3 errors now propagate instead of being silently swallowed (#118, #119)
- Removed hardcoded `/mnt/nvme-raid0` path from `.cargo/config.toml` (#109, #110)
- Doctor SSH test handles missing `ssh` binary in CI containers
- `generate_installer` complexity reduced for CB-200 compliance (Refs PMAT-131)
- `generated_contracts.rs` is now a build artifact (gitignored, replaced 5858-line stale stub)
- 11 silently-ignored CLI flags now emit warnings
- `ingest_state_dir` errors are logged instead of silently discarded
- Security advisories: `tar 0.4.45` (RUSTSEC-2026-0067/0068), `rustls-webpki 0.103.10` (RUSTSEC-2026-0049)
- Contract-trait enforcement test added (provable-contracts §23)
- Parser whitelist now recognizes top-level `dist:` field — previously `forjar fmt → forjar validate --strict` failed and many commands logged spurious "unknown field 'dist'" warnings against dist-aware configs
- Race condition in `cli::colors` tests serialized via per-module `Mutex` — global `NO_COLOR` atomic could be flipped by a parallel test mid-assertion, causing intermittent CI failures

### Security
- Updated multiple deps for RUSTSEC-2026-{0007,0009,0041,0044-0049,0067,0068}
- `deny.toml`: explicit advisory ignores documented with reason + review date
- `RUSTSEC-2026-0104` (rustls-webpki CRL panic) acknowledged with mitigation note

## [1.2.1] - 2026-03-13

### Added
- `forjar dist` command and full release pipeline (FJ-3600)
- `github_release` resource type for nightly binary installation (FJ-034)
- WASM plugin runtime via `wasmi` (FJ-3404, #80)
- Watch daemon for filesystem-driven re-apply (FJ-3102)
- `aarch64-linux-gnu` to nightly build matrix
- All competitive feature gaps implemented (#77-#84)
- Refactored 26 oversized files under 500-line limit (Refs PMAT-029, PMAT-056)

### Fixed
- Vendor OpenSSL handling for cross-compile (later made opt-in via feature flag in 1.3.0)
- Template parameter resolution in `github_release` resource fields (Refs FJ-034)
- Asset filename preservation in `github_release` download
- Pre-apply drift check now passes machine context for container transports (Refs PMAT-058)

## [1.2.0] - 2026-03-10

### Added
- `forjar dist` command family (FJ-3600): generate distribution artifacts from YAML config
- 7 artifact generators: shell installer, Homebrew formula, cargo-binstall, Nix flake, GitHub Action, deb, rpm
- DistConfig type system with per-target libc variant support
- macOS targets (x86_64-apple-darwin, aarch64-apple-darwin) in release pipeline
- `cargo binstall forjar` support via `[package.metadata.binstall]`
- `.github/actions/setup-forjar` composite action for CI consumers
- `install.sh` at repo root for `curl | sh` installs
- `flake.nix` at repo root for `nix run github:paiml/forjar`
- Automated Homebrew tap publishing with real SHA256 checksums on release
- 56 Popperian falsification tests for dist generators
- `dist-forjar.yaml` — dogfood config for forjar's own distribution

## [1.1.1] - 2026-03-04

### Fixed
- Refactored 3 dispatch functions below TDG Grade A complexity threshold (CB-200)
- Removed 23 unwrap() calls from production code (kaizen RUST-UNWRAP-001)
- Fixed clippy warnings when building without encryption feature
- Pinned GitHub Actions reusable workflow to SHA (CB-953)

### Changed
- Made `age` encryption crate optional via `encryption` feature flag
- Reduced prod transitive dependencies from 305 to 253 (CB-081)
- Added `.pmat.yaml` with CB-954 suppression for `secrets: inherit`

## [1.1.0] - 2026-03-03

### Added
- Features #164-#166: Complexity analysis, impact analysis, drift prediction CLI commands
- Chapter 17: Operational Intelligence in the forjar book
- Chapter 18: Supply Chain Security & Resilience in the forjar book
- 7 new cookbook recipes (79-85) with A-grade quality scores
- 500+ new tests across 21 test files for 95%+ line coverage
- 3 cargo run examples: complexity, impact, drift prediction

### Fixed
- 5 stale entries in v2 spec falsification audit log (PMAT-042 through PMAT-046)

## [1.0.0] - 2026-03-01

### Added
- 163/163 v2 spec features complete
- Reproducible binary builds (FJ-095)
- Formal verification proofs: Kani + Verus
- State safety with BLAKE3 integrity chains
- MLOps/DataOps resource types (model, dataset, pipeline)
- Agent infrastructure (pull-based, registry, SBOM)
- Post-quantum signing (ML-DSA-65)
- GPU container support (NVIDIA + ROCm)
- Store system: content-addressable with GC
- Recipe system with expansion and validation
- 8000+ unit tests, 95%+ line coverage

## [0.2.0] - 2026-02-24

### Added
- SSH transport with batch mode and connection pooling (FJ-040)
- Container transport with Docker/Podman support (FJ-050)
- Rolling deployment with wave-based execution (FJ-060)
- Drift detection with anomaly scoring (FJ-070)
- Fleet status reporting with 200+ analytics flags (FJ-080)
- Lock file security auditing (FJ-090)
- Configuration validation with 30+ checks (FJ-100)
- Graph analysis: dependency visualization, impact, topology (FJ-110)

### Changed
- Upgraded to BLAKE3 1.8 for 15% faster hashing
- Switched to serde_yaml_ng for improved YAML parsing

## [0.1.0] - 2026-02-16

### Added
- YAML configuration parser with validation (FJ-001)
- Dependency resolver with Kahn's topological sort (FJ-003)
- Execution planner with BLAKE3 desired-state hashing (FJ-004)
- Script codegen for package, file, service, mount resources (FJ-005)
- File resource: create, directory, symlink, absent states (FJ-007)
- Package resource: apt, cargo, uv providers (FJ-006)
- Service resource: systemd start/stop/enable (FJ-009)
- Mount resource: NFS/bind mount/unmount (FJ-006)
- Local transport executor (FJ-010)
- SSH transport executor (FJ-011)
- Full apply orchestration with Jidoka failure policy (FJ-012)
- Atomic lock file persistence (FJ-013)
- BLAKE3 hashing for files, directories, strings (FJ-014)
- Append-only JSONL event log (FJ-015)
- Drift detection via hash comparison (FJ-016)
- CLI subcommands: init, validate, plan, apply, drift, status (FJ-017)
- Recipe system with typed inputs, validation, namespaced expansion (FJ-019)
- Provable contracts integration with 15 falsification tests (FJ-020)
- 254 unit tests across all modules
- Criterion benchmarks with 95% confidence intervals
