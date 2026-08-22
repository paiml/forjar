//! MCP registry, server, and schema export for forjar tools.

use pforge_config::{ForgeConfig, ForgeMetadata, OptimizationLevel, ParamSchema, TransportType};
use pforge_runtime::{HandlerRegistry, McpServer};

use super::handlers::*;

// ── Registry + Server ───────────────────────────────────────────────

/// Export MCP tool schemas as a JSON-serializable structure.
///
/// Each tool includes its name, description, and input/output JSON schemas
/// derived from `schemars::JsonSchema`.
pub fn export_schema() -> serde_json::Value {
    let tools: Vec<serde_json::Value> = crate::verb::verbs()
        .iter()
        .map(|v| {
            serde_json::json!({
                "name": v.mcp_name(),
                "description": v.description,
                "input_schema": (v.input_schema)(),
                "output_schema": (v.output_schema)(),
                // FVS: annotations were absent entirely, so an agent could not
                // tell a read-only verb from a mutating one. Derived from
                // `Effects` so it cannot drift from the truth.
                "annotations": { "readOnlyHint": v.effects.read_only() },
            })
        })
        .collect();

    serde_json::json!({
        "schema_version": "1.0",
        "server": "forjar-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "tool_count": tools.len(),
        "tools": tools,
    })
}

/// Register every forjar handler into `reg`.
///
/// ONE list, used by BOTH `build_registry` and `serve`. They previously carried
/// separate copies of the same nine lines, and only `serve` was reachable in
/// production — `build_registry` was called from tests and nowhere else. So
/// `test_fj063_build_registry_has_all_tools` asserted that a registry no user
/// ever touched had all the tools, and adding a tenth tool to it while
/// forgetting `serve` would have shipped green.
///
/// Handler registration is generic over each handler's type, so this cannot be
/// a data-driven loop over the verb table. `register_all_matches_the_verb_table`
/// closes that gap by asserting the two agree.
fn register_all(reg: &mut HandlerRegistry) {
    reg.register("forjar_validate", ValidateHandler);
    reg.register("forjar_plan", PlanHandler);
    reg.register("forjar_drift", DriftHandler);
    reg.register("forjar_lint", LintHandler);
    reg.register("forjar_graph", GraphHandler);
    reg.register("forjar_show", ShowHandler);
    reg.register("forjar_status", StatusHandler);
    reg.register("forjar_trace", TraceHandler);
    reg.register("forjar_anomaly", AnomalyHandler);
}

/// Build the MCP handler registry with all forjar tool handlers.
pub fn build_registry() -> HandlerRegistry {
    let mut registry = HandlerRegistry::new();
    register_all(&mut registry);
    registry
}

/// Build the ForgeConfig for the forjar MCP server.
///
/// Exposed as `pub(super)` for testing from sibling test modules.
#[cfg(test)]
pub(super) fn build_forge_config_for_test() -> ForgeConfig {
    build_forge_config()
}

fn build_forge_config() -> ForgeConfig {
    use pforge_config::{HandlerRef, ToolDef};
    use rustc_hash::FxHashMap;

    let empty_params = ParamSchema {
        fields: FxHashMap::default(),
    };

    ForgeConfig {
        forge: ForgeMetadata {
            name: "forjar-mcp".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            transport: TransportType::Stdio,
            optimization: OptimizationLevel::Release,
        },
        tools: crate::verb::verbs()
            .iter()
            .map(|v| ToolDef::Native {
                name: v.mcp_name(),
                description: v.description.to_string(),
                handler: HandlerRef {
                    // pforge resolves this by name at config level; the runtime
                    // registry above is what actually dispatches.
                    path: format!("handlers::{}", v.name),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(v.timeout_ms),
            })
            .collect(),
        resources: vec![],
        prompts: vec![],
        state: None,
    }
}

/// Start the forjar MCP server (stdio transport).
pub async fn serve() -> Result<(), String> {
    let config = build_forge_config();
    let server = McpServer::new(config);

    // Register forjar handlers into the server's registry — the SAME list
    // build_registry() uses, so the tested set and the served set cannot differ.
    let registry = server.registry();
    {
        let mut reg = registry.write().await;
        register_all(&mut reg);
    }

    server
        .run()
        .await
        .map_err(|e| format!("MCP server error: {e}"))
}
