# Nix-Compatible Reproducible Package Management

**Version**: 0.1.0-spec
**Status**: Draft
**Author**: Noah Gift / Pragmatic AI Labs
**Date**: 2026-03-02
**Ticket Range**: FJ-1300–FJ-1399

---

## 1. Vision & Motivation

Forjar treats machines as knowable systems — BLAKE3 hashing, tripwire drift detection, and pepita kernel namespace isolation already make every apply auditable and traceable. What's missing is **content-addressed artifact caching**: `apt install curl` produces different binaries across time, mirrors, and architectures. The machine is knowable, but its inputs are not.

The content-addressed store model (pioneered by Nix, Guix, and others) solves this. Forjar brings that insight into its SSH-first, YAML-first, multi-machine world as a native capability — Nix compatibility is optional, not foundational.

### 1.1 Problem Statement

Today, forjar's package resources call `apt install` or `cargo install` at apply time. The result depends on mirror state, package index freshness, and upstream release timing. Two identical applies on two machines can produce different binaries. This violates the "knowable system" thesis.

### 1.2 Key Insight

The key insight is the **content-addressed store**. Hash all inputs, use that hash as the storage key, and you get reproducibility (same inputs → same output), cacheability (build once, substitute everywhere), rollback (previous generations persist), and garbage collection (unreachable entries are safe to delete). This works with any provider — apt, cargo, uv, or optionally nix.

### 1.3 Competitive Position

| Dimension | Nix | Docker | Ansible | Terraform | **Forjar** |
|-----------|-----|--------|---------|-----------|-----------|
| Content-addressed store | SHA256 | Layer hashes | No | No | **BLAKE3** |
| Hermetic builds | Full | Dockerfile | No | No | **4 purity levels** |
| Cache | HTTP | OCI registry | No | No | **SSH-only** |
| Expression | Nix lang | Dockerfile | Jinja2 | HCL | **YAML + bashrs** |
| Multi-machine | NixOps | Swarm/K8s | Inventory | Workspaces | **Native SSH fleet** |
| Import-and-own | No | No | No | No | **Yes (derivations)** |

### 1.4 Expression Layer

YAML templates expanded by Rust, producing bashrs-purified POSIX shell. Recipe expansion (`src/core/recipe/expansion.rs`) resolves `{{inputs.*}}` templates. This is the derivation model — no new language needed.

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

### 2.3 Store Metadata with Provenance (FJ-1301)

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
references: []
provenance:
  origin_provider: "nix"                  # what tool originally produced this
  origin_ref: "nixpkgs#ripgrep@14.1.0"   # upstream identifier for diff/sync
  origin_hash: "sha256:def456..."         # upstream's native hash (for diffing)
  origin_rev: "abc123..."                 # upstream version pin (nixpkgs rev, docker tag, tofu state serial)
  derived_from: ["blake3:aaa..."]         # parent store entries (derivation chain)
  derivation_depth: 1                     # 0 = direct import, N = N derivation steps from import
```

The `provenance` block is the traceability chain. `origin_provider` + `origin_ref` record where the artifact first entered the forjar store. `derived_from` tracks the derivation DAG. `derivation_depth: 0` means direct import; depth > 0 means the artifact was derived from other store entries. This enables `forjar diff` and `forjar sync` (Section 10.7).

### 2.4 Profile Generations (FJ-1302)

Profile symlinks under `/var/forjar/profiles/`: `system-1 → store/<hash-a>/content`, `system-2 → store/<hash-b>/content`, `current → system-2`. Rollback: `forjar rollback` re-points `current` to the previous generation. Atomic via `rename(2)`.

### 2.5 YAML Integration (FJ-1303)

`store: true` field on `Resource` (extending `src/core/types/resource.rs`) opts into the store model. Without it, resources behave as today.

### 2.6 Reference Scanning (FJ-1304)

Store entries track references via `meta.yaml`. The GC follows these to build a reachability graph. References discovered by scanning output files for store path hashes (conservative scanning).

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

Extends pepita kernel namespace isolation (`src/transport/pepita.rs`, `src/resources/pepita/mod.rs`). Existing: PID/mount/net namespaces, cgroups v2, overlayfs. Store sandbox adds: read-only bind mounts for inputs, minimal `/dev`, seccomp BPF (`connect`/`mount`/`ptrace` denied), tmpfs `/tmp`.

**Config** (FJ-1315): `sandbox: { level: full, memory_mb: 2048, cpus: 4.0, timeout: 600 }` on any resource with `store: true`.

**Lifecycle** (FJ-1316): create namespace → overlay mount (lower=inputs, upper=tmpfs) → bind inputs read-only → cgroup limits → bashrs-purified build → extract outputs → `hash_directory()` → store → destroy namespace. All steps reuse existing pepita functions.

---

## 6. Binary Cache (FJ-1320–FJ-1324)

### 6.1 Cache Transport (FJ-1320)

SSH-only. Sovereign — no HTTP client crate, no tokens, no TLS certificates. Uses forjar's existing SSH transport.

```yaml
cache:
  sources:
    - type: ssh
      host: cache.internal
      user: forjar
      path: /var/forjar/cache
    - type: local
      path: /var/forjar/store
