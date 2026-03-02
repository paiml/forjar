# Nix-Compatible Reproducible Package Management

**Version**: 0.1.0-spec
**Status**: Draft
**Author**: Noah Gift / Pragmatic AI Labs
**Date**: 2026-03-02
**Ticket Range**: FJ-1300–FJ-1399

---

## 1. Vision & Motivation

Forjar treats machines as knowable systems — BLAKE3 hashing, tripwire drift detection, and pepita kernel namespace isolation already make every apply auditable and traceable. What's missing is **content-addressed artifact caching**: `apt install curl` produces different binaries across time, mirrors, and architectures. The machine is knowable, but its inputs are not.

Nix solved this with content-addressed stores and hermetic builds. Forjar brings that insight into its SSH-first, YAML-first, multi-machine world — without becoming Nix.

### 1.1 Problem Statement

Today, forjar's package resources call `apt install` or `cargo install` at apply time. The result depends on mirror state, package index freshness, and upstream release timing. Two identical applies on two machines can produce different binaries. This violates the "knowable system" thesis.

### 1.2 Key Insight

Nix's core contribution is not its language — it's the **content-addressed store**. Hash all inputs, use that hash as the storage key, and you get reproducibility (same inputs → same output), cacheability (build once, substitute everywhere), rollback (previous generations persist), and garbage collection (unreachable entries are safe to delete).

### 1.3 Competitive Position

| Dimension | Nix | Guix | Docker | Ansible | **Forjar (proposed)** |
|-----------|-----|------|--------|---------|-----------------------|
| Content-addressed store | Yes (SHA256) | Yes (SHA256) | Layer hashes | No | **Yes (BLAKE3)** |
| Hermetic builds | Full (sandbox) | Full (sandbox) | Dockerfile isolation | No | **Incremental (4 levels)** |
| Binary cache | HTTP (cachix) | HTTP (substitutes) | Registry (OCI) | No | **SSH-first + optional HTTP** |
| Expression language | Nix language | Guile Scheme | Dockerfile | Jinja2 | **YAML templates + bashrs** |
| Rollback | Generations | Generations | Image tags | No | **Profile generations** |
| Multi-machine | NixOps (abandoned) | `guix deploy` | Swarm/K8s | Inventory | **Native (SSH fleet)** |
| Bare-metal first | Yes | Yes | No | Yes | **Yes** |
| Dependency count | ~1500 (nixpkgs) | ~1200 | ~200 Go modules | ~50 pip | **16 crates** |

### 1.4 Expression Layer

Forjar already has an expression language: **YAML templates expanded by Rust, producing bashrs-purified POSIX shell**. Recipe expansion (`src/core/recipe/expansion.rs`) resolves `{{inputs.*}}` templates across all `Option<String>` fields. bashrs purifies the generated shell into provably safe POSIX. This is the derivation model — inputs flow through templates into purified build scripts. No new language is needed; the store model extends what exists.

---

## 2. Forjar Store Model (FJ-1300–FJ-1304)

### 2.1 Store Layout

```
/var/forjar/store/
├── <blake3-hash-1>/
│   ├── meta.yaml          # Input manifest, build info, timestamps
│   └── content/            # Actual build output (files, dirs)
├── <blake3-hash-2>/
│   ├── meta.yaml
│   └── content/
└── .gc-roots/              # Symlinks to live store paths
```

Consistent with existing forjar paths (`/var/forjar/tripwire/`, `/run/forjar/`).

### 2.2 Path Derivation (FJ-1300)

Store paths reuse `composite_hash()` from `src/tripwire/hasher.rs:86`:

```
store_path = composite_hash([recipe_hash, input_hashes..., arch, provider])
```

- `recipe_hash`: BLAKE3 of recipe YAML after template expansion
- `input_hashes`: BLAKE3 of all transitive inputs (packages, files, source tarballs)
- `arch`: target machine architecture
- `provider`: package provider (`apt`, `cargo`, `uv`)

### 2.3 Store Metadata (FJ-1301)

