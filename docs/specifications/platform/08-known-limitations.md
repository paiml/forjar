# 08: Known Limitations

> Honest boundaries, falsification results, and what doesn't work.

**Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## L1: Verus Proofs Are Model-Level, Not Implementation-Level

The Verus specs in `verus_spec.rs` prove properties of a simplified `ResourceState { desired_hash, current_hash, converged }` model. The real system has a dual-hash architecture:

- **Plan-time**: `hash_desired_state()` = BLAKE3 of config struct fields joined by `\0`
- **Executor**: `rl.hash` = set by resource handlers after apply (usually `hash_desired_state`, but handler-dependent)

If any resource handler stores a different hash (e.g., a live state query hash), the idempotency loop could produce false negatives (unnecessary re-applies). The Verus model doesn't capture this.

**Closing the gap** (Phase 6, FJ-2006): Audit all handlers to verify they store `hash_desired_state` as `rl.hash`, extend the Verus model to handle dual-hash domains.

---

## L2: Undo and Destroy Are Best-Effort for Side-Effect Resources

| Resource Type | Undo Reliable? | Undo-Destroy Reliable? | Why |
|--------------|---------------|----------------------|-----|
| File (content:) | Yes | Yes | Inline content is deterministic |
| File (source:) | Mostly | Mostly | External path may change or disappear |
| Package | Mostly | Best-effort | Version may float between destroy and restore |
| Service | Yes | Best-effort | Runtime state (PIDs, sockets) not captured |
| Cron | Yes | Yes | Declarative schedule, fully reproducible |
| Mount | Yes | Mostly | Source device must still exist |
| Docker/Pepita | Yes | Yes | Ephemeral by design |
| GPU | Yes | Yes | Config-only, no persistent state |
| Task | **No** | **No** | Arbitrary commands — side effects are irreversible |
| User | **No** | **No** | Home directory, file ownership, groups — data loss |
| Network | **No** | **No** | Routing state, iptables rules — complex side effects |
| Model | **No** | **No** | Downloaded artifacts — re-download may differ |
| Recipe | **No** | **No** | Composed from multiple resource types |

---

## L3: No Distributed Atomicity

Multi-machine operations are **not atomic**. If machine B fails mid-undo while machine A succeeded:
- Machine A is at generation N-1
- Machine B is partially between N and N-1
- There is no `--abort` — rolling back A while B is broken creates worse inconsistency
- Recovery is always forward: fix B, then `forjar undo --resume`

This is a deliberate design choice, not a missing feature. Distributed 2PC adds coordinator complexity, reduces availability, and fails in the same network-partition scenarios that caused the original failure.

---

## L4: ~~Destroy State Cleanup Bug~~ (RESOLVED)

**Fixed in FJ-2005**: `cleanup_succeeded_entries()` now removes only succeeded resource entries from lock files on partial failure. Full lock removal only occurs when all resources destroy successfully. See `destroy.rs:66-90`.

---

## L5: CQRS Integrity Depends on Flat Files

`state.db` is a derived read model that can be fully reconstructed from flat files. If corrupted or deleted, `forjar ingest` rebuilds it from:
- `state/<machine>/state.lock.yaml` — resources table
- `state/<machine>/events.jsonl` — events table
- `state/<machine>/destroy-log.jsonl` — destroy_log table
- `state/generations/*/` — generations table

**Invariant**: No write-side data may exist only in SQLite. Every mutation must be written to a flat file first, then ingested into SQLite.

---

## L6: Config Snapshot Storage

The git-first approach (store `git_ref`, retrieve config via `git show`) fails when:
- The git repo is rebased/force-pushed and old commits disappear
- The user runs `git gc --aggressive` and prunes unreferenced objects
- The apply happens outside a git repo

For these cases, the full YAML fallback (`config_snapshot`) is stored. Users who always commit before apply pay zero storage cost. Users who apply from dirty trees accumulate full YAML copies — mitigated by `gc_generations()`.

---

## L7: Package Layer Requires Reference System

Path 1's package-to-layer conversion runs `dpkg -L <pkg>` to enumerate installed files. This requires a system where those packages are already installed. For cross-architecture or clean-room builds, this means either:
- A running container with the base image (falls back to container transport)
- A pepita namespace with debootstrapped rootfs
- A cached package file list from a previous build

This is a bootstrap problem, not a fundamental limitation. After the first build, the file list is cached in the store.

---

## L8: Non-Deterministic Build Layers

Path 2 (pepita sandbox) executes arbitrary scripts. If those scripts fetch floating dependencies (`pip install torch` without version pin), the output is non-deterministic across builds. The layer will have a different BLAKE3 hash each time, defeating caching.

