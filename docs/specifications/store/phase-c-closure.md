# Phase C: Closures & Locking (FJ-1310–FJ-1314)

**Status**: ✅ Complete
**Implementation**: `src/core/store/lockfile.rs`, `src/core/store/pin_tripwire.rs`

---

## 1. Lock File Format (FJ-1310)

```yaml
# forjar.inputs.lock.yaml — analogous to flake.lock / Cargo.lock
schema: "1.0"
pins:
  nginx: { provider: apt, version: "1.24.0-1ubuntu1", hash: "blake3:abc123..." }
  my-recipe: { type: recipe, git_rev: "a1b2c3d4e5f6", hash: "blake3:def456..." }
```

## 2. CLI Commands (FJ-1311–FJ-1313)

```bash
forjar pin                   # pin all inputs to current versions
forjar pin --update nginx    # re-resolve and re-hash specific pin
forjar pin --update          # update all pins
forjar pin --check           # CI gate — fail if lock file is stale
```

## 3. Tripwire Integration (FJ-1314)

Input pinning extends tripwire upstream detection. During `forjar apply`, the lock file is compared against resolved inputs — if an input has changed (upstream release, git push, file edit), forjar warns before applying. Reuses state management patterns from `src/core/state/mod.rs` (atomic writes, lock file diffing).
