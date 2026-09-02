//! MCP registry, server, and schema export for forjar tools.

use std::sync::Arc;

use pforge_runtime::HandlerRegistry;
use tokio::sync::RwLock;

use super::adapter::VerbToolAdapter;
use super::handlers::*;
use super::handlers_ops::*;

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
    reg.register("forjar_remediate", RemediateHandler);
    reg.register("forjar_audit", AuditHandler);
    reg.register("forjar_workspace", WorkspaceHandler);
}

/// Build the MCP handler registry with all forjar tool handlers.
pub fn build_registry() -> HandlerRegistry {
    let mut registry = HandlerRegistry::new();
    register_all(&mut registry);
    registry
}

/// Build the pmcp server: one tool per verb-table row, metadata and all.
///
/// forjar#375: this used to be a `pforge_config::ForgeConfig` handed to
/// `pforge_runtime::McpServer`, and pforge's own tool adapter answered
/// `metadata()` with a bare `ToolInfo::new(..)` — which hard-sets
/// `annotations: None, output_schema: None`. So `--schema` published
/// `readOnlyHint` for all twelve tools and a connected agent received it for
/// none. Building the pmcp server here is what lets [`VerbToolAdapter`] fill
/// the annotation in.
///
/// Nothing observable was lost with `ForgeConfig`: pforge never enforced
/// `timeout_ms` for `Native` tools (its serve path applies it to `Cli` and
/// `Http` handlers only, and prints "requires handler implementation" for
/// `Native`), and the `inputSchema` it derived was already byte-identical to
/// `(v.input_schema)()`.
///
/// `(v.output_schema)()` is NOT handed over: an `outputSchema` on the wire
/// promises `structuredContent` that pmcp 1.20 only sends for widget tools, and
/// the official MCP client rejects every call to a tool that promises it and
/// does not deliver. `--schema` still publishes it, where it documents rather
/// than promises. See the header of [`super::adapter`].
///
/// `.name("forjar-mcp")` is asserted by `e2e_mcp_stdio_t`; `.tool()` is what
/// sets the `tools` capability the same suite checks.
fn build_server(registry: Arc<RwLock<HandlerRegistry>>) -> Result<pmcp::Server, String> {
    let mut builder = pmcp::Server::builder()
        .name("forjar-mcp")
        .version(env!("CARGO_PKG_VERSION"));

    for v in crate::verb::verbs() {
        builder = builder.tool(
            v.mcp_name(),
            VerbToolAdapter {
                registry: registry.clone(),
                name: v.mcp_name(),
                description: v.description.to_string(),
                // Derived, never a literal — see VerbToolAdapter::read_only.
                read_only: v.effects.read_only(),
                input_schema: (v.input_schema)(),
            },
        );
    }

    builder
        .build()
        .map_err(|e| format!("cannot build MCP server: {e}"))
}

/// Start the forjar MCP server (stdio transport).
pub async fn serve() -> Result<(), String> {
    // The SAME `register_all` list `build_registry()` uses, so the tested set
    // and the served set cannot differ — and dispatch stays with pforge's
    // registry, which is load-bearing rather than incidental. See the header of
    // `mcp::adapter`.
    let registry = Arc::new(RwLock::new(HandlerRegistry::new()));
    register_all(&mut *registry.write().await);

    build_server(registry)?
        .run_stdio()
        .await
        .map_err(|e| format!("MCP server error: {e}"))
}
