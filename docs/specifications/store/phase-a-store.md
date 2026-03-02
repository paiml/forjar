# Phase A: Store Model (FJ-1300–FJ-1304)

**Status**: ✅ Complete
**Implementation**: `src/core/store/path.rs`, `src/core/store/meta.rs`, `src/core/store/profile.rs`, `src/core/store/reference.rs`

---

## 1. Vision & Motivation

Forjar treats machines as knowable systems — BLAKE3 hashing, tripwire drift detection, and pepita kernel namespace isolation already make every apply auditable and traceable. What's missing is **content-addressed artifact caching**: `apt install curl` produces different binaries across time, mirrors, and architectures. The machine is knowable, but its inputs are not.

The content-addressed store model (pioneered by Nix, Guix, and others) solves this. Forjar brings that insight into its SSH-first, YAML-first, multi-machine world as a native capability — Nix compatibility is optional, not foundational.

### 1.1 Problem Statement

Today, forjar's package resources call `apt install` or `cargo install` at apply time. The result depends on mirror state, package index freshness, and upstream release timing. Two identical applies on two machines can produce different binaries. This violates the "knowable system" thesis.

### 1.2 Key Insight

The key insight is the **content-addressed store** [1][2]. Hash all inputs, use that hash as the storage key, and you get reproducibility (same inputs → same output), cacheability (build once, substitute everywhere), rollback (previous generations persist), and garbage collection (unreachable entries are safe to delete). This works with any provider — apt, cargo, uv, or optionally nix. Note: content-addressing is necessary but not sufficient — even Nix achieves only 69–91% bitwise reproducibility at scale [3], primarily due to timestamps, build paths, and non-deterministic compilers.

### 1.3 Competitive Position

| Dimension | Nix | Docker | Ansible | Terraform | **Forjar** |
|-----------|-----|--------|---------|-----------|-----------|
| Content-addressed store | SHA256 | Layer hashes | No | No | **BLAKE3** [4] |
| Hermetic builds | Full | Dockerfile | No | No | **4 purity levels** |
| Cache | HTTP | OCI registry | No | No | **SSH-only** |
| Expression | Nix lang | Dockerfile | Jinja2 | HCL | **YAML + bashrs** |
| Multi-machine | NixOps | Swarm/K8s | Inventory | Workspaces | **Native SSH fleet** |
| Import-and-own | No | No | No | No | **Yes (derivations)** |

### 1.4 Expression Layer

YAML templates expanded by Rust, producing bashrs-purified POSIX shell. Recipe expansion (`src/core/recipe/expansion.rs`) resolves `{{inputs.*}}` templates. This is the derivation model — no new language needed.

---

## 2. Store Layout

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

## 3. Path Derivation (FJ-1300)

Store paths reuse `composite_hash()` from `src/tripwire/hasher.rs:86`:

```
store_path = composite_hash([recipe_hash, input_hashes..., arch, provider])
```

- `recipe_hash`: BLAKE3 of recipe YAML after template expansion
- `input_hashes`: BLAKE3 of all transitive inputs (packages, files, source tarballs)
- `arch`: target machine architecture
- `provider`: package provider (`apt`, `cargo`, `uv`)

## 4. Store Metadata with Provenance (FJ-1301)

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
  origin_provider: "nix"
  origin_ref: "nixpkgs#ripgrep@14.1.0"
  origin_hash: "sha256:def456..."
  derived_from: ["blake3:aaa..."]
  derivation_depth: 1
```

The `provenance` block is the traceability chain. `origin_provider` + `origin_ref` record where the artifact entered the store. `derived_from` tracks the derivation DAG. Depth 0 = direct import; depth > 0 = derived. Enables `forjar diff` and `forjar sync`.

## 5. Profile Generations (FJ-1302)

Profile symlinks under `/var/forjar/profiles/`: `system-1 → store/<hash-a>/content`, `system-2 → store/<hash-b>/content`, `current → system-2`. Rollback: `forjar rollback` re-points `current` to the previous generation. Atomic via `rename(2)`.

## 6. YAML Integration (FJ-1303)

`store: true` field on `Resource` (extending `src/core/types/resource.rs`) opts into the store model. Without it, resources behave as today.

## 7. Reference Scanning (FJ-1304)

Store entries track references via `meta.yaml`. The GC follows these to build a reachability graph. References discovered by scanning output files for store path hashes (conservative scanning).

---

## References

- [1] E. Dolstra, "The Purely Functional Software Deployment Model," PhD thesis, Utrecht University, 2006
- [2] L. Courtès, "Functional Package Management with Guix," arXiv:1305.4584, 2013
- [3] J. Malka et al., "Does Functional Package Management Enable Reproducible Builds at Scale? Yes," arXiv:2501.15919, 2025
- [4] J. O'Connor et al., "BLAKE3: one function, fast everywhere," spec paper, 2020