```yaml
# /var/forjar/store/<hash>/meta.yaml
schema: "1.0"
store_hash: "blake3:abc123..."
recipe_hash: "blake3:def456..."
input_hashes: ["blake3:111...", "blake3:222..."]
arch: "x86_64"
provider: "apt"
created_at: "2026-03-02T10:00:00Z"
generator: "forjar 0.10.0"
references: []          # other store paths this entry depends on
```

### 2.4 Profile Generations (FJ-1302)

Profile symlinks under `/var/forjar/profiles/`: `system-1 → store/<hash-a>/content`, `system-2 → store/<hash-b>/content`, `current → system-2`. Rollback: `forjar rollback` re-points `current` to the previous generation. Atomic via `rename(2)`.

### 2.5 YAML Integration (FJ-1303)

```yaml
resources:
  my-package:
    type: package
    machine: web-01
    provider: apt
    packages: [nginx]
    version: "1.24.0"
    store: true              # NEW: enable content-addressed caching
```

The `store: true` field on `Resource` (extending `src/core/types/resource.rs`) opts a resource into the store model. Without it, resources behave as today.

### 2.6 Reference Scanning (FJ-1304)

Store entries track references to other store paths via the `references` field in `meta.yaml`. The GC uses this to build a reachability graph from roots. References are discovered by scanning output files for store path hashes (conservative scanning, same approach as Nix).

---

## 3. Recipe Purity Model (FJ-1305–FJ-1309)

### 3.1 Purity Levels

| Level | Name | Definition | Example |
|-------|------|------------|---------|
| 0 | **Pure** | All inputs hashed, sandboxed, deterministic | `store: true` + `sandbox: full` + pinned version |
| 1 | **Pinned** | Version-locked but not sandboxed | `version: "1.24.0"` + `store: true` |
| 2 | **Constrained** | Provider-scoped but floating version | `provider: apt` + `packages: [nginx]` (no version) |
| 3 | **Impure** | Unconstrained network/side-effect access | `curl \| bash` install scripts |

### 3.2 Purity Monotonicity

A recipe's purity level is the **maximum** (least pure) of all its transitive dependencies. A pure recipe that depends on a constrained input is at most constrained. This is enforced statically.

### 3.3 Static Analysis (FJ-1305)

Purity classification reuses existing detection: `version:` field presence, SAF `curl|bash`/`wget|sh` pattern matching, `sandbox:` block presence, transitive `depends_on`/`recipe` closure.

### 3.4 Validation Command (FJ-1306)

`forjar validate --check-recipe-purity` reports each resource's purity level (Pure/Pinned/Constrained/Impure) with reason.

### 3.5 Input Closure Tracking (FJ-1307)

Each resource's input closure is the set of all transitive inputs. The closure hash is `composite_hash()` over all input hashes, sorted lexicographically. Identical closures produce identical store paths.

---

## 4. Input Pinning (FJ-1310–FJ-1314)

### 4.1 Lock File Format (FJ-1310)

```yaml
# forjar.inputs.lock.yaml — analogous to flake.lock / Cargo.lock
schema: "1.0"
generated_at: "2026-03-02T10:00:00Z"
generator: "forjar 0.10.0"
pins:
  nginx: { provider: apt, version: "1.24.0-1ubuntu1", hash: "blake3:abc123..." }
  my-recipe: { type: recipe, git_rev: "a1b2c3d4e5f6", hash: "blake3:def456..." }
  config-template: { type: file, source: "templates/nginx.conf", hash: "blake3:789abc..." }
```

### 4.2 CLI Commands (FJ-1311–FJ-1313)

```bash
forjar pin                   # pin all inputs to current versions
forjar pin --update nginx    # re-resolve and re-hash specific pin
forjar pin --update          # update all pins
forjar pin --check           # CI gate — fail if lock file is stale
```

### 4.3 Tripwire Integration (FJ-1314)

Input pinning extends tripwire upstream detection. During `forjar apply`, the lock file is compared against resolved inputs — if an input has changed (upstream release, git push, file edit), forjar warns before applying. Reuses state management patterns from `src/core/state/mod.rs` (atomic writes, lock file diffing).

---

