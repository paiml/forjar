# Forjar Platform Specification

> Idempotent convergence, full-stack undo, sub-second query, and optimized container builds.

**Status**: Draft | **Date**: 2026-03-05 | **Spec IDs**: FJ-2000 through FJ-2706

---

## Vision

Forjar is a convergence engine for heterogeneous infrastructure. It manages packages, files, services, GPU configs, and container images across SSH hosts, Docker containers, and kernel namespaces — with formally specified idempotency, content-addressed state, and sub-second queryability.

This specification covers five capabilities that close the gap between "good IaC tool" and "world-class platform":

1. **Sub-second SQLite query engine** — FTS5-powered inventory, health, drift, and history queries
2. **Generation model with active undo** — Nix-style snapshots extended with config tracking and cross-machine undo
3. **Formal idempotency guarantees** — Verus-specified reconciliation loop with honest scope boundaries
4. **Multi-machine stack operations** — Setup/teardown/undo across heterogeneous GPU fleets
5. **Optimized container image builds** — Daemonless, content-addressed OCI images from declarative resources

---

## Architecture Overview

```
                    ┌───────────────────────────────────┐
                    │         forjar CLI                  │
                    │  apply | destroy | undo | query     │
                    │  build | generations | drift        │
                    └──────────┬────────────────────────┘
                               │
          ┌────────────────────┼──────────────────────┐
          │                    │                       │
┌─────────▼──────┐  ┌─────────▼──────┐  ┌────────────▼────────┐
│  Convergence   │  │  Query Engine  │  │  Image Builder       │
│  Engine        │  │  (SQLite)      │  │  (OCI)               │
│                │  │                │  │                      │
│ planner/       │  │ state.db       │  │ Path 1: Direct tar   │
│ executor/      │  │ FTS5 + WAL    │  │ Path 2: Pepita→OCI   │
│ codegen/       │  │ ingest pipeline│  │ Path 3: type: image  │
│ resolver/      │  │                │  │                      │
└───────┬────────┘  └───────┬────────┘  └──────────┬──────────┘
        │                   │                      │
┌───────▼───────────────────▼──────────────────────▼──────────┐
│                    State Layer                               │
│                                                              │
│  state/<machine>/state.lock.yaml    (resource convergence)   │
│  state/<machine>/events.jsonl       (provenance log)         │
│  state/<machine>/destroy-log.jsonl  (undo-destroy records)   │
│  state/generations/<N>/             (Nix-style snapshots)    │
│  state/images/<name>/               (OCI image layouts)      │
│  /var/lib/forjar/store/<hash>       (content-addressed)      │
│                                                              │
│  CQRS: flat files = source of truth; state.db = read model   │
└───────┬──────────────────────────────────────────┬──────────┘
        │                                          │
┌───────▼──────────────────────────────────────────▼──────────┐
│                    Transport Layer                            │
│                                                              │
│  pepita  → unshare + cgroups v2 + overlayfs  (~10-50ms)     │
│  container → docker/podman exec              (~500ms-1s)     │
│  local   → bash -euo pipefail                (instant)       │
│  ssh     → ControlMaster persistent          (~100-500ms)    │
│                                                              │
│  All scripts purified through bashrs before dispatch         │
└──────────────────────────────────────────────────────────────┘
```

### Core Principles

1. **CQRS**: YAML/JSONL flat files are the source of truth. SQLite is a derived read model. If `state.db` is deleted, `forjar ingest` rebuilds it from flat files.
2. **Content-addressed**: BLAKE3 for internal store and drift detection. SHA-256 for OCI compatibility. Both digests computed per artifact (BLAKE3 for store addressing, SHA-256 for OCI manifests).
3. **Idempotent by construction**: Observe-diff-act reconciliation with hash comparison. Second apply is always a no-op — zero remote I/O, zero mutations (state files are read and hashes recomputed, but no convergence actions execute).
4. **Transport-agnostic**: Same YAML works across pepita, Docker, local, and SSH. Transport is dispatched at runtime.
5. **Honest boundaries**: Formal properties scoped to what's actually provable. Known limitations documented, not hidden.

---

## Component Specifications

