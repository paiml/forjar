# 04: Multi-Machine Operations

> Transport abstraction, pepita deep-dive, and setup/teardown patterns across heterogeneous GPU fleets.

**Spec ID**: FJ-2000 | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## Transport Abstraction

The same resource YAML works identically across all targets. Transport dispatched at runtime (`transport/mod.rs:exec_script`):

```
fn exec_script(machine, script):
    match machine:
        pepita    → pepita::exec()      # unshare + overlay
        container → container::exec()   # docker exec / podman exec
        local     → local::exec()       # bash -euo pipefail
        _         → ssh::exec()         # SSH ControlMaster
```

| Machine Type | Transport | GPU Access | Cold Start |
|-------------|-----------|------------|------------|
| Bare metal SSH (intel, lambda) | `ssh.rs` ControlMaster | Host GPU direct | ~100-500ms |
| Docker on remote (jetson) | `container.rs` via `ssh.rs` | `--gpus all` / `--device /dev/kfd` | ~500ms-1s |
| Docker local (CI) | `container.rs` | `--gpus all` / `--device /dev/dri` | ~500ms-1s |
| Pepita namespace | `pepita.rs` | Host GPU direct | ~10-50ms |

All scripts purified through `bashrs` before dispatch.

---

## Pepita: Kernel-Native Isolation Without Docker

Forjar's zero-dependency isolation layer (`transport/pepita.rs`, `resources/pepita/mod.rs`).

```
unshare --fork --pid --mount [--net] --mount-proc -- sleep infinity
         │          │       │      │         │           │
         │          │       │      │         │           └─ PID 1 init
         │          │       │      │         └─ /proc remounted
         │          │       │      └─ network namespace (optional)
         │          │       └─ mount namespace (filesystem isolation)
         │          └─ PID namespace (process isolation)
         └─ fork into new namespace
```

Script execution: `nsenter --target {pid} --mount --pid [--net] -- bash`, piping purified script to stdin.

### Kernel Primitives

| Primitive | Implementation | Purpose |
|-----------|---------------|---------|
| PID namespace | `unshare --pid` | Process isolation; PID 1 = `sleep infinity` |
| Mount namespace | `unshare --mount` | Filesystem isolation; overlayfs CoW |
| Network namespace | `unshare --net` (optional) | Network isolation |
| Cgroups v2 | `/sys/fs/cgroup/forjar-{name}/` | `memory.max`, `cpu.max`, `cpuset.cpus` |
| Overlayfs | `mount -t overlay` | Read-only lower + writable upper |
| nsenter | `nsenter --target {pid}` | Enter namespace for script exec |

**Not used**: UTS, IPC, user namespace (fields defined, not implemented), chroot, seccomp BPF (flag only), AppArmor/SELinux.

### Pepita vs Docker vs Firecracker

| Capability | Docker | Pepita | Firecracker |
|-----------|--------|--------|-------------|
| Memory limit | cgroups v1 | cgroups v2 | QEMU config |
| Filesystem isolation | overlayfs + layers | overlayfs (explicit) | ext4 rootfs |
| Network isolation | veth + iptables (auto) | netns (manual) | TAP devices |
| GPU passthrough | `--gpus all`, `--device` | **Host direct** | Not designed for GPU |
| Cold start | ~500ms-1s | **~10-50ms** | ~100ms |
| RAM overhead | ~50-100MB | **~5-10MB** | ~10-20MB |
| Dependencies | Docker daemon (~100MB) | **Shell + coreutils (0)** | Firecracker binary (~20MB) |
| Multi-tenant isolation | Moderate (shared kernel) | Moderate (shared kernel) | **Strong (VM boundary)** |

### GPU — The Key Insight

Docker requires explicit passthrough (`--gpus all` + nvidia-container-toolkit). Pepita namespaces share the host device tree. `/dev/nvidia0`, `/dev/kfd` are simply there.

```
Docker: forjar apply → SSH → docker create --gpus all → docker exec → bash
        (~500ms-1s, ~50-100MB, requires nvidia-container-toolkit)

Pepita: forjar apply → SSH → nsenter --pid --mount → bash
        (~10-50ms, ~5-10MB, zero extra dependencies)
```

### When Pepita Replaces Docker

