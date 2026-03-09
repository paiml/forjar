# MCP Registry & Image Assembler

Falsification coverage for MCP tools registry and FJ-2104 image assembly.

## MCP Tool Registry

Schema export and handler registration for the forjar MCP server:

```rust
use forjar::mcp::registry::{export_schema, build_registry};

// Export tool schemas (JSON)
let schema = export_schema();
assert_eq!(schema["tool_count"], 9);
assert_eq!(schema["server"], "forjar-mcp");

// Build handler registry
let registry = build_registry();
assert_eq!(registry.len(), 9);
assert!(registry.has_handler("forjar_validate"));
```

### Registered Tools

| Tool | Description |
|------|-------------|
| forjar_validate | Validate forjar.yaml configuration |
| forjar_plan | Show execution plan for changes |
| forjar_drift | Detect configuration drift |
| forjar_lint | Lint config for best practices |
| forjar_graph | Generate dependency graph |
| forjar_show | Show resolved config |
| forjar_status | Show current state |
| forjar_trace | View trace provenance |
| forjar_anomaly | Detect anomalous behavior |

## Image Assembler (FJ-2104)

Builds complete OCI images from resource definitions:

```rust
use forjar::core::store::image_assembler::assemble_image;
use forjar::core::store::layer_builder::LayerEntry;
use forjar::core::types::{ImageBuildPlan, LayerStrategy, OciLayerConfig};

let plan = ImageBuildPlan {
    tag: "myapp:latest".into(),
    base_image: None,
    layers: vec![LayerStrategy::Files { paths: vec!["app".into()] }],
    labels: vec![("org.forjar".into(), "true".into())],
    entrypoint: Some(vec!["/bin/sh".into()]),
};

let entries = vec![vec![LayerEntry {
    path: "app".into(), content: b"binary".to_vec(),
    mode: 0o755, is_dir: false,
}]];

let result = assemble_image(&plan, &entries, output_dir,
    &OciLayerConfig::default(), Some("arm64"))?;

assert!(result.layout_dir.join("oci-layout").exists());
assert!(result.layout_dir.join("index.json").exists());
```

### Output Structure

```text
output_dir/
  oci-layout          # {"imageLayoutVersion": "1.0.0"}
  index.json          # OCI Image Index
  manifest.json       # Docker-compat manifest (for docker load)
  blobs/sha256/       # Content-addressed blobs (config, manifest, layers)
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_mcp_registry_image.rs` | 14 | ~246 |
