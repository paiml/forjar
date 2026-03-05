# 07: Competitive Analysis

> Combined positioning across IaC platforms and container builders.

**Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## IaC Platform Comparison

| Capability | Terraform | Ansible | NixOS | Pulumi | **Forjar** |
|-----------|-----------|---------|-------|--------|------------|
| Idempotent apply | Yes (provider-specific) | Partial | Yes (by construction) | Yes | **Yes (hash-based planner)** |
| State history | S3 versioning | None | Generations | Cloud backend | **SQLite generations** |
| Stack undo | Re-apply old .tfstate | None | `nix-env --rollback` | Cloud restore | **`forjar undo` (active diff)** |
| Stack destroy | `terraform destroy` | Manual playbook | `nix-collect-garbage` | `pulumi destroy` | **`forjar destroy`** |
| Undo destroy | Re-apply | N/A | Profile switch | Cloud restore | **`forjar undo-destroy` (best-effort)** |
| Inventory query | `terraform show`, TF Cloud | `ansible-inventory`, AWX | `nix-env --list` (<1s) | `pulumi stack`, Cloud | **`forjar query` (local FTS5)** |
| Query latency | <1s local, seconds API | Seconds (SSH scan) | <1s (local nix store) | Seconds (API) | **<50ms (local SQLite)** |
| Multi-machine atomic | N/A (cloud-per-resource) | Best-effort | NixOps (deployment model) | N/A (cloud) | **Phased + resume (no 2PC)** |
| Drift detection | `terraform plan` (API refresh) | None built-in | Immutable — drift prevented | `pulumi refresh` | **Tripwire (BLAKE3, offline)** |
| Formal guarantees | Provider contracts | None | Nix store purity proofs | None | **Verus specs (model-level)** |
| Event sourcing | Cloud audit log | None | None | Activity log | **Append-only JSONL + SQLite** |

---

## Container Builder Comparison

| Capability | Docker Build | Buildah | Nix dockerTools | Kaniko | ko | **Forjar** |
|-----------|-------------|---------|-----------------|--------|-----|------------|
| Daemon required | Yes | No | No | No | No | **No** |
| Root required | Yes | Optional | No | No | No | **Optional (pepita path)** |
| Dockerfile required | Yes | Optional | No | Yes | No | **No** |
| Layer caching | Layer-by-layer | Manual | Store-based | Partial | N/A | **Content-addressed store** |
| Reproducible | No (timestamps) | No | Yes (epoch mtime) | No | Partial | **Yes (epoch mtime, sorted tar)** |
| GPU builds | `--gpus` (BuildKit) | Manual | No | No | No | **Pepita: host GPU direct** |
| Semantic layers | No (per-RUN) | No | Popularity algorithm | No | App-aware | **Resource-type-aware** |
| Drift detection | None | None | Nix store hash | None | None | **BLAKE3 tripwire** |
| Cross-arch | BuildKit QEMU | QEMU | Cross-compile | No | Go cross-compile | **Pepita + QEMU** |
| Registry push | Built-in | Buildah push | skopeo | Built-in | Built-in | **OCI Distribution Spec** |
| Offline distribution | docker save | skopeo copy | Nix store copy | N/A | N/A | **FAR archive (zstd + BLAKE3)** |

---

## Key Differentiators

### IaC Platform

1. **Local sub-second queryable state**: Runs entirely offline against local SQLite. Terraform Cloud and Pulumi Cloud offer similar search but require network roundtrips and a paid service. NixOS achieves comparable local speed via the Nix store. Forjar's advantage is FTS5 full-text search across resources, events, and drift history in a single local DB.

2. **Verus-specified idempotency (model-level)**: The reconciliation loop properties (termination, convergence, idempotency) are formally specified in Verus — verified against a simplified model. This is design-time confidence, not full implementation verification. NixOS achieves idempotency through a fundamentally stronger approach: immutable packages prevent drift by construction.

