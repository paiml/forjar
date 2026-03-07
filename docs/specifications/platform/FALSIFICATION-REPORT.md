# Falsification Report: forjar-platform-spec.md

> Systematic verification of every falsifiable claim against the actual codebase.
> Generated: 2026-03-06 | Method: Code audit with 4 parallel agents
> Updated: 2026-03-07 | 29/29 resolved (U3 deferred — needs root)
> Deep falsification: 42/42 phases IMPLEMENTED. P0 safety fix (F12), real sandbox I/O (F10-F11), error handling (F13-F14), behavior spec execution (F15).

---

## Severity Key

- **F** = Falsified (spec says X, code does Y)
- **E** = Exaggerated (partially true, overstated)
- **S** = Stale (was true, code has since changed)
- **U** = Unverified (no test/benchmark validates the claim)
- **C** = Confirmed

---

## Critical Falsifications

### ~~F1: `forjar diff --generation 3 7` does not exist~~ FIXED

**Resolved**: `GenerationCmd::Diff { from, to, state_dir, json }` added to `subcmd_args.rs`. `cmd_generation_diff()` in `generation.rs` loads lock files from both generation directories, computes per-machine `diff_resource_sets()`, and outputs text or JSON. 7 tests added in `tests_cov_gen_diff.rs`. Commit `dce1768`.

---

### ~~F2: `forjar undo-destroy` replay is not implemented~~ FIXED

**Resolved**: Replay loop implemented in `undo.rs`. For each entry with `config_fragment`, deserializes the `Resource`, generates a convergence script via `codegen::apply_script()`, and executes via `transport::exec_script()`. Entries without `config_fragment` are skipped with error. Returns `Err` if any entries fail. Commit `dce1768`.

---

### ~~F3: Incremental ingest with cursor not implemented~~ PARTIALLY FIXED

**Resolved**: `ingest_cursor` table now exists in the SQLite schema (`db.rs`). The `IngestCursor` type in `sqlite_schema_types.rs` provides `is_ingested()`/`mark_ingested()` methods. Wiring into `ingest_state_dir()` for actual mtime/hash skip is deferred to Phase 6 (optimization).

---

### ~~F4: Content policy does not exist~~ FIXED (spec corrected)

**Resolved**: Spec text changed from "content policy" to "path restrictions" — `deny_paths` IS the content restriction mechanism. Section 10 subtitle updated. Main spec table updated.

---

### ~~F5: Dual-digest "single pass" is false~~ FIXED (spec corrected)

**Resolved**: Spec text changed from "Dual-digest computed in a single pass" to "Both digests computed per artifact (BLAKE3 for store addressing, SHA-256 for OCI manifests)". Commit `dce1768`.

---

## Exaggerations

### ~~E1: "Second apply is always a no-op (<1ms, zero I/O)"~~ FIXED (spec corrected)

**Resolved**: Spec text changed to "zero remote I/O, zero mutations (state files are read and hashes recomputed, but no convergence actions execute)". Commit `dce1768`.

---

### ~~E2: "flock advisory locking" is actually PID-file locking~~ FIXED (spec corrected)

**Resolved**: Main spec and L13 in known-limitations updated to say "PID-file with liveness check" instead of "flock". TOCTOU race documented. Commit `dce1768`.

---

### ~~E3: Secret providers are type definitions only~~ FIXED (spec corrected)

**Resolved**: Spec updated to show implementation status per provider. Age encryption marked as implemented, others as planned. Main spec table updated to "Age encryption (env/file/SOPS planned)". Status note added to 10-security-model.md.

---

### ~~E4: pepita overlayfs not implemented~~ FIXED (spec corrected)

**Resolved**: Spec updated in 04-multi-machine-ops.md. Mount namespace entry now says "mount namespace isolation" instead of "overlayfs CoW". Overlayfs entry clarifies "(store sandbox only; pepita transport uses mount namespace without overlayfs)". Comparison table updated to "mount namespace (overlayfs planned)".

---

### E5: Kani harnesses are bounded toy models (mostly) — REMEDIATED

