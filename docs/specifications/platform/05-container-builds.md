# 05: Container Image Builds

> Daemonless, content-addressed OCI image construction from Forjar resources.

**Spec IDs**: FJ-2101–FJ-2104 | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## Core Insight: Resources Are Layers

Forjar's resource model provides semantic understanding that Dockerfiles lack. The builder knows what changes together (DAG), what changes rarely (packages) vs often (configs), and what shares across images. Layer ordering is automatic.

## Three Build Paths

| Path | Model | When | Root? | GPU? |
|------|-------|------|:---:|:---:|
| 1. Direct Assembly | ko/Nix — tar files directly | File/package resources | No | N/A |
| 2. Pepita-to-OCI | buildah — sandbox + export upper | Build scripts, pip install | Yes | Yes |
| 3. Image Resource | Declarative YAML composing 1 & 2 | User-facing interface | Depends | Yes |

---

## Path 1: Direct Layer Assembly (ko/Nix Model)

No filesystem ops, no mounts, no namespaces. Pure data transformation.

```
Resource: type=file, path=/etc/app/config.yaml, content=...
  → Layer: tar containing /etc/app/config.yaml with declared content

Resource: type=package, packages=[curl, jq]
  → Query: dpkg -L curl jq (list installed files)
  → Layer: tar of those file paths

Resource: type=file, path=/usr/local/bin/app, source=./build/app
  → Layer: tar containing /usr/local/bin/app from source
```

### Layer Assignment Algorithm

Nix-inspired popularity sort adapted for Forjar's DAG:

```
fn assign_layers(resources, max_layers) -> Vec<Layer>:
    groups = []
    // Group 1: packages (rarely change, shared across images)
    groups.push(ResourceGroup::Packages(filter(Package)))
    // Group 2: runtime (services, mounts — moderate change)
    groups.push(ResourceGroup::Runtime(filter(Service | Mount | Cron)))
    // Group 3+: individual files (change most — one per layer up to limit)
    for file_resource in filter(File):
        if groups.len() < max_layers - 1:
            groups.push(ResourceGroup::Single(file_resource))
        else:
            groups.last_mut().merge(file_resource)  // overflow catch-all
    // Build layers bottom-up: least-changing first
    groups.into_iter().map(build_layer).collect()
```

### Layer Construction

```
fn build_layer(group) -> Layer:
    let mut tar = TarBuilder::new()
    tar.set_mtime(1)   // 1970-01-01T00:00:01Z for reproducibility
    // ... add files based on resource type ...
    tar.sort_entries()  // deterministic ordering

    let uncompressed = tar.finish()
    let diff_id = sha256(uncompressed)          // OCI DiffID
    let compressed = gzip(uncompressed)
    let digest = sha256(compressed)             // OCI layer digest
    let store_hash = blake3(uncompressed)       // Forjar store address
    Layer { digest, diff_id, store_hash, compressed, size: compressed.len() }
```

**Advantages**: Zero deps, reproducible, fast, cross-platform (works on macOS).
**Limitation**: No RUN commands — only declarative resources. Package file listing requires a reference system.

---

## Path 2: Pepita-to-OCI Export (Buildah Model)

For resources needing execution (tasks, scripts, builds). Pepita sandbox → converge → tar overlay upper.

### Extending the Derivation Lifecycle

The existing 10-step derivation lifecycle gets a new step 7.5:

```
Steps 1-7: [existing] resolve inputs → compute hash → check store →
           create namespace → mount overlay → execute script → hash output

Step 7.5 (NEW): Export overlay upper → OCI layer
  tar -cf layer.tar -C /upper .
  Convert overlayfs whiteouts to OCI whiteouts
  Compute sha256 (DiffID + digest), blake3 (store)

Steps 8-10: [existing] atomic move to store → write meta.yaml → destroy namespace
```

### Whiteout Conversion

