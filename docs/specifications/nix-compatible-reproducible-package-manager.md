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

Purity classification reuses existing detection logic:

- **Version pins**: presence of `version:` field on package resources
- **Pipe-to-shell detection**: SAF scoring logic (`curl|bash`, `wget|sh` pattern matching)
- **Sandbox declaration**: presence of `sandbox:` config block
- **Input closure**: all transitive `depends_on` and `recipe` references

### 3.4 Validation Command (FJ-1306)

```bash
forjar validate --check-recipe-purity

# Output:
# my-package:     Pure (0)    — version pinned, sandboxed, all inputs hashed
# my-service:     Pinned (1)  — version pinned, not sandboxed
# legacy-setup:   Impure (3)  — contains curl|bash pattern
```

### 3.5 Input Closure Tracking (FJ-1307)

Each resource's input closure is the set of all transitive inputs that affect its output. The closure hash is `composite_hash()` over all input hashes, sorted lexicographically. Two resources with identical closures produce identical store paths.

---

## 4. Input Pinning (FJ-1310–FJ-1314)

### 4.1 Lock File Format (FJ-1310)

```yaml
# forjar.inputs.lock.yaml — analogous to flake.lock / Cargo.lock
schema: "1.0"
generated_at: "2026-03-02T10:00:00Z"
generator: "forjar 0.10.0"
pins:
  nginx:
    provider: apt
    version: "1.24.0-1ubuntu1"
    hash: "blake3:abc123..."
    pinned_at: "2026-03-01T12:00:00Z"
  my-recipe:
    type: recipe
    git_rev: "a1b2c3d4e5f6"
    hash: "blake3:def456..."
    pinned_at: "2026-03-01T12:00:00Z"
  config-template:
    type: file
    source: "templates/nginx.conf"
    hash: "blake3:789abc..."
    pinned_at: "2026-03-01T12:00:00Z"
```

### 4.2 CLI Commands (FJ-1311–FJ-1313)

```bash
forjar pin                   # pin all inputs to current versions
forjar pin --update nginx    # re-resolve and re-hash specific pin
forjar pin --update          # update all pins
forjar pin --check           # CI gate — fail if lock file is stale
```

### 4.3 Tripwire Integration (FJ-1314)

Input pinning extends tripwire upstream detection. During `forjar apply`, the lock file is compared against resolved inputs. If an input has changed (upstream release, git push, file edit), forjar warns before applying:

```
WARN: input 'nginx' version changed: 1.24.0-1ubuntu1 → 1.24.0-2ubuntu1
      Run 'forjar pin --update nginx' to accept, or pin to exact version.
```

This reuses the existing state management patterns from `src/core/state/mod.rs` — atomic writes, lock file diffing, per-machine state tracking.

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
    sandbox:
      level: full          # full | network-only | none
      allowed_paths:        # additional read-only bind mounts
        - /usr/share/ca-certificates
      memory_mb: 2048
      cpus: 4.0
      timeout: 600         # build timeout in seconds
```

### 5.3 Sandbox Lifecycle (FJ-1316)

```
1. Create namespace     (unshare — reuses pepita.rs:ensure_namespace)
2. Mount overlay        (lower=store inputs, upper=tmpfs — reuses pepita/mod.rs:apply_present)
3. Bind inputs          (read-only bind mounts of store dependencies)
4. Apply cgroup limits  (reuses pepita.rs:apply_cgroup_limits)
5. Execute build        (pipe bashrs-purified script to bash inside namespace)
6. Extract outputs      (copy from overlay upper to staging)
7. Hash outputs         (hash_directory from hasher.rs)
8. Store                (move to /var/forjar/store/<hash>/content/)
9. Write meta.yaml      (record input closure, references)
10. Destroy namespace   (reuses pepita.rs:cleanup_namespace)
```

### 5.4 Seccomp Profile (FJ-1317)

Default seccomp profile for pure builds: deny `connect(2)` (no network), deny `mount(2)` (no filesystem escape), deny `ptrace(2)` (no debugging other processes). Allow standard build syscalls (read, write, open, exec, mmap, etc.). Extends the existing `seccomp: true` field on `Resource` (`src/core/types/resource.rs:223`).

---

## 6. Binary Cache (FJ-1320–FJ-1324)

### 6.1 Cache Architecture

```
Substitution order:
1. Local store     (/var/forjar/store/<hash>/)
2. Cache sources   (SSH remotes, optional HTTP read-only)
3. Build from scratch
```

### 6.2 SSH Cache Transport (FJ-1320)

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

```bash
# Automate steps 1-3
forjar convert --reproducible