**Spec claim** (09-provable-design-by-contract.md):
> Kani real-code harnesses

**Reality**: 17 `#[kani::proof]` harnesses in `kani_proofs.rs`. Deprecation notice (lines 11-18) calls them "abstract-model harnesses operating on simplified state." Real-code harnesses exist (4 of 17) but use tiny bounds (4 packages, 8-char strings). Useful but overstated.

**Five-Whys Root Cause**: Fundamental mismatch between Kani's exhaustive bounded verification model and Forjar's complex `Resource` type (30+ fields, nested Options). Kani state space explodes exponentially with `Option<String>` fields. See full analysis in 09-provable-design-by-contract.md § Five-Whys.

**Remediation**:
1. ~~Rename harnesses from `proof_*_real` to `proof_*_bounded` for honesty~~ DONE (2026-03-06)
2. Accept that proptest provides empirical verification where Kani cannot go
3. ~~Add `debug_assert!` in production functions as Tier 1 safety net~~ ALREADY EXISTS (`planner/mod.rs:225-230`, `planner/mod.rs:306-310`)
4. Spec 09 Phase 14 checkboxes corrected to reflect actual status

---

### E6: Runtime contracts are on spec wrappers, not production code — REMEDIATED

**Spec claim** (09-provable-design-by-contract.md):
> All critical-path functions have `#[ensures]` contracts

**Reality**: `#[requires]`/`#[ensures]` attributes exist only on `spec_*()` wrapper functions inside `verus_spec.rs` (8 functions). Actual production functions like `hash_desired_state()`, `determine_present_action()` have NO contracts. Custom `#[contract]` macros exist on a few `tripwire/hasher.rs` functions but these are metadata tags, not runtime checks.

**Five-Whys Root Cause**: The `contracts` crate's `#[ensures]` macro cannot express postconditions over complex types like `HashMap<String, StateLock>`. Contracts were added to Verus spec wrappers (which use simple model types) but never ported to production functions. See full analysis in 09-provable-design-by-contract.md § Five-Whys.

**Remediation**:
1. ~~Use `debug_assert!` directly inside production functions~~ DONE for all 4 critical-path functions:
   - `determine_present_action` (`planner/mod.rs:225-230`)
   - `hash_desired_state` (`planner/mod.rs:306-310`)
   - `save_lock` (`core/state/mod.rs:56-57`)
   - `build_execution_order` (`core/resolver/dag.rs:31-40`)
2. Spec 09 Phase 13 updated (2026-03-06) to IMPLEMENTED status
3. OCI functions also contracted: `build_layer` (determinism), `assemble_image` (OCI layout validity), `compute_dual_digest` (consistency), `write_oci_layout` (integrity)
4. G4 fully remediated: store idempotency contract + OCI manifest media type assertions added (2026-03-07)

---

### E7: Spec 09 gaps section contradicts its own implementation section — FIXED

**Finding** (2026-03-06): 09-provable-design-by-contract.md § "Five Gaps" describes G1 as "The Critical Path Is Uncontracted" with `determine_present_action` having "NO precondition, NO postcondition." But the Implementation section (Phase 13) marked `[x]` for "`#[debug_ensures]` on `determine_present_action`." These contradict within the same document.

**Root Cause**: Checkboxes were marked when spec wrapper functions were annotated, without verifying that production functions received the same treatment. The gap section was written from code audit; the implementation section was written from intent.

**Resolution**: Phase 13 checkboxes corrected (2026-03-06) to show `[ ]` for items that exist only as spec wrappers. Gap section remains accurate.

---

### E8: PARTIAL phases with all-`[x]` items

**Finding** (2026-03-06): Multiple spec files label phases as "PARTIAL" but every item within is `[x]`. This is misleading — readers assume `[x]` means "done" and "PARTIAL" means "some items unchecked." The actual meaning is "types/stubs exist (`[x]`) but end-to-end runtime behavior is unverified (PARTIAL)."

**Affected files**: 05-container-builds.md (Phases 8-10), 14-testing-strategy.md (Phases 28, 30-32, 34), 15-task-framework.md (Phases 36, 40).