```
fn export_overlay_to_layer(overlay) -> Layer:
    let mut tar = TarBuilder::new()
    tar.set_mtime(1)
    for entry in walk_dir(overlay.upper_dir):
        if is_overlay_whiteout(entry):
            // overlayfs: char device 0/0
            // OCI: .wh.<original_name>
            tar.add_whiteout(parent.join(format!(".wh.{}", name)))
        elif is_overlay_opaque(entry):
            // overlayfs: xattr trusted.overlay.opaque=y
            // OCI: .wh..wh..opq
            tar.add_file(entry.join(".wh..wh..opq"), b"", "0644", "root")
        else:
            tar.add_path_from_disk(entry)
    tar.sort_entries()
    // ... digest computation same as Path 1
```

### Multi-Layer from DAG Tiers

```
Tier 0: apt-get install curl jq tree      → Layer 0 (pepita sandbox)
Tier 1: systemd units, cron entries        → Layer 1 (pepita sandbox)
Tier 2: config.yaml, scripts               → Layer 2 (direct tar)

Each tier: start namespace with previous tier's output as lower,
converge resources, export upper as new layer.
```

**Advantages**: Full resource support (tasks, scripts, compiled outputs), GPU builds via pepita.
**Limitation**: Linux-only, root/user-namespace required, non-deterministic by default.

### Determinism Levels

`build.deterministic` is not a boolean — it's a spectrum. The flag controls which sources of non-determinism are eliminated:

| Level | Flag Value | Network | Timestamps | Env Vars | CPU Features | Filesystem Order |
|-------|-----------|---------|------------|----------|-------------|-----------------|
| `false` (default) | No restrictions | Allowed | Real | Host env | Host-detected | OS-dependent |
| `network` | Network disabled | Blocked | Real | Host env | Host-detected | OS-dependent |
| `strict` | Full lockdown | Blocked | Epoch | Sanitized | Fixed | Sorted |
| `true` | Alias for `strict` | Blocked | Epoch | Sanitized | Fixed | Sorted |

`strict` mode:
- `SOURCE_DATE_EPOCH=1` — all timestamps are epoch
- `HOME=/nonexistent`, `USER=nobody` — no user-dependent paths
- `LANG=C`, `LC_ALL=C` — deterministic locale
- `TZ=UTC` — deterministic timezone
- CPU feature override: `RUSTFLAGS="-C target-cpu=x86-64"` (baseline, no AVX/SSE4 auto-detect)
- Directory entries sorted before hashing (override `readdir` non-determinism)
- Verification: build twice, compare BLAKE3 of output. Mismatch → error with diff of changed files.

**What `strict` cannot fix**: Compiler version differences, kernel ABI differences, non-deterministic allocators in third-party code. These require a pinned build environment (base image + pepita namespace), not just flag settings.

---

## Path 3: Declarative Image Resource Type

New `type: image` composing paths 1 and 2.

```yaml
resources:
  my-app-image:
    type: image
    name: myregistry.io/myapp
    tag: "{{params.version}}"
    base: ubuntu:22.04

    layers:
      - name: system-packages
        type: package
        provider: apt
        packages: [curl, jq, tree, nginx]

      - name: runtime-config
        type: files
        files:
          - path: /etc/nginx/nginx.conf
            content: |
              worker_processes auto;
              events { worker_connections 1024; }
          - path: /etc/app/config.yaml
            source: ./config/production.yaml

      - name: application
        type: build
        script: |
          cd /src && cargo build --release
          cp target/release/myapp $out/usr/local/bin/myapp
        inputs:
          src: ./src

    entrypoint: ["/usr/local/bin/myapp"]
    cmd: ["--config", "/etc/app/config.yaml"]
    env: { RUST_LOG: info }
    expose: ["8080/tcp"]
    user: nobody
    workdir: /app
    labels:
      org.opencontainers.image.source: "https://github.com/myorg/myapp"

    build:
      deterministic: false    # see Determinism Levels below
      cache: true
      max_layers: 10
      compress: gzip          # gzip (compat) or zstd (OCI 1.1)
```

### Layer Type Dispatch

| `type` | Build Path | Mechanism |
|--------|-----------|-----------|
| `package` | Path 1 | Package file list → tar |
| `files` | Path 1 | Declared files → tar (zero I/O for inline) |
| `build` | Path 2 | Pepita sandbox → export upper |
| `copy` | Path 1 | Copy from host/other image |
| `derivation` | Path 2 | Full store derivation → export $out |

---

