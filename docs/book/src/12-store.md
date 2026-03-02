# Content-Addressed Store

Forjar's content-addressed store provides reproducible, hermetic package
management inspired by [Nix](https://nixos.org/). Every build output is
placed under a deterministic path derived from its inputs, ensuring that
identical inputs always produce identical store paths.

## Store Model

```
/var/lib/forjar/store/
├── <blake3-hash>/
│   ├── meta.yaml       # Provenance, input hashes, timestamps
│   └── content/        # Build output (files, dirs)
└── .gc-roots/          # Symlinks to live store paths
```

Store entries are **write-once**: once created, they are never modified.
Updates produce new entries with new hashes. The hash is computed from:

- Recipe hash
- Sorted input hashes (closure)
- Target architecture
- Provider identifier

## Provider Import Pipeline

External tools seed the store via the **provider import** bridge
(`src/core/store/provider_exec.rs`):

1. **Validate** import config (reference, arch, provider-specific rules)
2. **Generate** CLI command via `provider::import_command()`
3. **I8 gate** — validate script via bashrs (`purifier::validate_script()`)
4. **Create** staging directory
5. **Execute** via transport layer (`transport::exec_script_timeout()`)
6. **Hash** staging output (BLAKE3 composite of all files)
7. **Atomic move** staging → store (`rename()`)
8. **Write** `meta.yaml` with provenance chain

Supported providers: `apt`, `cargo`, `uv`, `nix`, `docker`, `tofu`,
`terraform`, `apr`.

## Substitution Protocol

Before building, forjar checks if the result already exists:

1. **Compute** closure hash from inputs
2. **Check local store** → `LocalHit` (skip everything)
3. **Check SSH caches** in order → `CacheHit` (pull via rsync)
4. **Cache miss** → build from scratch in sandbox
5. **Store** result locally
6. **Auto-push** to first SSH cache (if configured)

Cache transport uses SSH+rsync — no HTTP, no tokens, no TLS complexity.
See `src/core/store/cache_exec.rs` for the execution bridge.

## Sandbox Lifecycle

Sandboxed builds follow a 10-step lifecycle
(`src/core/store/sandbox_exec.rs`):

| Step | Description |
|------|-------------|
| 1 | Create PID/mount/net namespace (pepita) |
| 2 | Mount overlayfs (lower=inputs, upper=tmpfs) |
| 3 | Bind inputs read-only |
| 4 | Apply cgroup limits (memory, CPU) |
| 5 | Apply seccomp BPF (Full level: deny connect/mount/ptrace) |
| 6 | Execute bashrs-purified build script |
| 7 | Extract outputs from `$out` |
| 8 | Compute BLAKE3 hash of output directory |
| 9 | Atomic move to content-addressed store |
| 10 | Destroy namespace and clean up |

Sandbox levels:

- **Full** — no network, read-only inputs, seccomp BPF, cgroups
- **NetworkOnly** — network allowed, filesystem isolation enforced
- **Minimal** — PID/mount namespaces only
- **None** — legacy behavior (no sandbox)

## Derivation Model

A derivation takes store entries as inputs, applies a transformation
inside a sandbox, and produces a new store entry. Derivations form a
**DAG** (directed acyclic graph) — each node depends on its inputs.

```yaml
derivation:
  inputs:
    base: { store: "blake3:abc123..." }
    config: { resource: "my-config" }
  script: |
    cp -r $inputs/base/* $out/
    patch -p1 < $inputs/config/patch.diff
  sandbox:
    level: full
```

Execution: `src/core/store/derivation_exec.rs` resolves inputs, computes
closure hashes, checks for store hits, and delegates to the sandbox for
cache-miss builds.

## Garbage Collection

GC uses a **mark-and-sweep** algorithm:

1. **Roots** — profile generations, lock file pins, `.gc-roots/` symlinks
2. **Mark** — BFS from roots following `references` in `meta.yaml`
3. **Sweep** — remove unreachable entries

The execution bridge (`src/core/store/gc_exec.rs`) adds:

- Path traversal protection (prevents `../../` attacks)
- Dry-run mode (report what would be deleted)
- GC journal for recovery diagnosis
- Partial failure continuation

## Pin Resolution

Lock file pins are resolved by querying providers
(`src/core/store/pin_resolve.rs`):

| Provider | Command |
|----------|---------|
| apt | `apt-cache policy <name>` |
| cargo | `cargo search <name> --limit 1` |
| nix | `nix eval nixpkgs#<name>.version --raw` |
| uv/pip | `pip index versions <name>` |
| docker | `docker image inspect <name>` |
| apr | `apr info <name> --format version` |

## Key Invariants

- **I8**: All shell scripts validated via bashrs before execution
- **Purity monotonicity**: resource purity ≥ max(dependency purity)
- **Write-once store**: hash = identity, atomic creation via temp+rename
- **Closure determinism**: identical inputs → identical store hashes
- **Provenance tracking**: every entry records origin provider, ref, hash

## References

- [Phase L Execution Spec](../specifications/store/phase-l-execution.md)
- [Store Specification Index](../specifications/store/README.md)