Each component is a self-contained document in the [`platform/`](platform/) subdirectory.

### State and Query

| # | Component | Spec ID | Lines | Description |
|---|-----------|---------|-------|-------------|
| 1 | [SQLite Query Engine](platform/01-sqlite-query-engine.md) | FJ-2001, FJ-2004 | ~480 | Schema, FTS5, ingest pipeline, `forjar query` CLI, performance targets |
| 2 | [Generation Model and Undo](platform/02-generation-undo.md) | FJ-2002, FJ-2003, FJ-2005 | ~480 | Extended generations, active undo algorithm, undo-destroy, multi-machine atomicity |

### Guarantees

| # | Component | Spec ID | Lines | Description |
|---|-----------|---------|-------|-------------|
| 3 | [Idempotency and Drift](platform/03-idempotency-drift.md) | FJ-2006 | ~300 | Verus formal properties, plan-time vs drift-time hash comparison, honest scope |

### Operations

| # | Component | Spec ID | Lines | Description |
|---|-----------|---------|-------|-------------|
| 4 | [Multi-Machine Operations](platform/04-multi-machine-ops.md) | FJ-2000 | ~500 | Transport abstraction, pepita deep-dive, setup/teardown patterns (single/multi/fleet) |

### Container Builds

| # | Component | Spec ID | Lines | Description |
|---|-----------|---------|-------|-------------|
| 5 | [Container Image Builds](platform/05-container-builds.md) | FJ-2101–FJ-2104 | ~500 | Three build paths, OCI assembly, `type: image` resource, layer optimization |
| 6 | [Distribution and Registry](platform/06-distribution.md) | FJ-2105, FJ-2106 | ~300 | `docker load`, registry push, FAR integration, store integration, drift detection |

### Verification

| # | Component | Spec ID | Lines | Description |
|---|-----------|---------|-------|-------------|
| 9 | [Provable Design by Contract](platform/09-provable-design-by-contract.md) | FJ-2200–FJ-2203 | ~500 | Four-tier verification: runtime contracts, Kani real-code harnesses, Verus narrowed proofs, structural enforcement |

### Security and Observability

| # | Component | Spec ID | Lines | Description |
|---|-----------|---------|-------|-------------|
| 10 | [Security Model](platform/10-security-model.md) | FJ-2300 | ~300 | Authorization, path restrictions, secret management, privilege boundaries |
| 11 | [Observability](platform/11-observability.md) | FJ-2301 | ~250 | Structured logging, progress reporting, error output, exit codes, diagnostics |

### Build Pipeline

| # | Component | Spec ID | Lines | Description |
|---|-----------|---------|-------|-------------|
| 12 | [Build Pipeline](platform/12-build-pipeline.md) | FJ-2400–FJ-2403 | ~450 | bashrs purification, apr model compilation, WASM deployment, forjar self-build |

### Configuration

| # | Component | Spec ID | Lines | Description |
|---|-----------|---------|-------|-------------|
| 13 | [Config Validation](platform/13-config-validation.md) | FJ-2500–FJ-2504 | ~450 | Parse, structural, deep validation, unknown field detection, LSP diagnostics |

### Testing

| # | Component | Spec ID | Lines | Description |
|---|-----------|---------|-------|-------------|
| 14 | [Testing Strategy](platform/14-testing-strategy.md) | FJ-2600–FJ-2607 | ~550 | Convergence property testing, idempotency verification, behavior specs, sandbox testing, mutation testing, coverage model |

### Task Framework

| # | Component | Spec ID | Lines | Description |
|---|-----------|---------|-------|-------------|
| 15 | [Task Framework](platform/15-task-framework.md) | FJ-2700–FJ-2706 | ~550 | Task modes (batch/pipeline/service/dispatch), quality gates, GPU targeting, distributed coordination, consumer integration |

### Cross-Cutting

| # | Component | Spec ID | Lines | Description |
|---|-----------|---------|-------|-------------|
| 7 | [Competitive Analysis](platform/07-competitive-analysis.md) | — | ~350 | Combined positioning vs Terraform, Ansible, NixOS, Pulumi, Docker, Buildah, systemd |
| 8 | [Known Limitations](platform/08-known-limitations.md) | — | ~400 | 16 honest limitations, falsification results, compatibility strategy |

