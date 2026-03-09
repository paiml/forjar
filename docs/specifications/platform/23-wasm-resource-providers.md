# WASM Resource Provider Plugin System

> User-authored resource types compiled to WebAssembly, sandboxed and content-addressed.

**Status**: Proposed | **Date**: 2026-03-09 | **Spec IDs**: FJ-3400 through FJ-3409

---

## Motivation

Forjar supports 12 built-in resource types (file, package, service, mount, user, docker, cron, network, gpu, model, task, build). Users needing custom resource types (Kubernetes CRDs, cloud-specific resources, internal tooling) must fork or wait for upstream. Terraform's provider ecosystem is its strongest competitive advantage. Forjar needs a plugin system — but sovereign: WASM-sandboxed, not Docker-based, not RPC-based.

### Chain of Thought: Sovereign Stack Implementation

```
Problem: No user-extensible resource types. Only 12 built-in.

STEP 1 — WASM Runtime (wasmtime, already in ecosystem)
  wasmtime is the WASM runtime used by trueno for compute kernels.
  Reuse the same runtime for resource provider plugins.
  WASI preview 2 provides file I/O, env vars, clock access.
  Capability-based security: plugins get ONLY the permissions declared in manifest.

STEP 2 — Plugin ABI (forjar-plugin-abi crate)
  Define a stable ABI for resource providers:
    fn check(state: &[u8]) -> PluginResult<Status>
    fn apply(state: &[u8]) -> PluginResult<ApplyOutcome>
    fn destroy(state: &[u8]) -> PluginResult<()>
    fn state_query(state: &[u8]) -> PluginResult<StateMap>
  State is serialized as MessagePack (compact, schema-free, fast).
  ABI versioned: v1 must remain stable. Breaking changes → v2.

STEP 3 — Plugin Lifecycle (batuta agent model)
  batuta's agent lifecycle (init → ready → running → shutdown) maps to plugins.
  Plugin loaded on first use, cached in memory for subsequent calls.
  Hot-reload: BLAKE3 hash of .wasm file checked before each invocation.
  If hash changed, reload plugin (allows development workflow).

STEP 4 — Content-Addressed Registry (forjar store)
  Plugins stored in /var/lib/forjar/plugins/<blake3-hash>.wasm
  Manifest: plugins/manifest.yaml with name, version, hash, permissions.
  `forjar plugin install` downloads and verifies BLAKE3.
  Local plugins: `forjar plugin build --path ./my-provider`

STEP 5 — Script Bridge (bashrs)
  For simple providers, allow "shell provider" mode:
  Plugin is a shell script with check/apply/destroy functions.
  bashrs validates the script. Executed in pepita namespace for isolation.
  Shell providers are the on-ramp; WASM providers are the destination.

STEP 6 — Quality (pmat + certeza)
  pmat grades plugin source code (TDG, complexity).
  certeza validates plugin test coverage.
  Plugin quality score displayed in `forjar plugin list`.

Conclusion: WASM sandbox provides security isolation without Docker overhead.
wasmtime is already in the stack. Plugin ABI is stable and versioned.
Content-addressed with BLAKE3. Shell bridge for easy adoption.
```

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│              forjar apply                          │
│  encounters type: "plugin:k8s-deployment"          │
└──────────┬──────────────────────────────────────┘
           │
┌──────────▼──────────────────────────────────────┐
│         Plugin Resolver                           │
│                                                   │
│  1. Lookup in plugin manifest                     │
│  2. Verify BLAKE3 hash                            │
│  3. Load WASM module (cached or fresh)            │
│  4. Create sandboxed instance with permissions    │
└──────────┬──────────────────────────────────────┘
           │
┌──────────▼──────────────────────────────────────┐
│         WASM Sandbox (wasmtime)                   │
│                                                   │
│  Capabilities granted per manifest:               │
│  - fs: read/write to specific paths               │
│  - net: connect to specific hosts                 │
│  - env: read specific env vars                    │
│  - exec: run specific binaries                    │
│                                                   │
│  Plugin ABI v1:                                   │
│    check(state) → Status                          │
│    apply(state) → ApplyOutcome                    │
│    destroy(state) → ()                            │
│    state_query(state) → StateMap                  │
└─────────────────────────────────────────────────┘
```

### Plugin Manifest

```yaml
# plugins/k8s-deployment/plugin.yaml
name: k8s-deployment
version: "0.1.0"
description: "Manage Kubernetes Deployments via kubectl"
abi_version: 1
wasm: k8s-deployment.wasm
blake3: "a7f3c2e1..."

