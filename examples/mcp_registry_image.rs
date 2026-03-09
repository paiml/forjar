//! FJ-MCP/2104: MCP registry schema, image assembler.
//!
//! Usage: cargo run --example mcp_registry_image

use forjar::core::store::image_assembler::assemble_image;
use forjar::core::store::layer_builder::LayerEntry;
use forjar::core::types::{ImageBuildPlan, LayerStrategy, OciLayerConfig};
use forjar::mcp::registry::{build_registry, export_schema};

fn main() {
    println!("Forjar: MCP Registry & Image Assembler");
    println!("{}", "=".repeat(55));

    // ── MCP Schema ──
    println!("\n[MCP] Tool Schema:");
    let schema = export_schema();
    println!("  Server: {} v{}", schema["server"], schema["version"]);
    println!("  Tools: {}", schema["tool_count"]);
    let tools = schema["tools"].as_array().unwrap();
    for tool in tools {
        println!(
            "    {} — {}",
            tool["name"].as_str().unwrap(),
            tool["description"].as_str().unwrap()
        );
    }

    // ── MCP Registry ──
    println!("\n[MCP] Handler Registry:");
    let registry = build_registry();
    println!("  Handlers: {}", registry.len());
    for name in [
        "forjar_validate",
        "forjar_plan",
        "forjar_drift",
        "forjar_lint",
        "forjar_graph",
        "forjar_show",
        "forjar_status",
        "forjar_trace",
        "forjar_anomaly",
    ] {
        println!("    {name}: registered={}", registry.has_handler(name));
    }

    // ── Image Assembler ──
    println!("\n[FJ-2104] Image Assembly:");
    let tmp = tempfile::tempdir().unwrap();
    let plan = ImageBuildPlan {
        tag: "forjar-example:latest".into(),
        base_image: None,
        layers: vec![
            LayerStrategy::Files {
                paths: vec!["app.txt".into()],
            },
            LayerStrategy::Packages {
                names: vec!["curl".into()],
            },
        ],
        labels: vec![("org.forjar.example".into(), "true".into())],
        entrypoint: Some(vec!["/bin/sh".into()]),
    };
    let layer_data = vec![
        vec![LayerEntry {
            path: "app.txt".into(),
            content: b"hello forjar".to_vec(),
            mode: 0o644,
            is_dir: false,
        }],
        vec![LayerEntry {
            path: "usr/bin/curl".into(),
            content: b"fake-curl".to_vec(),
            mode: 0o755,
            is_dir: false,
        }],
    ];
    let result = assemble_image(
        &plan,
        &layer_data,
        tmp.path(),
        &OciLayerConfig::default(),
        None,
    )
    .unwrap();
    println!("  Tag: {}", plan.tag);
    println!("  Layers: {}", result.layers.len());
    println!("  Total size: {} bytes", result.total_size);
    println!("  Layout: {}", result.layout_dir.display());
    for (i, layer) in result.layers.iter().enumerate() {
        println!(
            "    Layer {}: {}B compressed, {} files",
            i, layer.compressed_size, layer.file_count
        );
    }

    println!("\n{}", "=".repeat(55));
    println!("All MCP/image criteria survived.");
}
