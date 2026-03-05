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

`--load` pipes the OCI tar to `docker load` / `podman load` (detected from `container.runtime`):

```
fn load_image(oci_dir, machine):
    let runtime = machine.container.as_ref().map(|c| &c.runtime).unwrap_or("docker")
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

### Phase 11: Distribution (FJ-2105)
- [x] Distribution target types: `DistTarget` (Load/Push/Far)
- [x] Push result types: `PushResult`, `PushKind` (Layer/Config/Manifest/Index)
- [x] Multi-arch types: `ArchBuild` with linux/amd64 and linux/arm64 constructors
- [x] Build report: `BuildReport`, `LayerReport`, `DistResult` with `format_summary()`
- [ ] `--load`: pipe OCI tar to `docker load` / `podman load`
- [ ] `--push`: OCI Distribution v1.1 (HEAD check + blob upload + manifest PUT)
- [ ] `--far`: wrap OCI image in FAR archive
- [ ] `--check-existing`: blob existence check
- [ ] Multi-arch: build matrix → OCI Image Index
- **Deliverable**: `forjar build --push` to registry

### Phase 12: Build Query/Drift (FJ-2106)
- [ ] Image resources in SQLite `resources` table
- [ ] `forjar query --type image`
- [ ] Image drift detection
- [ ] Build timing in `--timing`
- **Deliverable**: `forjar query --type image --drift`
