# Phase L: Execution Layer (FJ-1358–FJ-1365)

**Status**: 🔧 Partial — execution bridges implemented, CLI wiring pending

---

## 1. Overview

The store specification (Phases A–K) defines types, validation, and benchmarks. This phase documents the execution gaps — where types exist but actual shell commands, file operations, and network transport have not been implemented. Each gap is assigned a ticket with preconditions, generated shell commands, I8 validation requirements, rollback strategy, and test plan.

## 2. Execution Gaps

| # | Gap | Ticket | Depends On | Priority |
|---|-----|--------|------------|----------|
| 1 | Provider invocation | FJ-1359 | Phase F types | High |
| 2 | Cache SSH transport | FJ-1360 | Phase E types | Medium |
| 3 | Derivation builder | FJ-1361 | FJ-1359, Phase D sandbox | High |
| 4 | Store diff/sync | FJ-1362 | Phase F provider types | Low |
| 5 | Convert --apply | FJ-1363 | Phase H convert types, FAR | Medium |
| 6 | Pin resolution | FJ-1364 | Phase C lockfile types | Medium |
| 7 | GC sweep | FJ-1365 | Phase E GC types | Low |

---

## 3. FJ-1359: Provider Invocation

**Preconditions**: `provider.rs` provider enum, `sandbox.rs` SandboxConfig, I8 gate in transport layer.

**Shell commands generated** (per provider):

| Provider | Command | Capture |
|----------|---------|---------|
| `apt` | `apt-get install -y --download-only <pkg>=<version>` | `/var/cache/apt/archives/*.deb` |
| `cargo` | `cargo install --root $out <crate>@<version>` | `$out/bin/*` |
| `uv` | `uv pip install --target $out <pkg>==<version>` | `$out/**` |
| `nix` | `nix build --print-out-paths <flake-ref>` | `/nix/store/<hash>-<name>/**` |
| `docker` | `docker create <image>` + `docker export <id>` | Filesystem tarball |
| `tofu` | `tofu -chdir=<dir> output -json` | JSON outputs |
| `terraform` | `terraform -chdir=<dir> output -json` | JSON outputs |

**I8 requirement**: All generated commands pass `validate_before_exec()`. Provider scripts are assembled in Rust (not templated YAML) and validated before transport dispatch.

**Rollback strategy**: Provider invocation is side-effect-free (downloads to temp dir). On failure: delete temp dir. On success: `hash_directory(temp) → rename to store`.

**Test plan**:
- Unit: mock provider CLI output, verify capture paths
- Integration: `apt` provider on CI (containerized)
- Property: identical inputs → identical store hashes (within provider determinism limits)

---

## 4. FJ-1360: Cache SSH Transport

**Preconditions**: `cache.rs` CacheConfig + SubstitutionResult types, SSH transport in `transport/ssh.rs`.

**Shell commands generated**:

| Operation | Command |
|-----------|---------|
| Check remote | `ssh <cache-host> test -d /var/forjar/store/<hash>` |
| Pull entry | `rsync -az <cache-host>:/var/forjar/store/<hash>/ <local-store>/<hash>/` |
| Push entry | `rsync -az <local-store>/<hash>/ <cache-host>:/var/forjar/store/<hash>/` |
| Verify remote | `ssh <cache-host> blake3sum /var/forjar/store/<hash>/content/**` |

**I8 requirement**: All rsync/ssh commands pass bashrs validation. Host/path values are sanitized (no shell metacharacters).

**Rollback strategy**: Pull writes to temp dir first, then atomic rename. Push is additive (store is write-once). Failed push leaves partial entry that `cache verify` will detect and clean.

**Test plan**:
- Unit: mock SSH transport, verify command generation
- Integration: SSH cache pull/push between two containers
- Chaos: kill mid-transfer, verify partial entries detected by `cache verify`

---

## 5. FJ-1361: Derivation Builder

**Preconditions**: `derivation.rs` Derivation type + DAG validation, `sandbox_exec.rs` sandbox lifecycle, FJ-1359 provider invocation.

**Execution flow**:
1. `validate_dag(graph)` → topological order
2. For each derivation in order:
   a. `collect_input_hashes()` → resolve all inputs
   b. `derivation_closure_hash()` → check store (cache hit = skip)
   c. Create pepita namespace: `unshare --mount --pid --net`
   d. Bind inputs read-only: `mount --bind -o ro <store>/<hash>/content <sandbox>/inputs/<name>`
   e. Create writable output: `mkdir -p <sandbox>/$out`
   f. Execute bashrs-validated script inside sandbox
   g. `hash_directory($out)` → atomic move to store
   h. Write `meta.yaml` with closure + provenance
   i. Destroy namespace

**I8 requirement**: Derivation `script` field is validated via `validate_or_purify()` before execution. Template expansion (`{{inputs.*}}`) happens before validation.

**Rollback strategy**: Each derivation is atomic. On failure: destroy namespace, delete temp output. Completed derivations persist (immutable store entries). Resume from next uncached derivation.

