# OCI Image Assembler

Falsification coverage for FJ-2104 (OCI image assembly from resource definitions).

## Assembly Pipeline

Connects layer builder with OCI types to produce loadable container images:

```rust
use forjar::core::store::image_assembler::assemble_image;
use forjar::core::store::layer_builder::LayerEntry;
use forjar::core::types::{ImageBuildPlan, LayerStrategy, OciLayerConfig};

let entries = vec![vec![LayerEntry::file("app/bin", b"binary", 0o755)]];
let result = assemble_image(&plan, &entries, output_dir, &OciLayerConfig::default(), None)?;
```

### Multi-Layer Concurrent Builds

When >1 layer exists, layers build concurrently via `thread::scope`:

```rust
let plan = ImageBuildPlan {
    tag: "webapp:latest".into(),
    layers: vec![
        LayerStrategy::Packages { names: vec!["nginx".into()] },
        LayerStrategy::Files { paths: vec!["/etc/nginx".into()] },
        LayerStrategy::Build { command: "npm build".into(), workdir: None },
    ],
    ..
};
// 3 layers build in parallel threads
let result = assemble_image(&plan, &entries, out, &config, None)?;
assert_eq!(result.layers.len(), 3);
```

### Architecture Support

```rust
// Default: amd64
let r = assemble_image(&plan, &entries, out, &config, None)?;
assert_eq!(r.config.architecture, "amd64");

// Explicit ARM64
let r = assemble_image(&plan, &entries, out, &config, Some("arm64"))?;
assert_eq!(r.config.architecture, "arm64");
```

## Output Structure

```
output/
├── oci-layout           # {"imageLayoutVersion":"1.0.0"}
├── index.json           # OCI index (schemaVersion: 2)
├── manifest.json        # Docker-compat manifest (for docker load)
└── blobs/sha256/
    ├── <config-hash>    # OCI image config JSON
    ├── <manifest-hash>  # OCI manifest JSON
    └── <layer-hash>...  # Compressed layer tarballs
```

## Rejection Criteria

16 falsification tests in `tests/falsification_image_assembler.rs`:

- Single-layer and multi-layer concurrent assembly
- Layer count mismatch error propagation
- Labels and entrypoint injection into config
- Target architecture (amd64 default, arm64)
- OCI layout file creation and valid JSON structure
- Docker manifest.json with RepoTags and layer paths
- Layer digests (sha256 prefix, non-zero compressed size)
- Build determinism (same input → same digest)
- History entries from layer strategy descriptions