**Root Cause**: The spec's `[x]` convention tracks type/struct existence, not runtime functionality. This distinction is not documented anywhere.

**Resolution**: Added clarifying notes to affected PARTIAL phases in 05-container-builds.md and 06-distribution.md (2026-03-06). Convention documented: `[x]` = "type or CLI wiring exists in code", PARTIAL = "end-to-end runtime flow not yet tested/integrated."

---

### F8: `cmd_logs` was a stub — types existed, runtime did not — REMEDIATED

**Spec claim** (11-observability.md, Phase 18):
> `forjar logs` CLI command reads structured logs with filtering, JSON output, and garbage collection.

**Original finding** (2026-03-07): `cmd_logs()` was a stub returning placeholder data with no file I/O.

**Remediation** (2026-03-07): `cli/logs.rs` created with real runtime:
1. `discover_runs()` scans `state/<machine>/runs/<run_id>/meta.yaml` — reads, deserializes, filters
2. `read_log_file()` / `read_script_file()` read `.log` and `.script` files
3. `cmd_logs_gc()` applies `LogRetention` policy with `--dry-run` and `--keep-failed`
4. `cmd_logs_follow()` finds most recent run directory
5. All CLI flags added: `--resource`, `--script`, `--all-machines`, `--gc --dry-run`, `--gc --keep-failed`
6. 27 integration tests with real tempdir run directories

**Fully remediated** (2026-03-07): `run_capture.rs` wired into `execute_resource()` → `handle_resource_output()`. Every `forjar apply` now generates `state/<machine>/runs/<run_id>/` with `meta.yaml`, `.log`, and `.script` files. `cmd_logs` reads them back with filtering.

---

### F9: `SandboxBackend` declared but never dispatched — REMEDIATED

**Spec claim** (14-testing-strategy.md, Phase 31):
> Convergence and mutation runners execute in isolated sandboxes (pepita, container, or chroot).

**Five-whys root cause**:
1. Why dead code? Runners use `simulate_apply()` stubs, never real sandbox
2. Why stubs only? No bridge from `SandboxBackend` → `sandbox_run::execute_sandbox_plan()`
3. Why no bridge? Two independent `SandboxConfig` types diverged during parallel development
4. Why two types? `store/sandbox.rs` (build-time, FJ-1315) and `test_runner_types.rs` (test-time, FJ-2603) created independently
5. Why not unified? No cross-cutting review between store and test-runner subsystems

**Remediation** (2026-03-07):
1. `convergence_runner.rs`: Added `SandboxBackend` field to `ConvergenceTestConfig`, `RunnerMode` enum, `backend_available()` + `resolve_mode()` dispatch
2. `convergence_container.rs`: Real container-based convergence testing — creates ephemeral Docker/Podman containers, runs apply/query scripts, compares state hashes
3. `mutation_container.rs`: Real container-based mutation testing — baseline, mutate, detect drift, re-converge inside ephemeral containers
4. `mutation_runner.rs`: Added `SandboxBackend` field to `MutationRunConfig`, `run_mutation_test_dispatch()` for mode-aware routing
5. `check_test.rs`: Both `cmd_test_convergence` and `cmd_test_mutation` now call `resolve_mode()` to print actual mode (simulated vs sandbox)
6. 9 new tests covering backend detection, mode resolution, dispatch routing, and config defaults
7. Graceful degradation: when backend is unavailable, falls back to simulated mode with clear messaging

**Status**: Fully remediated. Container backend (Docker/Podman) executes real convergence and mutation tests in ephemeral containers. Pepita backend falls back to simulated (requires pepita binary). Chroot falls back to simulated (requires root).

---

### F10: Mutation drift detection was hardcoded to true (fake 100% score) — FIXED

**Spec claim** (14-testing-strategy.md):
> Mutation testing detects drift by comparing pre/post-mutation state hashes.

**Reality** (before fix): `run_mutation_test()` used `simulate_apply()` stubs that returned `detected: true` for all mutations. The "100% mutation score" was fabricated — no real file I/O, no actual drift detection.