## 5. Build Sandboxing (FJ-1315–FJ-1319)

### 5.1 Pepita Extension

Build sandboxing extends the existing pepita kernel namespace isolation (`src/transport/pepita.rs`, `src/resources/pepita/mod.rs`). Today, pepita provides:

- PID/mount/network namespaces via `unshare(2)` / `nsenter(1)`
- cgroups v2 resource limits (memory, CPU)
- overlayfs copy-on-write layers
- Network namespace isolation

The store sandbox adds:

- **Read-only bind mounts** for input dependencies
- **Minimal `/dev`** (null, zero, urandom only)
- **seccomp BPF** syscall filtering (no network, no mount, no ptrace)
- **tmpfs `/tmp`** for build scratch space

### 5.2 Sandbox Configuration (FJ-1315)

```yaml
resources:
  my-build:
    type: package
    provider: cargo
    packages: [my-tool]
    version: "0.5.0"
    store: true
    sandbox: { level: full, memory_mb: 2048, cpus: 4.0, timeout: 600 }
```

### 5.3 Sandbox Lifecycle (FJ-1316)

Create namespace (`pepita.rs:ensure_namespace`) → mount overlay (lower=store inputs, upper=tmpfs) → bind inputs read-only → apply cgroup limits (`pepita.rs:apply_cgroup_limits`) → execute bashrs-purified build script → extract outputs → `hash_directory()` from `hasher.rs` → move to `/var/forjar/store/<hash>/content/` → write `meta.yaml` → destroy namespace (`pepita.rs:cleanup_namespace`).

### 5.4 Seccomp Profile (FJ-1317)

Default for pure builds: deny `connect(2)`, `mount(2)`, `ptrace(2)`. Allow standard build syscalls (read, write, open, exec, mmap). Extends `seccomp: true` on `Resource` (`src/core/types/resource.rs:223`).

---

## 6. Binary Cache (FJ-1320–FJ-1324)

### 6.1 Cache Transport (FJ-1320)

Primary transport. Sovereign — no external dependencies, no tokens, no TLS certificates. Uses forjar's existing SSH transport.

```yaml
cache:
  sources:
    - type: ssh
      host: cache.internal
      user: forjar
      path: /var/forjar/cache
    - type: local
      path: /var/forjar/store
    - type: http                              # optional read-only (FJ-1321)
      url: https://cache.internal/forjar-store
```

The optional HTTP source supports CI environments and fleet scale. Forjar does **not** run an HTTP daemon — the server is external (nginx, S3, any static host). Analogous to Nix's `substituters`: forjar consumes HTTP cache, it doesn't serve it. Content-addressed hashes provide integrity — no auth required.

### 6.3 Substitution Protocol (FJ-1322)

Compute store hash from input closure → check local store → check cache sources in order (SSH, HTTP) → build from scratch (sandbox if configured) → store result, optionally push to cache.

### 6.4 CLI (FJ-1323–FJ-1324)

```bash
forjar cache list                    # list local store entries
forjar cache push <remote>           # push local store to SSH remote
forjar cache pull <hash>             # pull specific entry from cache
forjar cache verify                  # verify all store entries (re-hash)
```

---

## 7. Garbage Collection (FJ-1325–FJ-1327)

### 7.1 GC Roots (FJ-1325)

A store entry is **live** if reachable from any GC root:

- Current profile symlink (`/var/forjar/profiles/current`)
- All profile generations (configurable retention: keep last N)
- Active lock file references (`forjar.inputs.lock.yaml` pins)
- Entries referenced by `/var/forjar/store/.gc-roots/` symlinks

### 7.2 Reference Scanning (FJ-1326)

Conservative mark-and-sweep:

1. **Mark**: walk all GC roots, follow `references` in each `meta.yaml`, mark as live
2. **Sweep**: any store entry not marked is dead — eligible for deletion

### 7.3 GC Command (FJ-1327)

```bash
forjar store gc                      # delete all unreachable entries
forjar store gc --dry-run            # preview what would be deleted
forjar store gc --older-than 90d     # age filter
forjar store gc --keep-generations 5 # generation retention
```