---

## What We Have (Production)

| Capability | Implementation | Status |
|-----------|----------------|--------|
| Idempotent apply | Hash-compare + NO-OP planner (`planner/mod.rs`) | Production |
| Destroy | Reverse-DAG teardown, `--yes` gated (`cli/destroy.rs`) | Production |
| Git rollback | Config replay via `HEAD~N` (`cmd_rollback`) | Production |
| Generations | Nix-style numbered snapshots, atomic symlink (`cli/generation.rs`) | Production |
| Named snapshots | Save/restore/delete state checkpoints (`cli/snapshot.rs`) | Production |
| Drift detection | BLAKE3 hashing + `state_query_script` (`tripwire/drift/`) | Production |
| Event log | Append-only JSONL per machine (`tripwire/eventlog.rs`) | Production |
| Reversibility | Classify reversible/irreversible (`planner/reversibility.rs`) | Production |
| Verus proofs | Formal idempotency/convergence/termination (`verus_spec.rs`) | Compile-gated |
| SSH ControlMaster | Persistent connections, O(1) handshakes (`transport/ssh.rs`) | Production |
| Content-addressed store | BLAKE3 store with derivations (`core/store/`) | Production |
| Pepita namespaces | unshare + cgroups v2 + overlayfs (`transport/pepita.rs`) | Production |
| FAR archives | Chunked tar + zstd + Merkle tree (`core/store/far.rs`) | Production |
| bashrs purification | I8 invariant: all scripts validated before exec (`core/purifier.rs`) | Production |
| apr model provider | Pull/cache models via `apr pull` (`resources/model.rs`) | Production |
| Task resources | Batch execution with completion_check + artifacts (`resources/task.rs`) | Production |
| Wave parallelism | DAG-respecting parallel execution (`executor/machine_wave.rs`) | Production |

## What This Spec Adds

| Capability | New Commands | Component |
|-----------|-------------|-----------|
| Sub-second query | `forjar query "bash" --health --drift --timing` | [01](platform/01-sqlite-query-engine.md) |
| Active undo | `forjar undo [--machine X] [--dry-run]` | [02](platform/02-generation-undo.md) |
| Undo destroy | `forjar undo-destroy [--machine X]` | [02](platform/02-generation-undo.md) |
| Generation diff | `forjar diff --generation 3 7` | [02](platform/02-generation-undo.md) |
| Container builds | `forjar build --resource img [--push\|--load\|--far]` | [05](platform/05-container-builds.md) |
| Image resource | `type: image` with `layers:` array | [05](platform/05-container-builds.md) |
| Provable contracts | `forjar contracts --coverage` + 4-tier verification | [09](platform/09-provable-design-by-contract.md) |
| Secret management | `{{ secrets.* }}` with Age encryption (env/file/SOPS planned) | [10](platform/10-security-model.md) |
| Structured output | `--json`, exit codes, `forjar doctor` | [11](platform/11-observability.md) |
| bashrs spec | I8 invariant documented, `forjar lint --bashrs-version` | [12](platform/12-build-pipeline.md) |
| apr compilation | `apr compile` integration, model drift detection | [12](platform/12-build-pipeline.md) |
| WASM deploy | `type: wasm_bundle` resource via presentar | [12](platform/12-build-pipeline.md) |
| Unknown field detection | Typo warnings, "did you mean?" suggestions | [13](platform/13-config-validation.md) |
| Deep validation | `forjar validate --deep` for templates, deps, overlaps | [13](platform/13-config-validation.md) |
| LSP enrichment | Real-time structural validation + autocompletion | [13](platform/13-config-validation.md) |
| Convergence testing | `forjar test convergence` with preservation matrix | [14](platform/14-testing-strategy.md) |
| Behavior specs | `.spec.yaml` with verify commands + soft assertions | [14](platform/14-testing-strategy.md) |
| Infrastructure mutation | `forjar test mutate` with mutation score grading | [14](platform/14-testing-strategy.md) |
| Task modes | `mode: batch\|pipeline\|service\|dispatch` | [15](platform/15-task-framework.md) |
| Quality gates | JSON/regex/threshold gates block downstream tasks | [15](platform/15-task-framework.md) |
| Consumer integration | alimentar, entrenar, apr-cli, batuta reference recipes | [15](platform/15-task-framework.md) |

