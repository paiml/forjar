# Phase L: Execution Layer (FJ-1358–FJ-1365)

**Status**: 🔲 Spec only — not yet implemented

---

## 1. Overview

The store specification (Phases A–K) defines types, validation, and benchmarks. This phase documents the execution gaps — where types exist but actual shell commands, file operations, and network transport have not been implemented.

## 2. Execution Gaps

| Gap | Ticket | Depends On | Description |
|-----|--------|------------|-------------|
| Provider invocation | FJ-1359 | Phase F | `provider.rs` types → actual `apt install`/`cargo install`/`nix build` shell commands |
| Cache SSH transport | FJ-1360 | Phase E | `cache.rs` SubstitutionResult → actual `scp`/`rsync` over SSH |
| Derivation builder | FJ-1361 | Phase F + D | `derivation.rs` validate_dag → sequential build execution in pepita |
| Store diff/sync | FJ-1362 | Phase F | `store_diff` types → actual file comparison + rsync |
| Convert --apply | FJ-1363 | Phase H | `convert` pipeline → actual FAR unpack + apply |
| Pin resolution | FJ-1364 | Phase C | `lockfile.rs` staleness → actual version resolution from providers |
| GC sweep | FJ-1365 | Phase E | `gc.rs` collect_roots → actual `rm -rf` of unreachable entries |

## 3. Per-Ticket Detail

Details to be expanded in Commit 5 (FJ-1358).