---

## 8. Recipe Conversion Strategy (FJ-1328–FJ-1332)

### 8.1 Five-Step Conversion Ladder

| Step | Action | Purity Result | Automated |
|------|--------|---------------|-----------|
| 1 | Add `version:` pins to all packages | Constrained → Pinned | Yes |
| 2 | Add `store: true` to cacheable resources | Enables store | Yes |
| 3 | Generate `forjar.inputs.lock.yaml` | Pins all inputs | Yes |
| 4 | Add `sandbox:` blocks | Pinned → Pure | Manual |
| 5 | Replace imperative hooks with declarative resources | Full purity | Manual |

### 8.2 Automated Conversion (FJ-1328)

`forjar convert --reproducible` automates steps 1-3: adds version pins, enables `store: true`, generates `forjar.inputs.lock.yaml`. Reports remaining impure resources (curl|bash patterns) that require manual intervention.

### 8.3 Reproducibility Score (FJ-1329)

`forjar validate --check-reproducibility-score` outputs a 0-100 score based on: percentage of resources at each purity level, store coverage, and input lock coverage.

### 8.4 Cookbook Recipes (FJ-1330–FJ-1332)

Recipes 63–67: version-pinned apt with store (63), cargo sandbox + input lock (64), multi-machine SSH cache deploy (65), reproducibility score CI gate (66), profile generation rollback (67).

---

## 9. Nix Binary Cache Import (FJ-1333–FJ-1336)

Pure Rust implementation of Nix's data formats and protocols. No `nix` binary, no daemon — forjar speaks the wire protocol natively and re-hashes everything into its own BLAKE3 store.

### 9.1 Scope

Three Nix layers are reimplemented in Rust (~1200 LOC total):