**Five-whys root cause**: When `SandboxBackend` dispatch was added (F9), the simulated mode kept using hash-based stubs. Nobody noticed because the mutation score always looked "perfect."

**Fix** (2026-03-07): `mutation_runner.rs` now creates real tempdir sandboxes, runs apply/mutation/drift scripts via `bash -euo pipefail`, and detects drift by comparing BLAKE3 hashes of script output before vs after mutation. Real scores: file mutations detected 100% (8/8), system mutations properly errored (2/2).

---

### F11: Convergence testing was simulated hashing (no real I/O) — FIXED

**Spec claim** (14-testing-strategy.md):
> Convergence tests verify apply-verify-reapply-verify cycle.

**Reality** (before fix): `run_convergence_test()` hashed the script text itself and compared against `expected_hash`. No scripts were ever executed. "Convergence" was string comparison, not behavior verification.

**Fix** (2026-03-07): `convergence_runner.rs` now creates real tempdir sandboxes with `$FORJAR_SANDBOX` env, executes scripts via bash, and verifies convergence (actual state matches expected), idempotency (second apply produces same state), and preservation (state persists without re-apply).

---

### F12: P0 SAFETY — Host system commands executed without sandboxing — FIXED

**Spec claim** (14-testing-strategy.md, 10-security-model.md):
> All test execution is sandboxed. System operators require container backend.

**Reality** (before fix): When local execution was added (F10/F11), scripts like `systemctl stop nginx` and `apt-get remove -y curl` were executed directly on the host system via `bash -euo pipefail`. Users were prompted for sudo access to restart nginx.

**Five-whys root cause**: The safety model assumed simulated mode (no real execution). When real execution was added, no safety gates were implemented — the code went from "never executes" to "executes everything" with no intermediate check.

**Fix** (2026-03-07): Defense-in-depth with two layers:
1. `mutation_runner.rs`: `is_safe_for_local()` blocks system operators (StopService, RemovePackage, KillProcess, UnmountFilesystem) with clear error message requiring container backend.
2. `convergence_runner.rs`: `UNSAFE_PATTERNS` blocklist rejects scripts containing `systemctl`, `apt-get`, `pkill`, `mount`, `umount`, `kill`, etc. before bash execution.

---

### F13: Webhook errors silently swallowed — FIXED

**Spec claim** (11-observability.md):
> Structured error reporting for all operations.

**Reality** (before fix): `send_webhook()` in `dispatch_notify.rs` used `let _ = Command::new("curl")...` — discarding both execution errors and non-zero exit codes silently.

**Fix** (2026-03-07): Changed to `match` with `eprintln!` warnings for failed webhooks (non-zero exit) and execution errors. Curl flag changed from `-s` to `-sf` (fail on HTTP errors).

---

### F15: Behavior specs were structural-only (never executed verify commands) — FIXED

**Spec claim** (14-testing-strategy.md, Phase 30):
> `forjar test behavior` executes YAML behavior specs

**Reality** (before fix): `cmd_test_behavior()` checked whether `assert_state`, `has_verify()`, or `is_convergence()` fields existed on each behavior entry — purely structural validation. Verify commands were never executed via bash. Every spec with a `verify:` field passed automatically.

**Five-whys root cause**: The behavior runner was implemented as a spec-loading demo during the type system phase. The `VerifyCommand` type was added with a `command` field but the execution loop never called `bash`. Nobody tested with specs that should fail.

**Fix** (2026-03-07): Added `execute_behavior()` function that runs `verify.command` via `bash -euo pipefail`, compares exit code and stdout against expected values. 4 new tests cover pass, fail, stdout mismatch, and no-assertion cases.

---

### F14: Thread panics silently dropped in wave execution — FIXED

**Spec claim** (04-multi-machine-ops.md):
> Wave execution handles errors per-resource with structured reporting.

**Reality** (before fix): `execute_wave_io()` in `machine_wave.rs` used `.filter_map(|h| h.join().ok())` — silently dropping any thread that panicked. A panic in one resource's apply would cause that resource to vanish from results with no error reported.