| Scenario | Use Pepita | Use Docker |
|----------|-----------|------------|
| GPU compute on bare metal | Yes — 10x faster cold start | Overkill |
| Reproducible builds (forjar store) | Yes — overlayfs CoW | Slower |
| Short-lived apply runs | Yes — ephemeral by design | Wasteful |
| SSH server without Docker | Yes — kernel 3.8+ | Can't |
| CI without `dockerd` | Yes — kernel primitives | Can't |
| Edge devices (Jetson) | Yes — minimal RAM | Tight on 8GB |

### When Docker Is Still Needed

| Scenario | Why Docker |
|----------|-----------|
| OCI image distribution | Registry pull |
| Multi-container networking | Automatic veth/bridge/iptables |
| Long-lived daemons | Restart policies, health checks |
| NVIDIA container runtime | CUDA toolkit isolation |
| Cross-architecture images | `docker buildx --platform` |

### Could Pepita Act Like Firecracker?

No. Fundamentally different isolation levels:

```
  Process    Namespace    Container    microVM       VM
  (none)     (pepita)     (Docker)     (Firecracker) (QEMU)
  ─────────────────────────────────────────────────────────►
  Speed                                              Isolation
```

Pepita: shared kernel, namespace boundary. Firecracker: separate kernel, VM boundary (KVM). A kernel exploit in pepita compromises the host; in Firecracker it's contained.

---

## Setup/Teardown Patterns

### Pattern 1: Single Node Container

One machine, ephemeral container, GPU passthrough.

```yaml
machines:
  jetson:
    hostname: jetson
    addr: 192.168.50.101
    arch: aarch64
    transport: container
    container:
      image: nvcr.io/nvidia/l4t-pytorch:r36.2.0-pth2.1
      ephemeral: true
      gpus: all

resources:
  workspace:
    type: file
    machine: jetson
    state: directory
    path: /workspace/loadtest

  bench-script:
    type: file
    machine: jetson
    depends_on: [workspace]
    path: /workspace/loadtest/run.sh
    content: |
      #!/bin/bash
      set -euo pipefail
      python3 benchmark.py --batch-sizes 1,4,16,64 --duration 300
    mode: '0755'
```

Setup: SSH → ensure_container() → DAG converge → state lock + event log.
Teardown: reverse DAG → rm scripts via container transport → container removed.

### Pattern 2: Multi-Container Same Host (CUDA + ROCm)

Two containers, different GPU stacks, parallel execution.

```yaml
machines:
  cuda-node:
    addr: container
    container: { image: nvidia/cuda:12.4.1-runtime-ubuntu22.04, gpus: all }
  rocm-node:
    addr: container
    container: { image: rocm/dev-ubuntu-22.04:6.1, devices: [/dev/kfd, /dev/dri] }

policy:
  parallel_machines: true
  failure: continue_independent
```

Both containers created concurrently. Separate state locks. Teardown: `--machine rocm-node` for single-vendor.

### Pattern 3: Multi-Host Fleet

Three hosts, cross-machine dependencies, heterogeneous architectures.

```
Setup DAG:
  Tier 0: test-data (intel)
  Tier 1: test-config (intel)
  Tier 2: jetson-workspace + lambda-workspace  (PARALLEL)
  Tier 3: jetson-bench + lambda-bench          (PARALLEL)

Teardown (reverse):
  Tier 0: jetson-bench + lambda-bench          (PARALLEL)
  Tier 1: jetson-workspace + lambda-workspace  (PARALLEL)
  Tier 2: test-config (intel)
  Tier 3: test-data (intel)
```

Per-machine teardown: `forjar destroy --yes --machine lambda` — only lambda resources, others untouched.

### Lifecycle Summary

```
                    forjar apply                     forjar destroy --yes
                    ────────────                     ────────────────────
Single node:        create container                 reverse DAG teardown
                    DAG-ordered converge             rm resources
                    write state.lock + events        rm container (ephemeral)

Multi-container     create N containers (parallel)   reverse DAG (parallel/tier)
same host:          DAG per machine (parallel)       rm resources per machine
                    N state locks, N event logs      rm containers

Multi-host          SSH ControlMaster per host       reverse cross-machine DAG
fleet:              cross-machine DAG tiers          tier 0: leaf tasks
                    per-machine state isolation      per-machine --machine filter
```

All patterns share the same code path: `executor/mod.rs` → `transport/mod.rs` → `codegen/`. The only difference is transport dispatch and `parallel_machines`. Cross-machine deps resolved by the same DAG resolver.