| Layer | LOC | Purpose |
|-------|-----|---------|
| NAR reader/writer (FJ-1333) | ~300 | Deterministic file tree serialization (Nix's archive format since 2003) |
| NARInfo parser (FJ-1334) | ~150 | Binary cache metadata (store path, references, signature, nar hash) |
| Store path computation (FJ-1335) | ~200 | SHA256 input hashing → `/nix/store/<nix-base32>-<name>` path derivation |
| Binary cache client (FJ-1336) | ~400 | Fetch .narinfo + .nar.xz from cache.nixos.org, verify ed25519 sig, unpack |

**Not reimplemented**: Nix language evaluator (~50k LOC), nixpkgs repository, Nix daemon. Forjar consumes Nix outputs, not expressions.

### 9.2 Provider Integration

```yaml
resources:
  ripgrep:
    type: package
    provider: nix
    packages: ["nixpkgs#ripgrep"]
    version: "14.1.0"
    store: true
    nix:
      nixpkgs_rev: "abc123..."        # pinned nixpkgs commit
      system: "x86_64-linux"
```

The `provider: nix` path: compute Nix store path from package + nixpkgs rev → fetch .narinfo from cache.nixos.org → verify ed25519 signature → download .nar.xz → decompress → unpack NAR into staging → rewrite `/nix/store/` references → hash with BLAKE3 → install to `/var/forjar/store/`.

### 9.3 Reference Rewriting

Nix binaries contain hardcoded `/nix/store/<hash>-<name>` paths. These must be rewritten when moving into the forjar store. ELF binaries use `patchelf --set-rpath` / `--set-interpreter`. Non-ELF files (scripts, configs) use byte-level scan and replace. Store path lengths are matched via truncated BLAKE3 hex (32 chars = 128 bits, collision-resistant) under `/fjr/store/` (11 chars, matches `/nix/store/` length) for in-place binary patching.

### 9.4 New Dependencies

| Crate | Purpose |
|-------|---------|
| `sha2` | SHA256 for Nix hash verification (Nix protocol requirement) |
| `ed25519-dalek` | Signature verification for narinfo (~15k LOC, pure Rust) |
| `xz2` | Decompress .nar.xz from cache.nixos.org |
| `zstd` (optional) | Decompress .nar.zst from newer caches |

Four crates. Nix itself pulls ~1500 packages.

---

## 10. Forjar Archive Format (FJ-1337–FJ-1340)

Forjar's sovereign package format — FAR (Forjar ARchive). Replaces NAR for forjar-native packages with BLAKE3 verified streaming, content-defined chunking, and built-in provenance.

### 10.1 Why Not NAR

| Dimension | NAR (2003) | FAR |
|-----------|------------|-----|
| Hash | SHA256 (no tree mode) | BLAKE3 (verified streaming, keyed mode) |
| Dedup | Whole-archive transfer | Content-defined chunking — delta transfers |
| Metadata | Scan full archive | Manifest-first — metadata without full download |
| Compression | xz (slow decompress) | Zstd (10x faster decompress, dictionary support) |
| Verification | Hash entire NAR, then compare | BLAKE3 tree mode — verify each 256KB chunk independently |
| Resume | No — restart from zero | Chunk-level resume, skip verified chunks |
| Provenance | None in archive | Recipe hash, input closure, builder identity inline |

### 10.2 Archive Layout (FJ-1337)

```
┌──────────────────────────┐
│ Magic: "forjar-ar-1\0"   │  12 bytes
│ Manifest length (u64 LE) │  8 bytes
├──────────────────────────┤
│ Manifest (YAML, zstd)    │  Metadata, file list, chunk map, provenance
├──────────────────────────┤
│ Chunk table (binary)     │  Per-chunk: blake3 hash + offset + length
├──────────────────────────┤
│ Chunks 0..N (zstd)       │  Content-defined boundaries (~64KB, Rabin fingerprint)
├──────────────────────────┤
│ Signature (ed25519)       │  64 bytes over manifest blake3 hash
└──────────────────────────┘
```

### 10.3 Manifest Schema (FJ-1338)

```yaml
schema: "1.0"
name: "ripgrep"
version: "14.1.0"
arch: "x86_64"
store_hash: "blake3:abc123..."
tree_hash: "blake3:def456..."          # BLAKE3 tree root
recipe_hash: "blake3:789abc..."
input_closure: ["blake3:aaa...", "blake3:bbb..."]
references: ["blake3:ccc..."]
provenance:
  sandbox_level: "full"
  builder_identity: "forjar@build-01"
  built_at: "2026-03-02T10:00:00Z"
  git_commit: "abc123"
files:
  - { path: "bin/rg", size: 5242880, mode: "0755", hash: "blake3:eee...", chunks: [0,1,2,3,4] }
  - { path: "share/man/man1/rg.1", size: 12345, mode: "0644", hash: "blake3:fff...", chunks: [5] }
total_chunks: 6
compressed_size: 1834567
```

### 10.4 BLAKE3 Verified Streaming (FJ-1339)

BLAKE3's tree hashing splits content into 256KB chunks, each hashed independently, combined in a binary tree. This enables: parallel verification (8 chunks on 8 cores), partial verification (verify chunk N without chunks 0..N-1), incremental transfer (reuse chunks from previous version), and resume (verify what you have, fetch what's missing).

### 10.5 Content-Defined Chunking (FJ-1340)

Rabin fingerprinting at ~64KB boundaries. When a package updates 14.1.0 → 14.1.1, only changed chunks transfer. Chunk hashes are BLAKE3 — matching chunks between old and new versions are skipped during `forjar cache push/pull`. This is the same dedup model as `casync`/`restic`, integrated into the archive format.

### 10.6 CLI

```bash
forjar archive pack <store-hash>     # pack store entry into .far
forjar archive unpack <file.far>     # unpack .far into store
forjar archive inspect <file.far>    # print manifest without unpacking
forjar archive verify <file.far>     # verify chunk hashes + signature
```

---

## 11. Implementation Phases