**Test plan**:
- Unit: DAG resolution, closure hash computation
- Integration: 3-step derivation chain in pepita container
- Property: identical inputs + script → identical output hash

---

## 6. FJ-1362: Store Diff/Sync

**Preconditions**: `store_diff.rs` diff types, `meta.yaml` provenance fields, provider invocation (FJ-1359).

**Execution flow**:

`forjar store diff <hash>`:
1. Read `meta.yaml` → extract `origin_provider`, `origin_ref`
2. Re-invoke provider to capture current upstream
3. `hash_directory(current)` → compare against `store_hash`
4. If different: report diff summary (files added/removed/changed)

`forjar store sync <hash> --apply`:
1. `diff` step above
2. Re-import upstream → new store entry
3. Replay each `derived_from` derivation step → new derived entries
4. Update profile symlink

**I8 requirement**: Provider re-invocation commands pass bashrs validation.

**Rollback strategy**: Diff is read-only. Sync creates new store entries (old ones preserved). Profile update is atomic symlink.

**Test plan**:
- Unit: diff computation from two store entries
- Integration: import → modify upstream → diff → sync → verify
- Edge: sync with broken derivation chain (missing intermediate)

---

## 7. FJ-1363: Convert --apply

**Preconditions**: `convert.rs` conversion pipeline, `far.rs` FAR encode/decode.

**Execution flow** (`forjar convert --reproducible --apply`):
1. Parse config → identify conversion targets
2. Steps 1-3 (automated): add version pins, enable store, generate lock file
3. For each `store: true` resource: invoke provider (FJ-1359) → store entry
4. Pack store entries as FAR archives
5. Write updated config YAML

**I8 requirement**: Generated version-pinned commands pass bashrs validation.

**Rollback strategy**: Original config backed up as `forjar.yaml.bak`. New config written atomically. Store entries are additive.

**Test plan**:
- Unit: conversion pipeline produces correct YAML
- Integration: convert a real config from Constrained → Pinned
- Roundtrip: convert → validate → plan shows no changes

---

## 8. FJ-1364: Pin Resolution

**Preconditions**: `lockfile.rs` LockFile + Pin types, provider CLIs available.

**Execution flow** (`forjar pin`):
1. Parse config → collect all inputs
2. For each input, resolve current version:
   - `apt`: `apt-cache policy <pkg>` → installed version
   - `cargo`: `cargo search <crate>` → latest version
   - `nix`: `nix eval <flake-ref>.version` → version string
3. Hash resolved version: `blake3::hash(provider + name + version)`
4. Write `forjar.inputs.lock.yaml` atomically

`forjar pin --check`:
1. Load lock file
2. Re-resolve all inputs
3. Compare hashes → report stale pins
4. Exit 1 if any stale (CI gate)

**I8 requirement**: Version resolution commands pass bashrs validation.

**Rollback strategy**: Lock file written atomically (temp + rename). `--check` is read-only.

**Test plan**:
- Unit: pin generation from mock provider output
- Integration: pin apt packages in container
- CI: `pin --check` fails on stale lock file

---

## 9. FJ-1365: GC Sweep

**Preconditions**: `gc.rs` GC root collection, `reference.rs` reference scanning, `profile.rs` profile generations.

**Execution flow** (`forjar store gc`):
1. Collect GC roots: current profile, last N generations, lock file pins, `.gc-roots/` symlinks
2. Walk roots: follow `references` in each root's `meta.yaml`
3. Mark all reachable store entries as live
4. Sweep: for each store entry not marked live:
   - Log entry hash + size to GC journal
   - `rm -rf /var/forjar/store/<hash>`
5. Report: entries removed, space reclaimed

**I8 requirement**: `rm -rf` commands pass bashrs validation. Path is validated to be under `/var/forjar/store/` (no path traversal).

**Rollback strategy**: GC is destructive by design (removes unreachable entries). Mitigations:
- `--dry-run` previews without deleting
- GC journal enables manual recovery if backup exists
- `--keep-generations N` preserves rollback capability
- Never automatic — always explicit user invocation

**Test plan**:
- Unit: root collection, reachability graph, sweep selection
- Integration: create entries → remove references → GC → verify deleted
- Safety: verify GC never removes entries reachable from current profile
- Edge: concurrent apply during GC (lock file prevents)

---

## 10. Implementation Order

Recommended implementation sequence based on dependencies:

1. **FJ-1359** (Provider invocation) — foundation for all others
2. **FJ-1364** (Pin resolution) — enables lock file creation
3. **FJ-1361** (Derivation builder) — depends on provider + sandbox
4. **FJ-1360** (Cache SSH transport) — parallel with derivation builder
5. **FJ-1363** (Convert --apply) — depends on provider + FAR
6. **FJ-1362** (Store diff/sync) — depends on provider re-invocation
7. **FJ-1365** (GC sweep) — lowest priority, depends on everything stable