## OCI Image Assembly

Common output path for all three build paths.

### OCI Layout

```
state/images/<resource_id>/
  oci-layout          {"imageLayoutVersion":"1.0.0"}
  index.json          Image index → manifest descriptor
  manifest.json       Docker compat (for docker load)
  blobs/sha256/
    <manifest_digest>
    <config_digest>
    <layer_0_digest>  ... <layer_N_digest>
```

### Dual Digest Strategy

| Digest | Purpose | Where |
|--------|---------|-------|
| BLAKE3 | Store address, drift, cache | `/var/lib/forjar/store/<blake3>` |
| SHA-256 (uncompressed) | OCI DiffID | Image config `rootfs.diff_ids[]` |
| SHA-256 (compressed) | OCI layer digest | Manifest + `blobs/sha256/` |

### Build Idempotency

```
image_hash = blake3(base_digest, layer_0_hash, ..., layer_N_hash, config_hash)
if store.exists(image_hash): return store.get(image_hash)  // <1ms, no rebuild
```

---

## GPU Training Image Example

```yaml
resources:
  training-image:
    type: image
    name: myregistry.io/training
    tag: "2.1.0-cuda12.4.1"
    base: "nvidia/cuda:12.4.1-runtime-ubuntu22.04"

    layers:
      - name: python-runtime
        type: package
        packages: [python3, python3-pip]

      - name: ml-deps
        type: build
        script: |
          python3 -m pip install --target $out/usr/local/lib/python3.11/dist-packages \
            torch==2.3.0 transformers==4.40.0
        sandbox: { level: network_only, memory_mb: 8192 }

      - name: training-code
        type: files
        files:
          - { path: /app/train.py, source: ./src/train.py }
          - { path: /app/run.sh, content: "#!/bin/bash\nexec python3 /app/train.py \"$@\"", mode: "0755" }

    entrypoint: ["/app/run.sh"]
    env: { CUDA_VISIBLE_DEVICES: "0" }
```

**Produced layers**: Base CUDA (shared) → Python (shared) → ML deps (rebuild on version change) → App code (tiny, rebuild on every push).

---

## Implementation

### Phase 7: OCI Assembly (FJ-2101)
- [x] `sha2` + `flate2` crates
- [x] OCI types: `OciManifest`, `OciDescriptor`, `OciImageConfig`, `OciRuntimeConfig`, `OciRootfs`
- [x] OCI index type: `OciIndex` with single-manifest constructor
- [x] OCI history entry: `OciHistoryEntry` for build provenance
- [x] Layer build result: `LayerBuildResult` with dual digest (BLAKE3 + SHA-256)
- [x] Image build config: `ImageBuildConfig` with determinism levels
- [x] Layer compression enum: `Gzip`, `Zstd`, `None`
- [x] Determinism level enum: `False`, `Network`, `Strict`, `True`
- [x] OCI layout writer, Docker compat `manifest.json`
- [x] `forjar oci pack <dir> --tag name:tag`

### Phase 8: Direct Layer Assembly (FJ-2102) — PARTIAL
- [x] `OciLayerConfig` — compression, deterministic, epoch mtime, sort order
- [x] Deterministic tar: `TarSortOrder` enum (Lexicographic, DirectoryFirst)
- [x] File → layer, Package → layer (`LayerStrategy` enum defined)
- [x] Layer caching: `LayerCacheEntry` with content_hash, oci_digest, store path
- [x] Dual digest: `DualDigest` BLAKE3 + SHA-256 with `oci_digest()`, `forjar_digest()`

### Phase 9: Pepita-to-OCI (FJ-2103) — PARTIAL
- [x] `export_overlay_upper()` in sandbox_exec.rs
- [x] Overlay-to-OCI whiteout conversion: `WhiteoutEntry` (FileDelete, OpaqueDir)
- [x] Multi-tier layer stacking

### Phase 10: Image Resource Type (FJ-2104) — PARTIAL
- [x] `ResourceType::Image`
- [x] Layer dispatch: `LayerStrategy` enum (Packages, Files, Build, Derivation)
- [x] Base image resolution: `BaseImageRef` with registry(), platform, resolved
- [x] `forjar build` CLI command
