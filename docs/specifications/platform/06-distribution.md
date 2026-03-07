# 06: Distribution and Registry

> Load, push, FAR export, store integration, and image drift detection.

**Spec IDs**: FJ-2105 (distribution), FJ-2106 (query/drift) | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## CLI

```bash
forjar build -f app.yaml --resource my-app-image          # build to OCI layout
forjar build -f app.yaml --resource my-app-image --load    # build + docker load
forjar build -f app.yaml --resource my-app-image --push    # build + registry push
forjar build -f app.yaml --resource my-app-image --far     # build + FAR archive
forjar build -f app.yaml --resource my-app-image --dry-run # show layers + cache status
forjar build -f app.yaml --resource my-app-image --no-cache
forjar build -f app.yaml                                   # build all image resources
forjar build --platform linux/amd64,linux/arm64            # multi-arch
```

### Output

```
$ forjar build -f training.yaml --resource training-image

Building training-image (myregistry.io/training:2.1.0-cuda12.4.1)
  Base: nvidia/cuda:12.4.1-runtime-ubuntu22.04 (cached, 3 layers)
  Layer 1/3: python-runtime (package)
    → store hit: blake3:a1b2c3... (cached, 0.2s)
  Layer 2/3: ml-deps (build)
    sandbox: pepita network_only, memory=8192MB
    → blake3:d4e5f6... (new, 47.3s)
  Layer 3/3: training-code (files)
    → blake3:g7h8i9... (new, 0.01s)

  Image: sha256:abc123... (1.2GB, 6 layers)
  Store: /var/lib/forjar/store/blake3:j0k1l2...
  Built in 47.8s (47.3s in ml-deps, 0.5s overhead)
```

---

## Local Load

`--load` pipes the OCI tar to `docker load` / `podman load` (auto-detected from PATH):

```
fn load_image(oci_dir):
    let runtime = which_runtime("docker") ? "docker"
                : which_runtime("podman") ? "podman"
                : error("--load requires docker or podman on PATH")
    let tar = create_oci_tar(oci_dir)
    exec_script(local, format!("{runtime} load"), stdin=tar)
```

---

## Registry Push

OCI Distribution Spec v1.1:

```
fn push_image(oci_dir, name, tag):
    let manifest = read_manifest(oci_dir)

    // 1. Push layer blobs (skip existing)
    for layer in manifest.layers:
        let exists = http_head(format!("/v2/{name}/blobs/{}", layer.digest))
        if exists.status == 200:
            continue  // already in registry
        let upload_url = http_post(format!("/v2/{name}/blobs/uploads/"))
        http_put(format!("{upload_url}?digest={}", layer.digest),
                 body=read_blob(oci_dir, layer.digest))

    // 2. Push config blob
    push_blob_if_missing(manifest.config.digest)

    // 3. Push manifest
    http_put(format!("/v2/{name}/manifests/{tag}"),
             content_type="application/vnd.oci.image.manifest.v1+json",
             body=manifest_json)
```

`--check-existing` enables the HEAD blob check (default: always check).

### Layer Size Limits

Registries impose per-layer and per-manifest size limits:

| Registry | Max Layer Size | Max Manifest Size |
|---------|---------------|-------------------|
| Docker Hub | 10GB | 4MB |
| AWS ECR | 10GB | — |
| GCR/Artifact Registry | 10GB | — |
| GitHub GHCR | 10GB | — |
| Self-hosted (Harbor) | Configurable | Configurable |

If a layer exceeds 10GB (common for CUDA + ML dependency layers):
1. **Warn at build time**: "Layer ml-deps is 14.2GB — may exceed registry limits"
2. **`--split-layers <max_size>`**: Split large `type: build` layers by running the build in stages, each producing a sub-layer under the size limit
3. **Chunked upload**: OCI Distribution Spec supports chunked blob upload (PATCH with Content-Range). Use for layers over 1GB regardless of total size — avoids timeout on slow connections.

The spec does NOT attempt automatic layer splitting by default (it changes layer count and invalidates caches). The user must opt in.

---

## FAR Integration

The existing FAR format wraps OCI images for offline distribution:

```bash
forjar build --resource my-image --far
# Produces: state/images/my-image.far (zstd-compressed, BLAKE3 Merkle)

forjar far import my-image.far --load
# Extracts → OCI layout → docker load
```

