# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