**Mitigation**: `build.deterministic: true` flag enables strict mode — network disabled, all inputs must be declared, reproducibility verified by building twice and comparing hashes.

---

## L9: SHA-256 vs BLAKE3 Dual Digest

OCI mandates SHA-256. Forjar uses BLAKE3 internally. We compute both, which means:
- Layer blobs are hashed twice (BLAKE3 for store, SHA-256 for OCI)
- No significant performance impact (both are fast), but it's conceptual overhead
- Cannot use BLAKE3 digests in OCI manifests — the ecosystem doesn't support it yet

---

## L10: Multi-Arch Build Constraints

Cross-architecture builds (e.g., building arm64 image on x86_64 host) require either QEMU user-mode emulation or a native build machine. Pepita uses `unshare(2)` which runs native arch only. Options:
- QEMU binfmt_misc registration + pepita (slower, works anywhere)
- SSH transport to a native-arch machine (fast, requires hardware)
- Path 1 only: direct tar assembly doesn't execute anything — arch-agnostic

---

## L11: No BuildKit/Dockerfile Compatibility

This system does not parse or execute Dockerfiles. Users with existing Dockerfiles must migrate to the declarative `type: image` resource format. This is intentional — Dockerfiles are imperative and non-reproducible by design. Consequences:
- No drop-in replacement for `docker build`
- Migration effort for existing Docker-based workflows
- Ecosystem tools that expect Dockerfiles (CI/CD, GitHub Actions) need adaptation

---

## L12: CQRS Invariant Is Policy, Not Mechanism

L5 states that no write-side data may exist only in SQLite. But there is no compile-time or runtime enforcement preventing a developer from writing directly to `state.db`. The invariant is a documented policy.

**Mitigation**: The `state.db` write path should be a single module (`core/store/db.rs`) that only accepts data from the ingest pipeline. Direct SQL INSERT/UPDATE outside this module should be flagged by code review. A `forjar ingest --verify` command can compare state.db contents against flat files and report any orphaned rows.

---

## L13: State Directory Requires Exclusive Access

The state directory (`state/`) uses PID-file locking (`.forjar.lock` containing the process PID) to prevent concurrent modification by multiple forjar processes. Stale locks from crashed processes are detected and cleaned up automatically. This does NOT prevent:
- External tools modifying state files directly
- Two users on different machines sharing a state directory via NFS (PID-file locking is host-local)
- A process that ignores the lock file

Note: This is NOT `flock(2)` advisory locking — it is a PID-file with liveness check via `/proc`. The PID-file approach has a small TOCTOU race window between checking and writing, but is simpler and works across all Linux filesystems.

For shared-state scenarios, use a CI/CD pipeline as the single writer, or use a lockfile-aware wrapper.

---

## L14: Verus Model-Implementation Correspondence Is Unverifiable

The Verus proofs (Tier 3 in [09-provable-design-by-contract.md](09-provable-design-by-contract.md)) prove properties of a `PlannerState` model. The spec assumes the model matches `determine_present_action`. But:
- If someone adds a new branch to `determine_present_action`, the Verus model may not be updated
- The proof still passes (it's about the model), but the real code may no longer satisfy the proved property
- No automated mechanism detects model-implementation divergence

**Mitigation**: The runtime contracts (Tier 1) encode the same postcondition as the Verus proof. If the implementation diverges from the model, the `#[debug_ensures]` contract will catch it during testing. This is a belt-and-suspenders approach, not a formal model-implementation proof.

---

## L15: No Backward/Forward State Compatibility

The spec introduces new state files (`destroy-log.jsonl`), new generation metadata fields (`git_ref`, `config_snapshot`, `action`), and a new SQLite schema. There is no versioned migration strategy.

**Compatibility rules**:
1. **Missing files are empty**: If `destroy-log.jsonl` doesn't exist, `destroy_log` table is empty (not an error)
2. **Unknown YAML fields are ignored**: `serde_yaml_ng` deserialization uses `#[serde(default)]` for all new fields. Old lock files work with new Forjar.
3. **Schema version in state.db**: `PRAGMA user_version` stores the schema version. On ingest, if version < current, run migrations. If version > current (downgrade), refuse with a clear error.
4. **No automatic downgrade**: Upgrading is safe (migrations run). Downgrading requires `forjar ingest --rebuild` which reconstructs state.db from flat files using the old schema.

---

## L16: ~~No Selective Apply~~ (RESOLVED)

**Fixed**: `forjar apply --resource <id>` is now supported via the `resource_filter` field on `ApplyArgs`. This filters convergence to a single resource (without resolving upstream dependencies — the user is responsible for ensuring dependencies are already converged).
