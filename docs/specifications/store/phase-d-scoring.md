# Phase D: Build Sandboxing & Repro Scoring (FJ-1315–FJ-1319)

**Status**: ✅ Complete
**Implementation**: `src/core/store/sandbox.rs`, `src/core/store/repro_score.rs`

---

## 1. Build Sandboxing (FJ-1315–FJ-1319)

Extends pepita kernel namespace isolation (`src/transport/pepita.rs`, `src/resources/pepita/mod.rs`). Existing: PID/mount/net namespaces, cgroups v2, overlayfs. Store sandbox adds: read-only bind mounts for inputs, minimal `/dev`, seccomp BPF [7] (`connect`/`mount`/`ptrace` denied), tmpfs `/tmp`. Caveat: seccomp usability is a known challenge — developers arrive at different filter sets for the same application [8]. Forjar mitigates this by providing preset profiles (`level: full`, `level: network-only`) rather than requiring raw BPF authoring.

### Config (FJ-1315)

`sandbox: { level: full, memory_mb: 2048, cpus: 4.0, timeout: 600 }` on any resource with `store: true`.

### Lifecycle (FJ-1316)

Create namespace → overlay mount (lower=inputs, upper=tmpfs) → bind inputs read-only → cgroup limits → bashrs-purified build → extract outputs → `hash_directory()` → store → destroy namespace. All steps reuse existing pepita functions.

### Preset Profiles

| Profile | Level | Memory | CPUs | Timeout | Notes |
|---------|-------|--------|------|---------|-------|
| `full` | Full | 2 GB | 4 | 600s | No network, full isolation |
| `network-only` | NetworkOnly | 4 GB | 8 | 1200s | Network allowed, FS isolated |
| `minimal` | Minimal | 1 GB | 2 | 300s | PID/mount namespaces only |
| `gpu` | NetworkOnly | 16 GB | 8 | 3600s | GPU passthrough via bind mount |

## 2. Reproducibility Score (FJ-1329)

`forjar validate --check-reproducibility-score` outputs a 0-100 score based on:

| Component | Weight | Scoring |
|-----------|--------|---------|
| Purity level | 50% | Pure=100, Pinned=75, Constrained=25, Impure=0 |
| Store coverage | 30% | Percentage of resources with `store: true` |
| Lock coverage | 20% | Percentage of resources with lock file pin |

Grade thresholds: A ≥ 90, B ≥ 75, C ≥ 50, D ≥ 25, F < 25.

---

## References

- [7] J. Jia et al., "Programmable System Call Security with eBPF," arXiv:2302.10366, 2023
- [8] M. Alhindi, J. Hallett, "Playing in the Sandbox: A Study on the Usability of Seccomp," arXiv:2506.10234, 2025