**Fix** (2026-03-07): Replaced with explicit panic handling via `extract_panic_message()`. Panicked threads return `Err("thread panic: <message>")` results and log to stderr. No results are silently dropped.

---

## Stale Claims (Code Has Changed)

### ~~S1: L4 "Destroy State Cleanup Bug"~~ FIXED (spec updated)

**Resolved**: L4 in known-limitations marked as RESOLVED. Commit `dce1768`.

---

### ~~S2: L16 "No Selective Apply"~~ FIXED (spec updated)

**Resolved**: L16 in known-limitations marked as RESOLVED, referencing `resource_filter` on ApplyArgs. Commit `dce1768`.

---

## Unverified Claims (No Evidence)

### ~~U1: Performance target <50ms for `forjar query`~~ VERIFIED

**Resolved**: Benchmark test `query_latency_under_50ms` in `tests_db_bench.rs` validates FTS5 query completes in <50ms for a 3-machine/40-resource dataset.

### ~~U2: state.db < 1MB for 3 machines~~ VERIFIED

**Resolved**: Benchmark test `state_db_size_under_1mb` in `tests_db_bench.rs` validates state.db stays under 1MB for 3 machines × 20 resources × 100 events each.

### U3: pepita namespace creation in 10-50ms

No benchmark measures pepita startup latency. Requires root/CAP_SYS_ADMIN — cannot be validated in unit tests.

---

## SQLite Schema Gaps

### ~~F6: FTS5 schema doesn't match spec~~ FIXED

**Resolved**: `resources_fts` now uses spec-compliant columns (`resource_id, resource_type, path, packages, content_preview`) with porter tokenizer. Removed `status` and `details_json` from FTS5 index (no longer indexes raw JSON). `fts5_search()` uses JOIN with `resources` table to retrieve `status`. `resources` table has `packages` and `content_preview` columns. Ingest pipeline extracts `packages` from package-type resources.

### ~~F7: Multiple spec-defined tables don't exist~~ FIXED

**Resolved**: All 5 missing schema elements now exist in `db.rs`:

| Table/Index | Status |
|-------------|--------|
| `destroy_log` | Added — ingested from `destroy-log.jsonl` |
| `drift_findings` | Added — populated by drift detection |
| `events_fts` | Added — FTS5 with porter tokenizer |
| `idx_resources_status` | Added |
| `ingest_cursor` | Added |

---

## Confirmed Claims (Verified Against Code)

| Claim | Location |
|-------|----------|
| Nix-style numbered generations with atomic symlink | `cli/generation.rs` |
| BLAKE3 for internal store and drift | `tripwire/hasher.rs` |
| SSH ControlMaster persistent connections | `transport/ssh.rs:65-69` |
| Content-addressed store | `core/store/` |
| FAR archives (zstd + BLAKE3 Merkle) | `core/store/far.rs` |
| bashrs purification (I8 invariant) | `transport/mod.rs:51-55` |
| Wave parallelism | `executor/machine_wave.rs` |
| WAL mode on state.db | `core/store/db.rs:13` |
| PRAGMA user_version schema versioning | `core/store/db.rs:113-121` |
| OCI Image Spec v1.1 compliance | `core/types/oci_types.rs` |
| Registry push (OCI Distribution v1.1) | `core/store/registry_push.rs` |
| `type: image` resource type | `core/types/resource.rs:425` |
| Three OCI build paths | `LayerBuildPath` enum |
| allowed_operators enforcement | `core/types/config.rs:464-466` |
| deny_paths enforcement (parse-time) | `core/parser/format_validation.rs:200-226` |
| Transport-agnostic runtime dispatch | `transport/mod.rs:86-107` |
| CQRS: flat files = source of truth | `core/store/ingest.rs` |
| Destroy-log JSONL writing | `cli/destroy.rs:92-126` |
| Active undo (observe-diff-act via cmd_apply) | `cli/undo.rs:131-215` |
| Daemonless container builds | No docker daemon dependency |
| `forjar build` CLI command | `commands/platform_args.rs:281` |
| Verus proofs exist (compile-gated) | `core/verus_spec.rs` |
| Four-tier verification structure | All four tiers present |