| Phase | Tickets | Priority | Depends On | Scope |
|-------|---------|----------|------------|-------|
| **A** | FJ-1300–FJ-1309 | Highest | Nothing | Store model, purity analysis, profile generations, input closure tracking |
| **B** | FJ-1310–FJ-1314 | High | Phase A | Input pinning, lock file, `forjar pin`, tripwire integration |
| **C** | FJ-1315–FJ-1319 | Medium | Phase A + pepita | Sandbox config, lifecycle, seccomp BPF, read-only bind mounts |
| **D** | FJ-1320–FJ-1327 | Lower | Phase A + B | SSH/HTTP cache, substitution protocol, GC roots, mark-and-sweep |
| **E** | FJ-1328–FJ-1332 | Parallel | Phase A + B | `forjar convert --reproducible`, reproducibility score, cookbook 63–67 |
| **F** | FJ-1333–FJ-1336 | Medium | Phase D | Nix binary cache import: NAR reader, narinfo parser, store path computation |
| **G** | FJ-1337–FJ-1340 | Lower | Phase A + D | FAR format: archive layout, manifest, BLAKE3 verified streaming, chunking |

---

## 12. Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Hash algorithm | BLAKE3 | Already used everywhere (`hasher.rs`), faster than SHA256, keyed mode available |
| Store location | `/var/forjar/store/` | Sovereign, consistent with `/var/forjar/tripwire/` |
| Primary cache transport | SSH | SSH-first design, no external deps, existing transport |
| Optional cache transport | HTTP (read-only) | CI/fleet pragmatism — forjar consumes, doesn't serve |
| Purity model | 4 levels | Incremental adoption — don't force purity on day one |
| Sandbox implementation | Pepita extension | Reuse existing `unshare`/`nsenter`/cgroups/overlay |
| Expression layer | YAML templates + bashrs | Already exists (`expansion.rs` + bashrs purification) — no new language |
| Lock format | YAML | Consistent with all forjar config |
| Profile generations | Symlink rotation | Atomic `rename(2)` switch + instant rollback |
| GC strategy | Explicit only | Never auto-delete — user controls store lifecycle |
| Nix import | Native Rust protocol | ~1200 LOC, 4 crates — no `nix` binary dependency |
| Archive format | FAR (forjar-native) | BLAKE3 verified streaming, content-defined chunking, manifest-first |
| Reference rewriting | Truncated BLAKE3 + patchelf | `/fjr/store/` matches `/nix/store/` path length for binary patching |

---

## 13. Non-Goals

**Not a Nix replacement.** Forjar consumes Nix binary cache outputs natively (Section 9) but never evaluates Nix expressions or replicates nixpkgs. The expression layer is YAML templates + bashrs-purified shell.

**Machine-level artifact manager, not a system package manager.** The store caches resource outputs — no package index, no dependency resolution, no SAT solving. It orchestrates existing package managers (apt, cargo, uv, nix) and caches their outputs.

**Not a container registry.** Store entries are resource state snapshots, not runnable OCI images. No `FROM` semantics, no layer composition.

**No required HTTP daemon.** SSH cache is primary. Optional read-only HTTP/nix-cache source for consuming external caches — forjar never opens a port.

---

## 14. Invariants

### Store Contracts

- **Write-once**: once `/var/forjar/store/<hash>/` is created, its contents are immutable. The hash *is* the content. Modification is corruption.
- **Hash integrity**: `hash_directory(store_path/content/) == store_hash` must hold at all times. `forjar cache verify` checks this.
- **Atomic creation**: store entries are built in a staging directory and atomically renamed into place (same pattern as `save_lock()` in `src/core/state/mod.rs:27`).

### Purity Contracts

- **Monotonicity**: a resource's purity level is always ≥ the maximum purity level of its transitive dependencies. A pure resource cannot depend on an impure input.
- **Closure determinism**: two resources with identical input closures always produce identical store hashes (given the same architecture and provider).
- **Classification stability**: a resource's purity level cannot improve without changing its definition. Adding `store: true` alone does not make a resource pure — it must also have pinned versions and (for level 0) a sandbox.

### Lock File Contracts

- **Completeness**: `forjar pin --check` fails if any resource input is not represented in the lock file.
- **Freshness**: lock file `hash` fields match the current resolved state of all inputs. Stale hashes are detected and reported.
- **Atomic update**: lock file writes use the same temp-file + rename pattern as state locks (`src/core/state/mod.rs`).