---

## Store Integration

Built layers are content-addressed in `/var/lib/forjar/store/`:

```
/var/lib/forjar/store/
  <blake3_1>/           # Base image layers (shared across images)
    layer.tar.gz
    meta.yaml           # origin: docker://ubuntu:22.04, oci_digest: sha256:...

  <blake3_2>/           # Package layer
    layer.tar.gz
    meta.yaml           # resources: [curl, jq], oci_digest: sha256:...

  <blake3_3>/           # Build layer
    layer.tar.gz
    meta.yaml           # derivation_hash: ..., sandbox: network_only

  <blake3_4>/           # Application layer
    layer.tar.gz
    meta.yaml           # resources: [train.py, config.yaml]
```

Cache invalidation: layer store hash = BLAKE3(resource definitions + base digests). If inputs unchanged → store hit → skip rebuild.

---

## Base Image Handling

```
fn resolve_base_image(base):
    // 1. Check store
    let digest = pin_resolve::resolve("docker", base)?
    if store::exists(digest): return store::load_layers(digest)

    // 2. Pull + extract + store
    let oci_tar = exec_script(local, format!("docker pull {} && docker save {}", base, base))
    let layers = oci::extract_layers(oci_tar)
    for layer in &layers:
        store::put(layer.store_hash, layer.compressed)
    store::put_meta(digest, BaseMeta { layers, arch, os })
    layers
```

---

## Image Drift Detection

Extend tripwire to deployed images:

```
fn check_image_drift(resource_id, machine, expected_digest):
    let actual = exec_script(machine,
        "docker inspect {container} --format '{{.Image}}'")
    if actual != expected_digest:
        DriftFinding {
            resource_type: Image,
            expected_hash: expected_digest,
            actual_hash: actual,
            detail: "deployed image differs from built image",
        }
```

## Query Integration

Images flow into the SQLite query model:

```bash
forjar query --type image                          # list built images
forjar query "training" --type image --history     # build history
forjar query --type image --timing                 # build duration stats
forjar query --type image --drift                  # stale deployments
```

---

## Implementation

### Phase 11: Distribution (FJ-2105) -- IMPLEMENTED
- [x] Distribution target types: `DistTarget` (Load/Push/Far) (type definition)
- [x] Push result types: `PushResult`, `PushKind` (Layer/Config/Manifest/Index) (type definition)
- [x] Multi-arch types: `ArchBuild` with linux/amd64 and linux/arm64 constructors (type definition)
- [x] Build report: `BuildReport`, `LayerReport`, `DistResult` with `format_summary()` (type + formatting)
- [x] `--load`: pipe OCI tar to `docker load` / `podman load` (code path exists in `registry_push.rs`)
- [x] `--push`: OCI Distribution v1.1 (HEAD check + blob upload + manifest PUT) — `cmd_build_push()` in `build_image.rs` calls `push_image()` from `registry_push.rs`; graceful fallback when registry unreachable
- [x] `--far`: wrap OCI image in FAR archive (builds on existing FAR infrastructure)
- [x] `--check-existing`: blob existence check via `check_blob_exists()` HEAD request
- [x] Multi-arch: build matrix → OCI Image Index (type-level)
- **Deliverable**: `forjar build --push` assembles image → discovers blobs → pushes to registry via OCI Distribution v1.1
- **Note**: End-to-end push wired (2026-03-07): `cmd_build_push()` → `discover_blobs()` → `push_image()` → `push_blob()` per blob. Graceful fallback when registry is unreachable.

### Phase 12: Build Query/Drift (FJ-2106) -- IMPLEMENTED
- [x] Image resources in SQLite `resources` table (schema supports `resource_type = 'image'`)
- [x] `forjar query --type image` (query filter exists)
- [x] Image drift detection (type-level — `check_image_drift` logic defined)
- [x] Build timing in `--timing` (uses existing `duration_secs` enrichment)
- **Deliverable**: `forjar query --type image --drift` — works once images are built and ingested

> **Convention note**: `[x]` means "type or CLI wiring exists in code." End-to-end validation depends on the container build pipeline (Phases 8-10) being completed. See FALSIFICATION-REPORT.md § E8.
