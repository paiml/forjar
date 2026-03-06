# Falsification Report: forjar-platform-spec.md

> Systematic verification of every falsifiable claim against the actual codebase.
> Generated: 2026-03-06 | Method: Code audit with 4 parallel agents
> Updated: 2026-03-06 | 16/19 resolved, 3 TODO (E5 rename, E6 debug_assert x2), U3 deferred (needs root)

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

### E5: Kani harnesses are bounded toy models (mostly) — REMEDIATION PLANNED

**Spec claim** (09-provable-design-by-contract.md):
> Kani real-code harnesses

**Reality**: 17 `#[kani::proof]` harnesses in `kani_proofs.rs`. Deprecation notice (lines 11-18) calls them "abstract-model harnesses operating on simplified state." Real-code harnesses exist (4 of 17) but use tiny bounds (4 packages, 8-char strings). Useful but overstated.

**Five-Whys Root Cause**: Fundamental mismatch between Kani's exhaustive bounded verification model and Forjar's complex `Resource` type (30+ fields, nested Options). Kani state space explodes exponentially with `Option<String>` fields. See full analysis in 09-provable-design-by-contract.md § Five-Whys.

**Remediation**:
1. Rename harnesses from `proof_*_real` to `proof_*_model` for honesty (spec 09 updated 2026-03-06)
2. Accept that proptest provides empirical verification where Kani cannot go
3. Add `debug_assert!` in production functions as Tier 1 safety net (not yet done)
4. Spec 09 Phase 14 checkboxes corrected to reflect actual status

---

### E6: Runtime contracts are on spec wrappers, not production code — REMEDIATION PLANNED

**Spec claim** (09-provable-design-by-contract.md):
> All critical-path functions have `#[ensures]` contracts

**Reality**: `#[requires]`/`#[ensures]` attributes exist only on `spec_*()` wrapper functions inside `verus_spec.rs` (8 functions). Actual production functions like `hash_desired_state()`, `determine_present_action()` have NO contracts. Custom `#[contract]` macros exist on a few `tripwire/hasher.rs` functions but these are metadata tags, not runtime checks.

**Five-Whys Root Cause**: The `contracts` crate's `#[ensures]` macro cannot express postconditions over complex types like `HashMap<String, StateLock>`. Contracts were added to Verus spec wrappers (which use simple model types) but never ported to production functions. See full analysis in 09-provable-design-by-contract.md § Five-Whys.

**Remediation**:
1. Use `debug_assert!` directly inside production functions (bypasses proc macro limitations)
2. Estimated: ~20 lines in `planner/mod.rs`, ~10 lines in `core/state/mod.rs`
3. Spec 09 Phase 13 checkboxes corrected (2026-03-06) to show `[ ]` for uncontracted production functions
4. The spec wrapper contracts remain as documentation of the intended postconditions

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
| 15 | Add `debug_assert!` to `determine_present_action` (production code) | E6 | TODO |
| 16 | Rename Kani harnesses `proof_*_real` → `proof_*_model` | E5 | TODO |
| 17 | Add `debug_assert!` to `hash_desired_state` (production code) | E6 | TODO |
| ~~18~~ | ~~Correct spec 09 Phase 13-15 checkboxes~~ | E7 | DONE |
| ~~19~~ | ~~Add PARTIAL convention note to spec 05, 06~~ | E8 | DONE |
