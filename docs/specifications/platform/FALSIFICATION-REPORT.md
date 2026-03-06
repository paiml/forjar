# Falsification Report: forjar-platform-spec.md

> Systematic verification of every falsifiable claim against the actual codebase.
> Generated: 2026-03-06 | Method: Code audit with 4 parallel agents

---

## Severity Key

- **F** = Falsified (spec says X, code does Y)
- **E** = Exaggerated (partially true, overstated)
- **S** = Stale (was true, code has since changed)
- **U** = Unverified (no test/benchmark validates the claim)
- **C** = Confirmed

---

## Critical Falsifications

### F1: `forjar diff --generation 3 7` does not exist

**Spec claim** (main spec line 184, 02-generation-undo.md):
> `forjar diff --generation 3 7`

**Reality**: Types exist (`GenerationDiff`, `diff_resource_sets()` in `generation_diff_types.rs`) but NO CLI subcommand binds them. `GenerationCmd` enum in `subcmd_args.rs` only has `List` and `Gc`. The `diff_cmd.rs` compares two config files, not two generation snapshots.

**Fix**: Add `Diff { from: u32, to: u32 }` variant to `GenerationCmd`, wire to `diff_resource_sets()`.

---

### F2: `forjar undo-destroy` replay is not implemented

**Spec claim** (main spec line 183, 02-generation-undo.md):
> `forjar undo-destroy [--machine X]` — replay destroy log to recreate resources

**Reality** (`undo.rs:308`):
```
println!("Replay not yet implemented — use `forjar apply` with the original config.");
```
The command exists, parses destroy-log.jsonl, categorizes resources, handles `--dry-run`, but the ACTUAL REPLAY returns `Ok(())` without doing anything.

**Fix**: Implement replay loop that calls `cmd_apply` with reconstructed config from `config_fragment`.

---

### F3: Incremental ingest with cursor not implemented

**Spec claim** (01-sqlite-query-engine.md lines 271-282):
> Incremental ingest via `IngestCursor` table, skip already-ingested data

**Reality**: `IngestCursor` type defined in `sqlite_schema_types.rs` with `is_ingested()` and `mark_ingested()` methods. But `ingest_state_dir()` in `ingest.rs` NEVER imports or uses it. Every ingest is a full rebuild. The `ingest_cursor` table doesn't exist in the actual schema.

**Fix**: Wire `IngestCursor` into `ingest_state_dir()`, add mtime/hash check to skip unchanged files.

---

### F4: Content policy does not exist

**Spec claim** (10-security-model.md, main spec line 188):
> Content policy enforcement

**Reality**: No content policy enforcement found anywhere. Only path blocking (`deny_paths`) and secret scanning exist.

**Fix**: Either implement content policy or remove the claim.

---

### F5: Dual-digest "single pass" is false

**Spec claim** (main spec line 72, 03-idempotency-drift.md):
> Dual-digest computed in a single pass

**Reality**: BLAKE3 and SHA-256 are computed in separate function calls. `hash_file()` uses BLAKE3 only; `sha256_digest()` is called separately for OCI. No single-pass streaming dual-hash exists.

**Fix**: Either implement a `DualHasher` that streams data through both algorithms simultaneously, or change "single pass" to "both computed".

---

## Exaggerations

### E1: "Second apply is always a no-op (<1ms, zero I/O)"

**Spec claim** (main spec line 73):
> Second apply is always a no-op (<1ms, zero I/O)

**Reality**: Second apply is a no-op in terms of *convergence actions* (no SSH, no package installs). But it still:
- Reads and parses the YAML config file
- Reads all `state.lock.yaml` files from disk
- Recomputes `hash_desired_state()` for every resource
- Compares hashes

"Zero I/O" is false. "Zero remote I/O" or "zero mutation I/O" would be accurate.

---

### E2: "flock advisory locking" is actually PID-file locking

**Spec claim** (main spec line 283, L13):
> `state/.forjar.lock` (flock) prevents concurrent modification
> advisory file locking (`flock`)

**Reality** (`core/state/mod.rs:217-249`): The code writes a PID file, checks if the PID is still running, and removes stale locks. There is NO `flock()` syscall, NO `fcntl` locking, NO `fs2::FileExt::lock_exclusive()`. PID-file locking has race conditions that true flock avoids.

---

### E3: Secret providers are type definitions only

**Spec claim** (main spec line 188, 10-security-model.md):
> `{{ secrets.* }}` with env/file/SOPS providers

**Reality**: `SecretProvider` enum has `Env`, `File`, `Sops`, `Op` variants in `security_types.rs`. But `secrets.rs` only implements Age encryption. The env/file/SOPS/Op dispatch logic is not wired. Template expansion of `{{ secrets.* }}` is planned, not production.

---

### E4: pepita overlayfs not implemented

**Spec claim** (main spec line 60, 04-multi-machine-ops.md):
> pepita → unshare + cgroups v2 + overlayfs (~10-50ms)

