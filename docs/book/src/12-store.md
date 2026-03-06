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

## FAR Archive Format

FAR (Forjar ARchive) is a binary format for distributing store
entries, kernel contracts, and model artifacts. Layout:

```
magic(12) → manifest_len(8) → zstd(manifest_yaml)
          → chunk_count(8) → chunk_table(48*N)
          → zstd(chunks) → sig_len(8) → sig
```

Key properties:

- **Streaming decode** — manifest and chunk table are read without
  loading chunk data, enabling inspection of large archives
- **Zstd compression** — both manifest and chunks are zstd-compressed
- **BLAKE3 per-chunk hashing** — 64KB fixed-size chunks, each with
  its own BLAKE3 hash for verified streaming
- **Binary Merkle tree** — tree root hash for integrity verification
- **Kernel contract metadata** — optional field for model onboarding
  (model type, required ops, coverage percentage)

```bash
# Pack a store entry into a FAR archive
forjar archive pack blake3:abc123 -o output.far

# Inspect a FAR archive (manifest only, no data load)
forjar archive inspect output.far

# Unpack a FAR archive into the store
forjar archive unpack output.far

# Verify archive integrity
forjar archive verify output.far
```

See `cargo run --example store_far_archive` for a complete demo.

## Secret Scanning (Phase I)

All sensitive values in forjar configs must use `ENC[age,...]`
encryption. The secret scanner enforces this with 15 regex patterns:

| Pattern | Detects |
|---------|---------|
| `aws_access_key` | AKIA prefix + 16 alphanumeric |
| `aws_secret_key` | aws_secret_access_key assignments |
| `private_key_pem` | RSA/EC/DSA/OPENSSH PEM headers |
| `github_token` | ghp_/ghs_ + 36+ chars |
| `generic_api_key` | api_key/apikey assignments |
| `generic_secret` | password/secret/token assignments |
| `jwt_token` | eyJ...eyJ JWT format |
| `slack_webhook` | hooks.slack.com/services URLs |
| `gcp_service_key` | "type": "service_account" JSON |
| `stripe_key` | sk_live/sk_test + 20+ chars |
| `database_url_pass` | mysql/postgres/mongodb with password |
| `base64_private` | private.key base64 assignments |
| `hex_secret_32` | 32+ hex char secret/key values |
| `ssh_password` | sshpass -p commands |
| `age_plaintext` | AGE-SECRET-KEY-1 raw values |

Scanning is integrated into config validation (`scan_yaml_str()`) and
can be run on raw text (`scan_text()`). Age-encrypted values
(`ENC[age,...]`) bypass all pattern checks.

See `cargo run --example store_secret_scan` for a complete demo.

## Bash Provability (I8 Invariant)

**Invariant I8**: No raw shell execution — all shell is bashrs-
validated before reaching the transport layer.

Three validation levels:

1. **`validate_script()`** — lint-based, errors only (fast path)
2. **`lint_script()`** — full diagnostics including warnings
3. **`purify_script()`** — parse, purify AST, reformat (strongest)

The recommended entry point is `validate_or_purify()`: validates
first (fast path), falls back to full purification if needed.

I8 enforcement points:

- `provider_exec.rs` — provider import commands
- `sandbox_run.rs` — sandbox build scripts
- `derivation_exec.rs` — derivation scripts
- `gc_exec.rs` — GC sweep commands
- `cache_exec.rs` — cache transport commands
- `sync_exec.rs` — diff/sync commands

See `cargo run --example store_bash_provability` for a complete demo.

## Performance Benchmarks (Phase J)

Store operations are benchmarked with Criterion.rs:

```bash
# Run all store benchmarks
cargo bench --bench store_bench

# Run core benchmarks
cargo bench --bench core_bench
```

| Operation | Module | Description |
|-----------|--------|-------------|
| store_path_hash | `path.rs` | BLAKE3 composite path derivation |
| purity_classify | `purity.rs` | 4-level purity classification |
| closure_hash | `closure.rs` | Transitive dependency closure |
| repro_score | `repro_score.rs` | Reproducibility scoring |
| far_encode | `far.rs` | FAR archive binary encoding |
| far_decode | `far.rs` | FAR manifest streaming decode |
| chunk_bytes | `chunker.rs` | Fixed-size 64KB chunking |
| tree_hash | `chunker.rs` | Binary Merkle tree root |
| secret_scan | `secret_scan.rs` | 15-pattern regex detection |
| bash_validate | `purifier.rs` | bashrs I8 shell validation |

See `cargo run --example store_benchmarks` for a demo of all
benchmarked operations with timing output.

## Execution Layer (Phase L)

All store operations are bridged to actual execution via the transport
layer. Each bridge module validates commands (I8) before execution and
handles rollback on failure.

| Bridge | Module | CLI Command |
|--------|--------|-------------|
| Provider import | `provider_exec.rs` | `forjar store-import <provider> <ref>` |
| GC sweep | `gc_exec.rs` | `forjar store gc [--dry-run]` |
| Pin resolution | `pin_resolve.rs` | `forjar pin [--check]` |
| Cache transport | `cache_exec.rs` | `forjar cache push/pull` |
| Convert apply | `convert_exec.rs` | `forjar convert --reproducible --apply` |
| Store diff/sync | `sync_exec.rs` | `forjar store diff/sync --apply` |
| Sandbox build | `sandbox_run.rs` | (internal — called by derivation executor) |

### Store Import Example

```bash
# Import nginx from apt into the content-addressed store
forjar store-import apt nginx --version 1.24.0

# Import a Docker image
forjar store-import docker alpine:3.18

# List supported providers
forjar store-import --list-providers
```

### GC Example