```

### 6.2 Substitution Protocol (FJ-1322)

Compute store hash from input closure → check local store → check SSH cache sources → build from scratch (sandbox if configured) → store result, optionally push to cache.

### 6.3 CLI (FJ-1323–FJ-1324)

```bash
forjar cache list                    # list local store entries
forjar cache push <remote>           # push local store to SSH remote
forjar cache pull <hash>             # pull specific entry from cache
forjar cache verify                  # verify all store entries (re-hash)
```

---

## 7. Garbage Collection (FJ-1325–FJ-1327)

**GC roots** (FJ-1325): current profile symlink, profile generations (keep last N), lock file pins, `.gc-roots/` symlinks.

**Mark-and-sweep** (FJ-1326): walk roots, follow `references` in `meta.yaml`, mark as live. Unreachable entries are dead.

**CLI** (FJ-1327): `forjar store gc` (delete unreachable), `--dry-run`, `--older-than 90d`, `--keep-generations 5`. GC is never automatic.

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

## 9. Universal Provider Import (FJ-1333–FJ-1340)

Any external tool can seed the forjar store. Each provider shells out to its native CLI, captures outputs, BLAKE3-hashes them, and stores the result. After import, all store entries are identical — provider-agnostic, distributable as FAR over SSH. Zero new crate dependencies for any provider.

### 9.1 Supported Providers (FJ-1333–FJ-1336)

| Provider | CLI | Capture Method | Example |
|----------|-----|----------------|---------|
| `apt` | `apt install` | Package files via dpkg manifest | `packages: [nginx]` |
| `cargo` | `cargo install` | Binary output in `$CARGO_HOME/bin/` | `packages: [ripgrep]` |
| `uv` | `uv pip install` | Virtualenv contents | `packages: [flask]` |
| `nix` | `nix build --print-out-paths` | Output tree in `/nix/store/` | `packages: ["nixpkgs#ripgrep"]` |
| `docker` | `docker export` | Filesystem snapshot from container | `image: "ubuntu:24.04"` |
| `tofu` | `tofu output -json` | State outputs (IPs, IDs, configs) | `source: "infra/"` |
| `terraform` | `terraform output -json` | State outputs | `source: "infra/"` |

All providers follow the same flow: invoke CLI → copy output to staging → `hash_directory()` → move to `/var/forjar/store/<hash>/content/` → write `meta.yaml`. Same pattern as existing `cargo` provider in `src/resources/package.rs`.

### 9.2 Provider-Specific Notes

**Nix** (FJ-1334): rewrites `/nix/store/` paths via `patchelf` + bashrs `sed`, re-hashed after rewriting. **Docker** (FJ-1335): `docker create` + `docker export` → unpack, strip Docker metadata, hash, store — the Dockerfile → pepita path. **Terraform/OpenTofu** (FJ-1336): outputs are structured data (IPs, IDs), stored as YAML in `content/outputs.yaml`, used as derivation inputs. **apr** (FJ-1337): model artifacts (gguf, safetensors, apr format) imported via `apr pull`, checksummed, stored with model lineage in provenance. **alimentar** (FJ-1338): dataset snapshots imported, hashed, versioned — derivations transform (filter, augment, split) inside pepita.

After import, all store entries are provider-agnostic and distribute identically via SSH.

---

## 10. Store Derivations (FJ-1341–FJ-1345)

Take one or more store entries as inputs, apply a transformation inside a pepita sandbox, produce a new store entry. This is how imported artifacts become forjar-native — the **import once, own forever** model.

### 10.2 Derivation Model (FJ-1341)

```yaml
resources:
  ml-rootfs:
    type: derivation
    store: true
    sandbox: { level: full, memory_mb: 4096, cpus: 8.0, timeout: 1800 }
    inputs:
      base_rootfs: { store: "blake3:aaa..." }      # Docker-imported Ubuntu
      cuda_toolkit: { store: "blake3:bbb..." }      # Nix-imported CUDA
      model_weights: { store: "blake3:ccc..." }      # forjar-built model
      config: { resource: "nginx-config" }           # another resource's output
    script: |
      # bashrs-purified — runs inside pepita sandbox
      cp -r {{inputs.base_rootfs}}/* $out/
      cp -r {{inputs.cuda_toolkit}}/* $out/usr/local/
      cp -r {{inputs.model_weights}}/* $out/opt/models/
      cp {{inputs.config}} $out/etc/nginx/nginx.conf
```

**Key fields:**
- `type: derivation` — new resource type
- `inputs:` — map of named inputs, each referencing a store hash or another resource
- `script:` — bashrs-purified shell executed inside pepita sandbox
- `$out` — the output directory (hashed after build, becomes the new store entry)

### 10.3 Derivation Lifecycle (FJ-1342)

Resolve inputs → compute closure hash → check store (hit = substitute, skip build) → create pepita namespace → bind inputs read-only → execute bashrs script (writes `$out`) → `hash_directory($out)` → atomic move to store → write `meta.yaml` (closure, provenance) → destroy namespace. Steps 4–10 reuse the sandbox lifecycle from Section 5.

### 10.4 Any Provider → Pepita Pipeline (FJ-1343)

```yaml
resources:
  ubuntu-base:                               # Step 1: import rootfs
    type: package
    provider: docker
    image: "ubuntu:24.04"
    store: true
  ml-rootfs:                                  # Step 2: derive combined rootfs
    type: derivation
    store: true
    sandbox: { level: full }
    inputs:
      base: { resource: "ubuntu-base" }
    script: |
      cp -r {{inputs.base}}/* $out/
      # add packages, configs, models...
  ml-sandbox:                                 # Step 3: boot pepita from derived rootfs
    type: pepita
    depends_on: [ml-rootfs]
    chroot_dir: "/var/forjar/store/{{ml-rootfs.store_hash}}/content"
    memory_limit: 8589934592
    cpuset: "0-7"
    netns: true
```

The provider is interchangeable: Docker → store → derivation → pepita. Or Nix → store → derivation → pepita. Or apt → store → derivation → pepita. The derivation is the universal adapter.

### 10.5 Derivation Chains (FJ-1344)

Derivations reference other derivations as inputs, forming a DAG (evaluated bottom-up via `depends_on`). Each step produces a new immutable store entry, independently cacheable and substitutable.

### 10.6 Import Once, Own Forever

The default user story: import from any provider (nix, docker, tofu, terraform, apt), derive your customization on top, pack as FAR, distribute over SSH. The source provider is never invoked again — your FAR is sovereign. The `meta.yaml` provenance chain records the origin for traceability, but the artifact's identity is its own BLAKE3 hash.

### 10.7 Upstream Diff and Sync (FJ-1345)

Provenance enables diffing against upstream. `meta.yaml` records `origin_provider`, `origin_ref`, `origin_hash`:

```bash
forjar store diff <hash>               # diff store entry against upstream origin
forjar store sync <hash> --apply       # re-import upstream, replay derivation chain
```

`diff` re-invokes the origin provider, captures current upstream, compares. `sync --apply` re-imports, then replays each `derived_from` step to produce an updated FAR. The MLOps story: upstream model weights change → `forjar store sync` detects → re-derives fine-tuned model → re-packs as FAR.

### 10.8 MLOps / AI Engineering Integration

Derivation + provenance directly supports the aprender/alimentar ecosystem:

- **Model artifacts** (apr, gguf, safetensors): imported via `forjar import apr`, stored with BLAKE3 checksums. Provenance tracks source (HuggingFace, apr registry), quantization, fine-tuning lineage.
- **Data artifacts** (alimentar): dataset snapshots hashed and versioned. Derivations transform (filter, augment, split) inside pepita. Full data lineage in provenance.
- **Training pipelines**: derivation chains: data (alimentar) → preprocess → train (aprender/trueno) → evaluate → model (apr) → deploy (pepita). Each step is a store entry, independently cacheable, fully traceable.

### 10.9 CLI

```bash
forjar import docker ubuntu:24.04       # import Docker image into store
forjar import nix nixpkgs#ripgrep       # import Nix package into store
forjar import tofu ./infra/             # import Terraform/OpenTofu outputs
forjar import apr meta-llama/Llama-3    # import model into store
forjar store list --show-provider       # list entries with source provider
forjar store diff <hash>                # diff against upstream origin
forjar store sync <hash> --apply        # re-import and re-derive
```

---

## 11. Forjar Archive Format (FJ-1346–FJ-1349)

Forjar's sovereign package format — FAR (Forjar ARchive). BLAKE3 verified streaming, content-defined chunking, built-in provenance.

### 11.1 Archive Layout (FJ-1346)

Advantages over NAR (2003): BLAKE3 verified streaming (vs SHA256), content-defined chunking for delta transfers, manifest-first metadata access, zstd compression (10x faster decompress than xz), chunk-level resume, inline provenance.

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
│ Signature (age)           │  age identity signature over manifest blake3 hash
└──────────────────────────┘
```

Zstd compression and age signatures use crates already in the dependency tree (transitive via `age v0.11`). BLAKE3 is a direct dependency. **Zero new crates for the FAR format.**

### 11.2 Manifest Schema (FJ-1347)

```yaml
schema: "1.0"
name: "ripgrep"
version: "14.1.0"
arch: "x86_64"
store_hash: "blake3:abc123..."
tree_hash: "blake3:def456..."          # BLAKE3 tree root (verified streaming)
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
total_chunks: 6
compressed_size: 1834567
```

### 11.3 Streaming and Chunking (FJ-1348–FJ-1349)

BLAKE3 tree hashing: 256KB chunks hashed independently, combined in binary tree — parallel verification, partial verification, incremental transfer, resume. Content-defined chunking via Rabin fingerprinting at ~64KB boundaries — only changed chunks transfer on version updates.

### 11.4 CLI

```bash
forjar archive pack <store-hash>     # pack store entry into .far
forjar archive unpack <file.far>     # unpack .far into store
forjar archive inspect <file.far>    # print manifest without unpacking
forjar archive verify <file.far>     # verify chunk hashes + signature
```

---

## 12. Implementation Phases

| Phase | Tickets | Priority | Depends On | Scope |
|-------|---------|----------|------------|-------|
| **A** | FJ-1300–FJ-1309 | Highest | Nothing | Store model, purity analysis, profile generations, input closure tracking |
| **B** | FJ-1310–FJ-1314 | High | Phase A | Input pinning, lock file, `forjar pin`, tripwire integration |
| **C** | FJ-1315–FJ-1319 | Medium | Phase A + pepita | Sandbox config, lifecycle, seccomp BPF, read-only bind mounts |
| **D** | FJ-1320–FJ-1327 | Lower | Phase A + B | SSH cache, substitution protocol, GC roots, mark-and-sweep |
| **E** | FJ-1328–FJ-1332 | Parallel | Phase A + B | `forjar convert --reproducible`, reproducibility score, cookbook 63–67 |
| **F** | FJ-1333–FJ-1340 | Medium | Phase A | Universal provider import (nix, docker, tofu, terraform) |
| **G** | FJ-1341–FJ-1345 | High | Phase A + C | Store derivations, derivation chains, Dockerfile→pepita pipeline |
| **H** | FJ-1346–FJ-1349 | Lower | Phase A + D | FAR format: archive layout, manifest, BLAKE3 streaming, chunking |

---

## 13. Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Hash | BLAKE3 | Already in tree, faster than SHA256, keyed mode, verified streaming |
| Store | `/var/forjar/store/` | Sovereign, consistent with `/var/forjar/tripwire/` |
| Cache | SSH only | No HTTP client crate. Sovereign. |
| Purity | 4 levels | Incremental adoption |
| Sandbox | Pepita extension | Reuse `unshare`/`nsenter`/cgroups/overlay |
| Expression | YAML templates + bashrs | Existing — no new language |
| Providers | All optional, all equal | apt/cargo/uv/nix/docker/tofu — shell out, capture, re-hash |
| Derivations | `type: derivation` | Inputs + bashrs + pepita → new store entry. Universal adapter. |
| Archive | FAR | BLAKE3 streaming, zstd, age signing — all existing deps |
| New crates | Zero | `sha2`/`zstd` transitive via `age`. No `ed25519-dalek`, no `xz2`. |

---

## 14. Non-Goals

**No external tool is required.** Every provider (nix, docker, tofu, terraform) is optional. The store, derivations, FAR, purity model, cache, and GC all work with `provider: apt` alone.

**Not a package manager.** No package index, no dependency resolution, no SAT solving. Orchestrates existing tools and caches their outputs.

**No new crate dependencies.** Entire spec uses existing deps: `blake3`, `age` (transitively: `sha2`, `zstd`, `chacha20poly1305`), `serde_yaml_ng`. No `ed25519-dalek`, no `xz2`/`liblzma`, no HTTP client crates.

**No HTTP daemon.** SSH only. Forjar never opens a port.

---

## 15. Invariants

**Store**: Write-once (hash *is* identity; modification is corruption). Hash integrity (`hash_directory(content/) == store_hash`) checked by `forjar cache verify`. Atomic creation via temp-dir + rename (same as `save_lock()` in `src/core/state/mod.rs:27`).

**Purity**: Monotonicity — a resource's purity ≥ max purity of its transitive deps (pure cannot depend on impure). Closure determinism — identical input closures → identical store hashes. Classification stability — purity cannot improve without definition changes.

**Derivations**: Input immutability (read-only bind mounts). Output isolation (`$out` only writable dir). Closure completeness (`composite_hash(inputs + script + arch)` — any change → new store entry). Provider erasure — once stored, source provider is metadata, not identity.

**Lock file**: Completeness (`forjar pin --check` fails if any input missing). Freshness (stale hashes detected). Atomic update (temp-file + rename).
