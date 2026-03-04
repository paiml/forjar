//! Example: FJ-063 — forjar MCP server via pforge
//!
//! Demonstrates building and introspecting the forjar MCP tool registry.
//! In production, `forjar mcp` starts the full stdio MCP server.
//!
//! ```bash
//! cargo run --example mcp_server
//! ```

use forjar::mcp;

fn main() {
    println!("=== FJ-063: forjar MCP Server (pforge) ===\n");

    // Build the handler registry
    let registry = mcp::build_registry();
    println!("Registered {} MCP tools:", registry.len());

    let tools = [
        ("forjar_validate", "Validate forjar.yaml configuration"),
        ("forjar_plan", "Show execution plan for changes"),
        ("forjar_drift", "Detect configuration drift"),
        ("forjar_lint", "Lint config for best practices"),
        ("forjar_graph", "Generate dependency graph"),
        ("forjar_show", "Show resolved config"),
        ("forjar_status", "Show state from lock files"),
    ];

    for (name, desc) in &tools {
        let registered = if registry.has_handler(name) {
            "[OK]"
        } else {
            "[MISSING]"
        };
        println!("  {registered} {name} — {desc}");
    }

    // Verify schema introspection
    println!("\nSchema introspection:");
    for (name, _) in &tools {
        if let Some(schema) = registry.get_input_schema(name) {
            let json = serde_json::to_string(&schema).unwrap_or_default();
            println!("  {} → {} bytes", name, json.len());
        }
    }

    // Verify no extra handlers
    assert!(!registry.has_handler("nonexistent"));
    assert!(!registry.is_empty());

    println!("\n=== MCP integration complete ===");
    println!("Start server: forjar mcp");
}