**Reality** (`transport/pepita.rs`): Uses `unshare` with PID/mount/net namespaces. Cgroups v2 limits applied. But overlayfs is NOT mounted — the `filesystem: "overlay"` config field exists in types but no code reads it. Pepita uses simple mount namespace, not overlay.

NOTE: `sandbox_exec.rs` DOES use overlayfs for container builds (store sandbox), but that's the store layer, not the pepita transport.

---

### E5: Kani harnesses are bounded toy models (mostly)

**Spec claim** (09-provable-design-by-contract.md):
> Kani real-code harnesses

**Reality**: 17 `#[kani::proof]` harnesses in `kani_proofs.rs`. Deprecation notice (lines 11-18) calls them "abstract-model harnesses operating on simplified state." Real-code harnesses exist (4 of 17) but use tiny bounds (4 packages, 8-char strings). Useful but overstated.

---

### E6: Runtime contracts are on spec wrappers, not production code

**Spec claim** (09-provable-design-by-contract.md):
> All critical-path functions have `#[ensures]` contracts

**Reality**: `#[requires]`/`#[ensures]` attributes exist only on `spec_*()` wrapper functions inside `verus_spec.rs` (8 functions). Actual production functions like `hash_desired_state()`, `determine_present_action()` have NO contracts. Custom `#[contract]` macros exist on a few `tripwire/hasher.rs` functions but these are metadata tags, not runtime checks.

---

## Stale Claims (Code Has Changed)

### S1: L4 "Destroy State Cleanup Bug" — already fixed

**Spec claim** (08-known-limitations.md L4):
> `cleanup_state_files()` removes `state.lock.yaml` even when some resources failed.
> Fix required (Phase 5, FJ-2005)

**Reality** (`destroy.rs:212-218`): The fix is already implemented. On full success, `cleanup_state_files()` removes entire locks. On partial failure, `cleanup_succeeded_entries()` only removes succeeded resource entries. The spec still lists this as an open bug.

---

### S2: L16 "No Selective Apply" — resource_filter exists

**Spec claim** (08-known-limitations.md L16):
> `forjar apply` converges the entire resource DAG. There is no `--resource` or `--type` filter.

**Reality** (`apply_args.rs:498`): `pub resource_filter: Option<String>` field exists on ApplyArgs. A resource filter IS available.

---

## Unverified Claims (No Evidence)

### U1: Performance target <50ms for `forjar query`

No benchmark or test measures query latency. The target is aspirational.

### U2: state.db < 1MB for 3 machines

No test assertion validates this size bound.

### U3: pepita namespace creation in 10-50ms

No benchmark measures pepita startup latency.

---

## SQLite Schema Gaps

### F6: FTS5 schema doesn't match spec

**Spec claim** (01-sqlite-query-engine.md lines 123-126):
> FTS5 fields: `resource_id, resource_type, path, packages, content_preview` with porter tokenizer

**Reality** (`core/store/db.rs:61-65`):
```sql
CREATE VIRTUAL TABLE IF NOT EXISTS resources_fts USING fts5(
    resource_id, resource_type, status, path, details_json,
    content='resources', content_rowid='id'
);
```
Missing: `packages`, `content_preview`. Extra: `status`, `details_json`. No porter tokenizer. Indexes raw JSON (violates spec's own "never index raw JSON" rule).

### F7: Multiple spec-defined tables don't exist

| Table | In Spec | In Code |
|-------|---------|---------|
| `destroy_log` | Yes (01-sqlite lines 90-101) | NO |
| `drift_findings` | Yes (01-sqlite lines 76-87) | NO |
| `events_fts` | Yes (01-sqlite lines 128-131) | NO |
| `idx_resources_status` | Yes (01-sqlite line 106) | NO |
| `ingest_cursor` | Yes (01-sqlite lines 274-281) | NO |

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

| Priority | Item | Severity |
|----------|------|----------|
| 1 | Wire `forjar generation diff` CLI command | F1 |
| 2 | Implement undo-destroy replay loop | F2 |
| 3 | Fix "flock" language → "PID-file locking" | E2 |
| 4 | Fix "zero I/O" → "zero remote I/O" | E1 |
| 5 | Create missing SQLite tables (destroy_log, drift_findings, events_fts) | F7 |
| 6 | Wire IngestCursor for incremental ingest | F3 |
| 7 | Implement FTS5 field extraction per spec | F6 |
| 8 | Remove L4 from known-limitations (already fixed) | S1 |
| 9 | Remove L16 from known-limitations (resource_filter exists) | S2 |
| 10 | Implement pepita overlayfs mount | E4 |
| 11 | Add performance benchmarks for query/pepita targets | U1-U3 |
| 12 | Implement secret provider dispatch (env/file/SOPS) | E3 |
| 13 | Fix dual-digest "single pass" claim | F5 |
| 14 | Remove content policy claim or implement it | F4 |