# Output:
# ✓ Added version pins to 12 packages
# ✓ Enabled store for 8 resources
# ✓ Generated forjar.inputs.lock.yaml with 20 pins
# ⚠ 3 resources remain impure (curl|bash patterns — manual fix required)
```

### 8.3 Reproducibility Score (FJ-1329)

```bash
forjar validate --check-reproducibility-score

# Output:
# Reproducibility score: 72/100
#   Pure resources:        5/15 (33%)
#   Pinned resources:      7/15 (47%)
#   Constrained resources: 2/15 (13%)
#   Impure resources:      1/15 (7%)
#   Store coverage:        8/15 (53%)
#   Input lock coverage:  12/20 (60%)
```

### 8.4 Cookbook Recipes (FJ-1330–FJ-1332)

Recipes 63–67 in forjar-cookbook demonstrate reproducible patterns:

- **Recipe 63**: Version-pinned apt packages with store caching
- **Recipe 64**: Cargo build with full sandbox and input lock
- **Recipe 65**: Multi-machine deploy from shared SSH cache
- **Recipe 66**: Reproducibility score gate in CI pipeline
- **Recipe 67**: Profile generation rollback workflow

---

## 9. Implementation Phases

| Phase | Tickets | Priority | Depends On | Scope |
|-------|---------|----------|------------|-------|
| **A** | FJ-1300–FJ-1309 | Highest | Nothing | Store path derivation, metadata, profile generations, `store:` field on Resource, reference scanning, purity static analysis, `validate --check-recipe-purity`, input closure tracking, monotonicity enforcement, store read/write |
| **B** | FJ-1310–FJ-1314 | High | Phase A | `forjar.inputs.lock.yaml` schema/parser, `forjar pin` / `pin --update` / `pin --check`, tripwire input drift detection |
| **C** | FJ-1315–FJ-1319 | Medium | Phase A + pepita | `sandbox:` config block, sandbox lifecycle, seccomp BPF profile, read-only bind mounts, timeout/resource limits |
| **D** | FJ-1320–FJ-1327 | Lower | Phase A + B | SSH cache transport, optional HTTP source, substitution protocol, `forjar cache` CLI, cache verification, GC roots, mark-and-sweep, `forjar store gc` |
| **E** | FJ-1328–FJ-1332 | Parallel | Phase A + B | `forjar convert --reproducible`, `validate --check-reproducibility-score`, cookbook recipes 63–67 |

---

## 10. Key Design Decisions

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

---

## 11. Non-Goals

### Not a Nix replacement

Forjar does not replicate Nix's package ecosystem (nixpkgs) or its functional evaluation model. The expression layer is YAML templates producing bashrs-purified shell — not a general-purpose functional language. Forjar will never evaluate Nix expressions or import from nixpkgs. The overlap is architectural (content-addressed store, hermetic builds), not operational.

### Machine-level artifact manager, not a system package manager

The store model caches **resource outputs at the machine level** — the result of applying a forjar resource to a target. It does not maintain a package index, perform dependency resolution, or implement SAT solving. It orchestrates existing package managers (apt, cargo, uv) and caches their outputs. Think "build cache with addressable generations," not "apt replacement."

### Not a container registry

Store entries are resource state snapshots, not runnable OCI images. There is no `FROM` semantics, no layer composition, no image manifest. Pepita overlays are build-time isolation, not distributable artifacts.

### No required HTTP daemon

Forjar does not run or manage an HTTP server. The SSH cache is the primary transport, consistent with forjar's sovereignty model. An optional read-only HTTP source allows consuming caches served by external infrastructure (nginx, S3, any static host) — but forjar never opens a port or manages a listener.

---

## 12. Invariants

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
