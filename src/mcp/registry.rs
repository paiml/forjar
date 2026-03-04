//! MCP registry, server, and schema export for forjar tools.

use pforge_config::{ForgeConfig, ForgeMetadata, OptimizationLevel, ParamSchema, TransportType};
use pforge_runtime::{HandlerRegistry, McpServer};

use super::handlers::*;
use super::types::*;

// ── Registry + Server ───────────────────────────────────────────────

/// Export MCP tool schemas as a JSON-serializable structure.
///
/// Each tool includes its name, description, and input/output JSON schemas
/// derived from `schemars::JsonSchema`.
pub fn export_schema() -> serde_json::Value {
    use schemars::schema_for;

    let tools = vec![
        serde_json::json!({
            "name": "forjar_validate",
            "description": "Validate a forjar.yaml configuration file",
            "input_schema": schema_for!(ValidateInput),
            "output_schema": schema_for!(ValidateOutput),
        }),
        serde_json::json!({
            "name": "forjar_plan",
            "description": "Show execution plan for infrastructure changes",
            "input_schema": schema_for!(PlanInput),
            "output_schema": schema_for!(PlanOutput),
        }),
        serde_json::json!({
            "name": "forjar_drift",
            "description": "Detect configuration drift from desired state",
            "input_schema": schema_for!(DriftInput),
            "output_schema": schema_for!(DriftOutput),
        }),
        serde_json::json!({
            "name": "forjar_lint",
            "description": "Lint forjar config for best practices and shell safety",
            "input_schema": schema_for!(LintInput),
            "output_schema": schema_for!(LintOutput),
        }),
        serde_json::json!({
            "name": "forjar_graph",
            "description": "Generate resource dependency graph (Mermaid/DOT)",
            "input_schema": schema_for!(GraphInput),
            "output_schema": schema_for!(GraphOutput),
        }),
        serde_json::json!({
            "name": "forjar_show",
            "description": "Show fully resolved config with templates expanded",
            "input_schema": schema_for!(ShowInput),
            "output_schema": schema_for!(ShowOutput),
        }),
        serde_json::json!({
            "name": "forjar_status",
            "description": "Show current state from lock files",
            "input_schema": schema_for!(StatusInput),
            "output_schema": schema_for!(StatusOutput),
        }),
        serde_json::json!({
            "name": "forjar_trace",
            "description": "View trace provenance data from apply runs",
            "input_schema": schema_for!(TraceInput),
            "output_schema": schema_for!(TraceOutput),
        }),
        serde_json::json!({
            "name": "forjar_anomaly",
            "description": "Detect anomalous resource behavior using ML-inspired analysis",
            "input_schema": schema_for!(AnomalyInput),
            "output_schema": schema_for!(AnomalyOutput),
        }),
    ];

    serde_json::json!({
        "schema_version": "1.0",
        "server": "forjar-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "tool_count": tools.len(),
        "tools": tools,
    })
}

pub fn build_registry() -> HandlerRegistry {
    let mut registry = HandlerRegistry::new();
    registry.register("forjar_validate", ValidateHandler);
    registry.register("forjar_plan", PlanHandler);
    registry.register("forjar_drift", DriftHandler);
    registry.register("forjar_lint", LintHandler);
    registry.register("forjar_graph", GraphHandler);
    registry.register("forjar_show", ShowHandler);
    registry.register("forjar_status", StatusHandler);
    registry.register("forjar_trace", TraceHandler);
    registry.register("forjar_anomaly", AnomalyHandler);
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
        tools: vec![
            ToolDef::Native {
                name: "forjar_validate".to_string(),
                description: "Validate a forjar.yaml configuration file".to_string(),
                handler: HandlerRef {
                    path: "handlers::validate".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(30000),
            },
            ToolDef::Native {
                name: "forjar_plan".to_string(),
                description: "Show execution plan for infrastructure changes".to_string(),
                handler: HandlerRef {
                    path: "handlers::plan".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(60000),
            },
            ToolDef::Native {
                name: "forjar_drift".to_string(),
                description: "Detect configuration drift from desired state".to_string(),
                handler: HandlerRef {
                    path: "handlers::drift".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(60000),
            },
            ToolDef::Native {
                name: "forjar_lint".to_string(),
                description: "Lint forjar config for best practices and shell safety".to_string(),
                handler: HandlerRef {
                    path: "handlers::lint".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(30000),
            },
            ToolDef::Native {
                name: "forjar_graph".to_string(),
                description: "Generate resource dependency graph (Mermaid/DOT)".to_string(),
                handler: HandlerRef {
                    path: "handlers::graph".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(10000),
            },
            ToolDef::Native {
                name: "forjar_show".to_string(),
                description: "Show fully resolved config with templates expanded".to_string(),
                handler: HandlerRef {
                    path: "handlers::show".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(30000),
            },
            ToolDef::Native {
                name: "forjar_status".to_string(),
                description: "Show current state from lock files".to_string(),
                handler: HandlerRef {
                    path: "handlers::status".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(10000),
            },
            ToolDef::Native {
                name: "forjar_trace".to_string(),
                description: "View trace provenance data from apply runs".to_string(),
                handler: HandlerRef {
                    path: "handlers::trace".to_string(),
                    inline: None,
                },
                params: empty_params.clone(),
                timeout_ms: Some(30000),
            },
            ToolDef::Native {
                name: "forjar_anomaly".to_string(),
                description: "Detect anomalous resource behavior using ML-inspired analysis"
                    .to_string(),
                handler: HandlerRef {
                    path: "handlers::anomaly".to_string(),
                    inline: None,
                },
                params: empty_params,
                timeout_ms: Some(30000),
            },
        ],
        resources: vec![],
        prompts: vec![],
        state: None,
    }
}

/// Start the forjar MCP server (stdio transport).
pub async fn serve() -> Result<(), String> {
    let config = build_forge_config();
    let server = McpServer::new(config);

    // Register forjar handlers into the server's registry
    let registry = server.registry();
    {
        let mut reg = registry.write().await;
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

    server
        .run()
        .await
        .map_err(|e| format!("MCP server error: {e}"))
}