---

## Action Items

| Priority | Item | Severity | Status |
|----------|------|----------|--------|
| ~~1~~ | ~~Wire `forjar generation diff` CLI command~~ | ~~F1~~ | DONE |
| ~~2~~ | ~~Implement undo-destroy replay loop~~ | ~~F2~~ | DONE |
| ~~3~~ | ~~Fix "flock" language → "PID-file locking"~~ | ~~E2~~ | DONE |
| ~~4~~ | ~~Fix "zero I/O" → "zero remote I/O"~~ | ~~E1~~ | DONE |
| ~~5~~ | ~~Create missing SQLite tables (destroy_log, drift_findings, events_fts)~~ | ~~F7~~ | DONE |
| ~~6~~ | ~~Add ingest_cursor table to schema~~ | ~~F3~~ | DONE (table exists; wiring deferred) |
| ~~7~~ | ~~Implement FTS5 field extraction per spec~~ | ~~F6~~ | DONE |
| ~~8~~ | ~~Mark L4 as resolved in known-limitations~~ | ~~S1~~ | DONE |
| ~~9~~ | ~~Mark L16 as resolved in known-limitations~~ | ~~S2~~ | DONE |
| ~~10~~ | ~~Fix pepita overlayfs spec language~~ | ~~E4~~ | DONE |
| ~~11~~ | ~~Add performance benchmarks for query/state.db targets~~ | ~~U1-U2~~ | DONE (U3 needs root) |
| ~~12~~ | ~~Fix secret provider spec language~~ | ~~E3~~ | DONE |
| ~~13~~ | ~~Fix dual-digest "single pass" claim~~ | ~~F5~~ | DONE |
| ~~14~~ | ~~Fix content policy spec language~~ | ~~F4~~ | DONE |
| ~~15~~ | ~~Add `debug_assert!` to `determine_present_action` (production code)~~ | ~~E6~~ | DONE (already exists: `planner/mod.rs:225-230`) |
| ~~16~~ | ~~Rename Kani harnesses `proof_*_real` → `proof_*_bounded`~~ | ~~E5~~ | DONE |
| ~~17~~ | ~~Add `debug_assert!` to `hash_desired_state` (production code)~~ | ~~E6~~ | DONE (already exists: `planner/mod.rs:306-310`) |
| ~~18~~ | ~~Correct spec 09 Phase 13-15 checkboxes~~ | E7 | DONE |
| ~~19~~ | ~~Add PARTIAL convention note to spec 05, 06~~ | E8 | DONE |
| ~~20~~ | ~~Implement `cmd_logs` runtime (read log files, real JSON data)~~ | F8 | DONE |
| ~~21~~ | ~~Add missing LogsArgs CLI flags (`--resource`, `--script`, `--all-machines`, `--gc --dry-run`, `--gc --keep-failed`)~~ | F8 | DONE |
| ~~22~~ | ~~Wire `execute_and_capture()` into apply pipeline to generate run logs~~ | F8 | DONE |
| ~~23~~ | ~~Wire `SandboxBackend` dispatch into convergence/mutation runners~~ | F9 | DONE |
| ~~24~~ | ~~Replace fake mutation drift detection with real sandbox I/O~~ | F10 | DONE |
| ~~25~~ | ~~Replace simulated convergence hashing with real script execution~~ | F11 | DONE |
| ~~26~~ | ~~Add host-protection safety guards (is_safe_for_local, UNSAFE_PATTERNS)~~ | F12 | DONE |
| ~~27~~ | ~~Log webhook errors instead of silently discarding~~ | F13 | DONE |
| ~~28~~ | ~~Handle thread panics in wave execution instead of dropping~~ | F14 | DONE |
| ~~29~~ | ~~Execute behavior spec verify commands via bash instead of structural check~~ | F15 | DONE |
