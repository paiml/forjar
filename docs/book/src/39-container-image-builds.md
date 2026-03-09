# Container Image Builds

Forjar builds OCI-compliant container images without a running daemon. Three independent build paths produce content-addressed, deterministic layers with dual-digest tracking (BLAKE3 for internal store, SHA-256 for OCI registries).

## Three Build Paths

### Path 1: Direct Assembly (Files/Packages → tar)

Build OCI layers directly from resource definitions — no container runtime needed.

```rust
use forjar::core::store::layer_builder::{build_layer, LayerEntry};
use forjar::core::types::OciLayerConfig;

let entries = vec![
    LayerEntry::dir("etc/", 0o755),
    LayerEntry::file("etc/nginx/nginx.conf", config_bytes, 0o644),
    LayerEntry::file("usr/bin/healthcheck", script_bytes, 0o755),
];

let config = OciLayerConfig::default(); // gzip, deterministic, epoch mtime=0
let (result, compressed) = build_layer(&entries, &config).unwrap();

println!("OCI digest:  {}", result.digest);      // sha256:...
println!("Store hash:  {}", result.store_hash);   // blake3:...
println!("Compressed:  {} bytes", result.compressed_size);
```

### Path 2: Container-Based Build (Pepita/Docker → overlay export)

Run build commands inside a container, capture filesystem changes via overlay scan, then assemble into OCI layers.

```yaml
resources:
  app-image:
    type: image
    base: ubuntu:22.04
    layers:
      - type: packages
        names: [nginx, curl, jq]
      - type: build
        command: "make install PREFIX=/usr/local"
      - type: files
        paths: [/etc/app/config.yaml]
```

### Path 3: Image Resource (Declarative YAML)

Compose paths 1 and 2 into a multi-layer build plan:

```rust
use forjar::core::types::{ImageBuildPlan, LayerStrategy};

let plan = ImageBuildPlan {
    tag: "myapp:1.0".into(),
    base_image: Some("ubuntu:22.04".into()),
    layers: vec![
        LayerStrategy::Packages { names: vec!["nginx".into()] },
        LayerStrategy::Files { paths: vec!["/etc/app.conf".into()] },
    ],
    labels: vec![("maintainer".into(), "team@example.com".into())],
    entrypoint: Some(vec!["nginx".into(), "-g".into(), "daemon off;".into()]),
};

// Tier plan groups layers for optimal building
let tiers = plan.tier_plan();
// Tier 0: Packages (sandbox build)
// Tier 1: Build commands (sandbox with overlay)
// Tier 2: File layers (direct tar, no sandbox)
// Tier 3: Store derivations (copy from store)
```

## Determinism

All layer builds are deterministic by default:

- Entries sorted lexicographically (or directory-first)
- Epoch mtime = 0 (Unix epoch)
- uid/gid = 0 (root)
- Consistent tar header format (GNU)

```rust
// Same inputs always produce same digests
let (r1, d1) = build_layer(&entries, &config).unwrap();
let (r2, d2) = build_layer(&entries, &config).unwrap();
assert_eq!(r1.digest, r2.digest);   // Deterministic
assert_eq!(d1, d2);                  // Bit-identical
```

## Dual Digest

Every blob gets both BLAKE3 (internal) and SHA-256 (OCI) digests computed in a single pass:

```rust
use forjar::core::store::layer_builder::compute_dual_digest;

let dd = compute_dual_digest(content);
println!("OCI:    {}", dd.oci_digest());     // sha256:...
println!("Forjar: {}", dd.forjar_digest());  // blake3:...
```

## Compression

Three algorithms supported:

| Algorithm | Use Case | Media Type |
|-----------|----------|------------|
| gzip | Maximum compatibility (default) | `application/vnd.oci.image.layer.v1.tar+gzip` |
| zstd | Better ratio, OCI 1.1+ | `application/vnd.oci.image.layer.v1.tar+zstd` |
| none | Testing/debugging | `application/vnd.oci.image.layer.v1.tar` |

## Distribution

```bash
forjar build --resource app-image --load    # docker/podman load
forjar build --resource app-image --push    # OCI registry push
forjar build --resource app-image --far     # FAR archive export
```

Registry push supports:
- HEAD blob check (skip existing)
- Monolithic PUT for small blobs (<64MB)
- Chunked PATCH+PUT for large blobs (≥64MB, 16MB chunks)

## Overlay Whiteouts

When building from container snapshots, file deletions are represented as OCI whiteouts:

| Overlay Change | OCI Whiteout |
|---------------|--------------|
| File deleted | `.wh.<filename>` |
| Directory opaque | `.wh..wh..opq` |

## Falsification

```bash
cargo run --example platform_security_falsification
```

Key invariants verified:
- Same inputs → identical digests (determinism)
- Different content → different digests (discrimination)
- Compression reduces size for large content
- OCI manifest schema always 2
- Layer count preserved through assembly
- Whiteout paths match OCI spec