---

## Implementation Roadmap

**Status**: 38/42 phases IMPLEMENTED (90%), 3 PARTIAL (pepita/Verus blocked), 1 INCOMPLETE (Kani state space). 9,764 tests, 95.03% coverage, zero clippy warnings. G4 contracts: 6/6 REMEDIATED.

Phases are ordered by dependency. Each phase is independently shippable.

| Phase | Spec ID | Component | Depends On | Deliverable |
|-------|---------|-----------|------------|-------------|
| 1 | FJ-2001 | SQLite Foundation | — | `forjar query "bash"` in <100ms |
| 2 | FJ-2002 | Extended Generations | Phase 1 | `forjar diff --generation 3 7` |
| 3 | FJ-2003 | Stack Undo | Phase 2 | `forjar undo --dry-run` across fleet |
| 4 | FJ-2004 | Query Enrichments | Phase 1 | `--health`, `--drift`, `--timing`, `-G` flags |
| 5 | FJ-2005 | Undo-Destroy | Phase 3 | `destroy → undo-destroy` round-trip |
| 6 | FJ-2006 | Verus Proofs | — | Model covers real hash pipeline |
| 7 | FJ-2101 | OCI Assembly | — | `forjar oci pack <dir>` produces valid OCI image |
| 8 | FJ-2102 | Direct Layer Assembly | Phase 7 | Build OCI image from file resources |
| 9 | FJ-2103 | Pepita-to-OCI Export | Phase 7 | Build OCI image with `type: build` layers |
| 10 | FJ-2104 | Image Resource Type | Phases 8, 9 | `forjar build` from declarative YAML |
| 11 | FJ-2105 | Distribution | Phase 10 | `--push` to registry, `--load`, `--far` |
| 12 | FJ-2106 | Build Query/Drift | Phases 1, 10 | `forjar query --type image --drift` |
| 13 | FJ-2200 | Runtime Contracts | — | All critical-path functions have `#[ensures]` contracts |
| 14 | FJ-2201 | Kani Real-Code Harnesses | Phase 13 | `cargo kani` passes on real-code harnesses |
| 15 | FJ-2202 | Verus Narrowed Proofs | Phase 14 | Conditional idempotency proof covers real hash pipeline |
| 16 | FJ-2203 | Structural Enforcement | Phase 14 | Handler invariant enforced via trait + `debug_assert` |
| 17 | FJ-2300 | Security Model | — | `allowed_operators`, `deny_paths`, `{{ secrets.* }}` |
| 18 | FJ-2301 | Observability | — | `--json`, `-v`, exit codes, `forjar doctor` |
| 19 | FJ-2400 | bashrs Purification | — | I8 invariant documented, purification benchmarks |
| 20 | FJ-2401 | apr Model Pipeline | — | Full pull-convert-compile-serve in forjar recipes |
| 21 | FJ-2402 | WASM Deployment | Phase 20 | Presentar apps deployable via `forjar apply` |
| 22 | FJ-2403 | Self-Build Hardening | — | Reproducible builds, binary size tracking |
| 23 | FJ-2500 | Unknown Field Detection | — | Typo warnings with "did you mean?" suggestions |
| 24 | FJ-2501 | Format Validation | Phase 23 | mode, port, path, addr format checks |
| 25 | FJ-2502 | Include Hardening | — | Circular detection, conflict warnings |
| 26 | FJ-2503 | Default Deep Validation | Phases 23, 24 | `forjar validate --deep` runs all checks |
| 27 | FJ-2504 | LSP Enrichment | Phase 23 | Real-time structural validation + autocompletion |
| 28 | FJ-2600 | Convergence Property Testing | — | Preservation matrix, proptest generators |
| 29 | FJ-2601 | Idempotency Verification | Phase 28 | Plan/script/hash idempotency tests |
| 30 | FJ-2602 | Behavior-Driven Infra Specs | — | `.spec.yaml` format, verify commands |
| 31 | FJ-2603 | Sandbox Testing | Phase 30 | Pepita/container sandbox lifecycle |
| 32 | FJ-2604 | Infrastructure Mutation | Phase 31 | Mutation operators per resource type |
| 33 | FJ-2605 | Coverage Model | Phases 28-32 | Five-level resource coverage (L0-L5) |
| 34 | FJ-2606 | Test Runner | Phases 28-33 | `forjar test` unified command |
| 35 | FJ-2607 | CI Integration | Phase 34 | behavior/convergence/mutation workflows |
| 36 | FJ-2700 | Task Modes | — | batch, pipeline, service, dispatch |
| 37 | FJ-2701 | Input/Output Tracking | Phase 36 | BLAKE3 input hashing, cache skip logic |
| 38 | FJ-2702 | Quality Gates | Phase 36 | JSON/regex/threshold gates |
| 39 | FJ-2703 | GPU Device Targeting | Phase 36 | CUDA_VISIBLE_DEVICES injection |
| 40 | FJ-2704 | Distributed Coordination | Phase 36 | gather/scatter/fan-out primitives |
| 41 | FJ-2705 | Consumer Integration | Phases 36-40 | alimentar, entrenar, apr-cli, batuta recipes |
| 42 | FJ-2706 | Task State Model | Phase 36 | Pipeline/service/dispatch state tracking |