3. **Undo-destroy (best-effort)**: Record pre-destroy state in `destroy-log.jsonl`, replay to restore. Reliable for files with inline `content:`; best-effort for packages (versions float), services (runtime state), and source files (external paths may change). Tasks/Users/Networks are irreversible and skipped. Terraform can't undo a destroy without external backup; Pulumi requires cloud-hosted state recovery.

4. **BLAKE3 tripwire (offline)**: Content-addressed drift detection runs entirely offline — no API calls. Terraform requires provider API refresh; Pulumi requires `pulumi refresh`. Forjar's tripwire works on air-gapped systems. Extended with SQLite-indexed drift history and resolution tracking.

5. **Active undo vs passive rollback**: Most tools (Terraform, Ansible) can only re-apply old config. Forjar's undo computes the minimal diff and handles both additions and removals — resources added since the target generation get destroyed, removed resources get re-created. Limited to reversible resource types.

6. **Generation model + event sourcing**: Existing Nix-style generations extended with git-ref config tracking, cross-machine coordination, and SQLite-queryable history. Full auditability with time-travel queries.

### Container Builds

7. **Semantic layer optimization**: Forjar knows that packages change rarely and config files change often — layer ordering is automatic, not manual `COPY --from` gymnastics.

8. **Three build paths, one interface**: Direct assembly for speed, pepita for full resource support, declarative YAML for simplicity. Same `forjar build` command.

9. **GPU-native builds**: Pepita sandbox shares host GPU — CUDA/ROCm compilation works without `--gpus` flags or NVIDIA container toolkit. No other daemonless builder supports this.

10. **Content-addressed everything**: BLAKE3 store for cache, SHA-256 for OCI compat, dual-digest in single pass. Layer cache survives across images that share resources.

11. **Integrated lifecycle**: Build, deploy, drift detect, query, undo — all in one tool. Other builders are build-only; deployment is a separate concern.

---

## systemd Comparison (Single-Machine Convergence)

For machine-level convergence on a single host, the most directly comparable built-in tool is systemd's declarative configuration layer:

| Capability | systemd-* | **Forjar** |
|-----------|-----------|------------|
| File management | `systemd-tmpfiles` (declarative) | `type: file` resource |
| User management | `systemd-sysusers` (declarative) | `type: user` resource |
| Network config | `systemd-networkd` (declarative) | `type: network` resource |
| Service management | systemd units (declarative) | `type: service` resource |
| Dependencies | systemd `After=`, `Requires=` | `depends_on:` DAG |
| Drift detection | None — re-applied on boot | BLAKE3 tripwire (on-demand) |
| State history | journald logs | Generations + SQLite |
| Undo | None | `forjar undo` |
| Cross-machine | None (single-host only) | SSH + pepita + container |
| GPU config | None | `type: gpu` resource |
| Container builds | None | `type: image` resource |

**Key difference**: systemd's declarative tools are present on every modern Linux system with zero additional dependencies. Forjar requires installation but offers cross-machine coordination, undo, drift detection, and the full resource DAG that systemd's per-subsystem tools lack.

systemd is the right choice for boot-time OS configuration managed by distro packages. Forjar is the right choice for application-level convergence across heterogeneous fleets with auditability requirements.

---

## What Others Do Better

Honest assessment of where competitors have the advantage.

| Area | Who | Why |
|------|-----|-----|
| Immutability | NixOS | Prevents drift by construction — fundamentally stronger than detecting drift after the fact |
| Ecosystem reach | Terraform | 3,000+ providers, massive community, established workflows |
| Agentless ad-hoc | Ansible | SSH push model works without any pre-installed agent or state directory |
| Cloud-native state | Pulumi | Team collaboration with hosted state, RBAC, audit trail out of the box |
| Dockerfile compat | Docker Build | Universal — every CI system, every tutorial, every developer knows Dockerfiles |
| Multi-tenant isolation | Firecracker | VM boundary (KVM) vs namespace boundary (pepita) — fundamentally different security level |
| Cross-compile builds | ko | Go's cross-compile is simpler than QEMU emulation for non-Go workloads |
| Zero-install machine config | systemd | Built into every modern Linux — no agent, no state directory, no installation |
