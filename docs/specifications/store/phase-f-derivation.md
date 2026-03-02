# Phase F: Derivations (FJ-1330–FJ-1344)

**Status**: 🔧 Partial — types ✅ / build execution 🔲
**Implementation**: `src/core/store/derivation.rs`, `src/core/store/derivation_exec.rs`, `src/core/store/provider.rs`

---

## 1. Recipe Conversion Strategy (FJ-1328–FJ-1332)

### Five-Step Conversion Ladder

| Step | Action | Purity Result | Automated |
|------|--------|---------------|-----------|
| 1 | Add `version:` pins to all packages | Constrained → Pinned | Yes |
| 2 | Add `store: true` to cacheable resources | Enables store | Yes |
| 3 | Generate `forjar.inputs.lock.yaml` | Pins all inputs | Yes |
| 4 | Add `sandbox:` blocks | Pinned → Pure | Manual |
| 5 | Replace imperative hooks with declarative resources | Full purity | Manual |

### Automated Conversion (FJ-1328)

`forjar convert --reproducible` automates steps 1-3: adds version pins, enables `store: true`, generates `forjar.inputs.lock.yaml`. Reports remaining impure resources (curl|bash patterns) that require manual intervention.

## 2. Universal Provider Import (FJ-1333–FJ-1340)

Any external tool can seed the forjar store. Each provider shells out to its native CLI, captures outputs, BLAKE3-hashes them, and stores the result. After import, all store entries are identical — provider-agnostic, distributable as FAR over SSH. Zero new crate dependencies for any provider.

### Supported Providers (FJ-1333–FJ-1336)

| Provider | CLI | Capture Method | Example |
|----------|-----|----------------|---------|
| `apt` | `apt install` | Package files via dpkg manifest | `packages: [nginx]` |
| `cargo` | `cargo install` | Binary output in `$CARGO_HOME/bin/` | `packages: [ripgrep]` |
| `uv` | `uv pip install` | Virtualenv contents | `packages: [flask]` |
| `nix` | `nix build --print-out-paths` | Output tree in `/nix/store/` | `packages: ["nixpkgs#ripgrep"]` |
| `docker` | `docker export` | Filesystem snapshot from container | `image: "ubuntu:24.04"` |
| `tofu` | `tofu output -json` | State outputs (IPs, IDs, configs) | `source: "infra/"` |
| `terraform` | `terraform output -json` | State outputs | `source: "infra/"` |

## 3. Store Derivations (FJ-1341–FJ-1345)

Take one or more store entries as inputs, apply a transformation inside a pepita sandbox, produce a new store entry. This is how imported artifacts become forjar-native — the **import once, own forever** model.

### Derivation Model (FJ-1341)

```yaml
resources:
  ml-rootfs:
    type: derivation
    store: true
    sandbox: { level: full, memory_mb: 4096, cpus: 8.0, timeout: 1800 }
    inputs:
      base_rootfs: { store: "blake3:aaa..." }
      cuda_toolkit: { store: "blake3:bbb..." }
      config: { resource: "nginx-config" }
    script: |
      cp -r {{inputs.base_rootfs}}/* $out/
      cp -r {{inputs.cuda_toolkit}}/* $out/usr/local/
```

### Derivation Lifecycle (FJ-1342)

Resolve inputs → compute closure hash → check store (hit = substitute, skip build) → create pepita namespace → bind inputs read-only → execute bashrs script (writes `$out`) → `hash_directory($out)` → atomic move to store → write `meta.yaml` (closure, provenance) → destroy namespace.

### Any Provider → Pepita Pipeline (FJ-1343)

Provider is interchangeable: Docker/Nix/apt → store → derivation → pepita. The derivation is the universal adapter.

### Derivation Chains (FJ-1344)

Derivations reference other derivations as inputs, forming a DAG (evaluated bottom-up via `depends_on`). Each step produces a new immutable store entry, independently cacheable and substitutable.

### Import Once, Own Forever

Import from any provider (nix, docker, tofu, terraform, apt), derive your customization on top, pack as FAR, distribute over SSH. The source provider is never invoked again. The `meta.yaml` provenance chain records the origin for traceability, but the artifact's identity is its own BLAKE3 hash.

### Upstream Diff and Sync (FJ-1345)

```bash
forjar store diff <hash>               # diff store entry against upstream origin
forjar store sync <hash> --apply       # re-import upstream, replay derivation chain
```

## 4. Remaining Work

| Gap | Status | Description |
|-----|--------|-------------|
| Provider invocation | 🔲 | Shell out to apt/cargo/nix/docker CLIs |
| Derivation builder | 🔲 | validate_dag → sequential build execution |
| Store diff/sync | 🔲 | Actual file comparison + rsync |