---

## Performance Targets

| Operation | Target | Mechanism |
|-----------|--------|-----------|
| `forjar query "bash"` | <50ms | FTS5 + covering indexes |
| `forjar query --health` | <100ms | Indexed GROUP BY |
| `forjar query --drift` | <50ms | Indexed drift_findings |
| `forjar generations` | <20ms | Indexed generation list |
| State ingest (50 resources) | <200ms | Batch INSERT in transaction |
| State ingest (1000 resources) | <2s | Incremental with cursor |
| `forjar build` (cached layers) | <1s | Store hash check, no rebuild |
| `forjar build` (file layers) | <5s | Direct tar construction |
| `forjar build --push` (no new layers) | <2s | HEAD blob check, skip upload |
| `state.db` for 3 machines | <1MB | SQLite is compact |

---

## State Compatibility

New Forjar versions must handle old state files, and old versions must not corrupt new state files.

| Rule | Mechanism |
|------|-----------|
| Missing files are empty | If `destroy-log.jsonl` doesn't exist, `destroy_log` table is empty (not an error) |
| Unknown YAML fields ignored | `#[serde(default)]` on all new fields — old lock files work with new Forjar |
| Schema version in state.db | `PRAGMA user_version` stores schema version; on ingest, run migrations if behind |
| No automatic downgrade | Upgrading is safe (migrations). Downgrading requires `forjar ingest --rebuild` |
| State directory lock | `state/.forjar.lock` (PID-file with liveness check) prevents concurrent modification |

---

## References

- [NSync: Automated Cloud IaC Reconciliation (arXiv 2510.20211)](https://arxiv.org/abs/2510.20211)
- [State Reconciliation Defects in IaC (FSE 2024)](https://2024.esec-fse.org/details/fse-2024-research-papers/50/)
- [Atomic Upgrading of Distributed Systems (Dolstra 2008)](https://edolstra.github.io/pubs/atomic-hotswup2008-submitted.pdf)
- [K8s Reconciliation Loop Pattern](https://oneuptime.com/blog/post/2026-02-09-operator-reconciliation-loop/view)
- [Event Sourcing Pattern (Microsoft)](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
- [SQLite FTS5 Extension](https://sqlite.org/fts5.html)
- [OCI Image Specification v1.1](https://github.com/opencontainers/image-spec)
- [OCI Distribution Specification v1.1](https://github.com/opencontainers/distribution-spec)
- [Nix dockerTools.buildLayeredImage](https://ryantm.github.io/nixpkgs/builders/images/dockertools/)
- [nix2container](https://github.com/nlewo/nix2container)
- [Buildah — daemonless container builds](https://github.com/containers/buildah)
- [ko — fast Go container images](https://ko.build/)
- pmat context.db — 7 tables, 2 FTS5 virtual tables, 15 indexes, sub-second on 5K+ functions