permissions:
  fs:
    read: ["~/.kube/config"]
  net:
    connect: ["kubernetes.default.svc:443"]
  exec:
    allow: ["kubectl"]
  env:
    read: ["KUBECONFIG", "KUBE_CONTEXT"]

# Resource schema (validated by forjar)
schema:
  required: [name, namespace, image]
  properties:
    name: { type: string }
    namespace: { type: string, default: "default" }
    image: { type: string }
    replicas: { type: integer, default: 1 }
    ports: { type: array, items: { type: integer } }
```

### Usage in forjar.yaml

```yaml
resources:
  my-app:
    type: "plugin:k8s-deployment"
    machine: k8s-control-plane
    name: my-app
    namespace: production
    image: "registry.internal/my-app:v2.1"
    replicas: 3
    ports: [8080, 9090]
    depends_on: [k8s-namespace]
```

---

## Spec IDs

| ID | Deliverable | Depends On |
|----|-------------|-----------|
| FJ-3400 | Plugin ABI v1 crate (forjar-plugin-abi) | — |
| FJ-3401 | wasmtime host integration with capability-based sandbox | FJ-3400 |
| FJ-3402 | Plugin manifest format and BLAKE3 verification | FJ-3400 |
| FJ-3403 | `forjar plugin install/list/remove` CLI | FJ-3402 |
| FJ-3404 | Plugin resolver: type "plugin:NAME" dispatch | FJ-3401 |
| FJ-3405 | Shell provider bridge (bashrs-validated scripts) | FJ-3400 |
| FJ-3406 | Plugin hot-reload via BLAKE3 hash check | FJ-3401 |
| FJ-3407 | `forjar plugin build --path ./provider` (cargo + wasm-pack) | FJ-3400 |
| FJ-3408 | Plugin schema validation (JSON Schema subset) | FJ-3402 |
| FJ-3409 | Integration test: custom plugin check/apply/destroy cycle | FJ-3404 |

---

## Performance Targets

| Operation | Target | Mechanism |
|-----------|--------|-----------|
| Plugin load (cold) | < 50ms | wasmtime AOT compilation cached |
| Plugin load (warm) | < 1ms | In-memory module cache |
| Plugin check() call | < 10ms overhead | WASM → host boundary, excluding plugin logic |
| Plugin apply() call | < 10ms overhead | WASM → host boundary, excluding plugin logic |
| BLAKE3 verification | < 1ms | Single-pass hash of .wasm file |

---

## Batuta Oracle Advice

**Recommendation**: batuta for plugin lifecycle management (init/ready/shutdown).
**Compute**: Scalar — WASM execution is CPU-bound.
**Supporting**: depyler for transpiling plugin implementations to optimized WASM.

## arXiv References

- [WebAssembly Runtimes Survey (2404.12621)](https://arxiv.org/abs/2404.12621) — 98-paper survey covering security, performance, plugin architectures
- [Cyber-physical WebAssembly: Pluggable Drivers (2410.22919)](https://arxiv.org/abs/2410.22919) — WASI interfaces for hardware with security mediation
- [WAMI: Compilation to WASM via MLIR (2506.16048)](https://arxiv.org/abs/2506.16048) — Preserving abstractions during WASM compilation
- [Hidden Technical Debt in ML Systems (1503.02531)](https://arxiv.org/abs/1503.02531) — Plugin boundaries reduce system-level entanglement

---

## Falsification Criteria

| ID | Claim | Rejection Test |
|----|-------|---------------|
| F-3400-1 | WASM sandbox isolates filesystem | Plugin tries to read /etc/shadow without fs permission; REJECT if read succeeds |
| F-3400-2 | WASM sandbox isolates network | Plugin tries to connect to unauthorized host; REJECT if connection succeeds |
| F-3400-3 | Plugin ABI is stable | Compile plugin against ABI v1, upgrade forjar; REJECT if plugin fails to load |
| F-3400-4 | BLAKE3 prevents tampered plugins | Modify one byte in .wasm; REJECT if plugin loads without hash error |
| F-3400-5 | Cold load < 50ms | Benchmark first load of 1MB .wasm file; REJECT if p95 > 50ms |
| F-3400-6 | Shell bridge validates scripts | Inject command injection in shell provider; REJECT if bashrs doesn't catch it |
| F-3400-7 | Hot-reload detects changes | Modify .wasm during apply cycle; REJECT if old version used after change |
| F-3400-8 | No non-sovereign WASM runtime | Audit Cargo.toml; REJECT if runtime other than wasmtime used |