```bash
# See what would be deleted (dry-run)
forjar store gc --dry-run

# Actually delete unreachable entries
forjar store gc

# Keep last 10 profile generations
forjar store gc --keep-generations 10
```

### Convert Example

```bash
# Analyze conversion opportunities
forjar convert --reproducible -f myconfig.yaml

# Apply conversion (backup + modify YAML + lock file)
forjar convert --reproducible --apply -f myconfig.yaml
```

## OCI Container Builds

Forjar can build OCI-compliant container images from resource definitions.

### Layer Strategy

Resources map to OCI layers via a tiered build plan:

| Tier | Strategy | Description |
|------|----------|-------------|
| 0 | `Packages` | System packages (apt, cargo, pip) |
| 1 | `Build` | Build commands (compile, install) |
| 2 | `Files` | Configuration files |
| 3 | `Derivation` | Store-path references |

### Runtime Layer Builder

`layer_builder.rs` creates actual OCI layer tarballs from in-memory resource definitions:

```rust
use forjar::core::store::layer_builder::{build_layer, LayerEntry};
use forjar::core::types::OciLayerConfig;

let entries = vec![
    LayerEntry::dir("etc/app/", 0o755),
    LayerEntry::file("etc/app/config.yaml", b"port: 8080\n", 0o644),
];
let (result, compressed_data) = build_layer(&entries, &OciLayerConfig::default()).unwrap();
// result.digest     — sha256 of compressed (OCI layer digest)
// result.diff_id    — sha256 of uncompressed (OCI DiffID)
// result.store_hash — blake3 of uncompressed (forjar store address)
```

Key properties:
- **Deterministic**: sorted entries, epoch mtime, uid/gid 0 — same inputs always produce identical digests
- **Order-independent**: lexicographic sort normalizes entry order regardless of input ordering
- **Dual-digest**: BLAKE3 for store addressing + SHA-256 for OCI compatibility
- **Compression**: gzip (default, widest compat), zstd (better ratio), or none

### OCI Layout

`forjar oci-pack` generates a spec-compliant [OCI Image Layout](https://github.com/opencontainers/image-spec/blob/main/image-layout.md):

```
output/
  oci-layout          # {"imageLayoutVersion": "1.0.0"}
  index.json          # OCI Image Index
  blobs/sha256/       # Content-addressed blobs
  manifest.json       # Docker-compat (for docker load)
```

### Image Assembly

`assemble_image()` combines layer building with OCI manifest generation:

```bash
forjar build -f forjar.yaml --resource my-image
```

This resolves the image resource, builds an `ImageBuildPlan`, creates layers
from file resources, and writes a complete OCI layout to `state/images/`:

```bash
cargo run --example image_assembler
# Layer 0: 4 files, 4096 -> 255 bytes (6% compressed)
# Layer 1: 4 files, 28160 -> 183 bytes (1% compressed)
# Manifest layers: 2, Config layers: 2
# Docker compat: tar -cf - -C output . | docker load
```

### Overlay Export

For sandbox-built images, `export_overlay_upper()` converts overlayfs upper directories to OCI layers:

1. Convert overlayfs whiteouts to OCI format (`.wh.` prefix)
2. Create layer tarball from upper directory
3. Compute DiffID (sha256 of uncompressed layer)

### Registry Push (OCI Distribution v1.1)

`forjar build --push` pushes images to OCI-compliant registries:

```bash
forjar build -f app.yaml --resource my-image --push
```

The push protocol follows OCI Distribution Spec v1.1:

1. **HEAD check** — `HEAD /v2/{name}/blobs/{digest}` to skip existing blobs
2. **Blob upload** — `POST /v2/{name}/blobs/uploads/` → `PUT ?digest=`
3. **Manifest push** — `PUT /v2/{name}/manifests/{tag}`

`--check-existing` (enabled by default) skips blobs that already exist
in the registry, making incremental pushes fast.

## Convergence Testing

Convergence verification runs in isolated sandboxes:

```bash
forjar test convergence config.yaml
forjar test convergence config.yaml --pairs    # preservation matrix
forjar test convergence config.yaml --parallel 4
```

The 6-step convergence cycle: generate scripts → apply → verify state →
re-apply → verify idempotency → verify preservation.

## Infrastructure Mutation Testing

Mutate target system state to verify drift detection:

```bash
forjar test mutate config.yaml --sandbox pepita
forjar test mutate config.yaml --mutations 50
```

8 mutation operators: delete file, modify content, change permissions,
stop service, remove package, kill process, unmount filesystem,
corrupt config. Parallel sandbox execution for throughput.

## Key Invariants

- **I8**: All shell scripts validated via bashrs before execution
- **Purity monotonicity**: resource purity ≥ max(dependency purity)
- **Write-once store**: hash = identity, atomic creation via temp+rename
- **Closure determinism**: identical inputs → identical store hashes
- **Provenance tracking**: every entry records origin provider, ref, hash

## SQLite Query Engine (FJ-2001)

Forjar maintains a SQLite database (`state/state.db`) for sub-second queries
across all machines, resources, and events. The database uses WAL mode with
FTS5 full-text search.

```bash
# Search resources by keyword
forjar state-query "bash" --state-dir state

# Health dashboard across all machines
forjar state-query --health --state-dir state

# Drift detection (content_hash != live_hash)
forjar state-query --drift --state-dir state

# Change frequency analysis
forjar state-query --churn --state-dir state

# Timing statistics
forjar state-query "converged" --state-dir state --timing

# Git history fusion (RRF ranking)
forjar state-query "bash" --state-dir state -G

# Output formats: --json, --csv, --sql
forjar state-query "package" --type package --json
```

The database is auto-ingested from state files on first query.

## References

- Phase L Execution Spec: `docs/specifications/store/phase-l-execution.md`
- Store Specification Index: `docs/specifications/store/README.md`
