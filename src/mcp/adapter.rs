//! The pmcp tool adapter forjar serves over stdio (paiml/forjar#375).
//!
//! # Why forjar owns this and pforge does not
//!
//! `forjar mcp --schema` published `annotations.readOnlyHint` for every tool.
//! The running server published nothing. Measured on 1.24.0 over real stdio,
//! the key set of every one of the twelve tool objects was exactly
//! `['description', 'inputSchema', 'name']` — no `annotations`.
//!
//! The discard point is three layers down and none of them is forjar's:
//! `pforge_config::ToolDef::Native` carries no annotations field, and
//! `pforge_runtime`'s own `PforgeToolAdapter::metadata()` returns a bare
//! `pmcp::types::ToolInfo::new(..)`, which hard-sets `output_schema: None,
//! annotations: None`. Both are byte-identical in pforge 0.2.1, so a version
//! bump fixes nothing. The wire type is willing — pmcp caches `metadata()`
//! verbatim at build time and serialises it whole — so the fix is to hand pmcp
//! a `ToolInfo` forjar filled in.
//!
//! That matters more than a missing field. `Effects::ReadOnly` means "safe for
//! an agent to call unattended", and the argument for it is that an agent
//! consults the hint before deciding it needs a human. It could not: the field
//! was never sent. Six test files asserted `readOnlyHint` and every one of them
//! read `export_schema()` or `mcp --schema`; the single suite that drove real
//! `tools/list` asserted only `name` and `inputSchema`. Agreement between two
//! non-wire surfaces cannot falsify what goes over the wire.
//!
//! # Why `outputSchema` is NOT published, though the same line dropped it
//!
//! It looks free and it is not. MCP 2025-06-18 — the revision this server
//! negotiates, measured — makes an output schema a PROMISE: *"If an output
//! schema is provided: Servers MUST provide structured results that conform to
//! this output schema."* pmcp 1.20 cannot keep it. Its `handle_call_tool` builds
//! `CallToolResult::new(vec![Content::Text { .. }])` and attaches
//! `structuredContent` only through `with_widget_enrichment`, which fires solely
//! for tools carrying ChatGPT widget `_meta`. Nothing a `ToolHandler` returns
//! reaches `structuredContent`, `TypedToolWithOutput` included.
//!
//! So a published `outputSchema` here is an unkeepable promise, and clients
//! enforce it. Driven against this binary with the OFFICIAL MCP TypeScript SDK
//! client (`@modelcontextprotocol/sdk` 1.30.0), which caches each tool's output
//! schema at `listTools()` and checks it on every call:
//!
//! ```text
//!   with `info.output_schema = Some(..)`:
//!     McpError -32600: Tool forjar_validate has an output schema but did not
//!                      return structured content
//!   without it:
//!     content: [{ type: "text", text: "{"valid":true,…}" }], isError: false
//! ```
//!
//! Every `tools/call` failed, on all twelve tools, for the most widely deployed
//! client stack there is — a strictly worse outcome than the missing hint #375
//! was opened for, which at least failed SAFE. `readOnlyHint` carries no such
//! obligation, which is why it is published and this is not.
//! `falsification_mcp_publishes_readonly_hint_over_stdio` asserts the pairing
//! rather than the absence, so the day pmcp can fill `structuredContent` the
//! schema goes back in and the test says so.
//!
//! # What is deliberately NOT changed
//!
//! DISPATCH stays with `pforge_runtime::HandlerRegistry` — the same registry
//! `build_registry()` returns and the tests exercise, so the tested set and the
//! served set still cannot diverge, and `dispatch(&str, &[u8]) -> Vec<u8>`
//! carries no pmcp type across the boundary.
//!
//! The verb table already has a JSON-in/JSON-out entry point, `VerbSpec::invoke`,
//! and wiring it in here is the obvious simplification. DO NOT. `invoke` builds
//! a `tokio::runtime::Runtime` internally (`verb/registry.rs`), and [`handle`] is
//! already running on one, so every `tools/call` would panic with "Cannot start
//! a runtime from within a runtime". Its existing callers — `verb::cli` and
//! `verb::http` — are synchronous, which is why they are fine and this is not.
//! `falsification_mcp_publishes_readonly_hint_over_stdio` dispatches every
//! advertised tool for exactly this reason: an adapter that made that swap would
//! satisfy every annotation assertion and serve a dead surface.
//!
//! [`handle`]: VerbToolAdapter::handle

use std::sync::Arc;

use pforge_runtime::HandlerRegistry;
use pmcp::types::{ToolAnnotations, ToolInfo};
use serde_json::Value;
use tokio::sync::RwLock;

/// One forjar verb, published to pmcp with the metadata pforge dropped.
pub struct VerbToolAdapter {
    /// The registry that owns every handler; dispatch goes through it.
    pub registry: Arc<RwLock<HandlerRegistry>>,
    /// The MCP name — `forjar_<verb>`, derived from the row, never typed.
    pub name: String,
    /// The verb's description, the same string `--schema` publishes.
    pub description: String,
    /// `Effects::read_only()` for this row. NEVER a literal: #356 established
    /// that a hint which is published and wrong is worse than one that is
    /// absent, so the first `Mutating` row to land must publish `false` here
    /// without anyone remembering to come back and edit it.
    pub read_only: bool,
    /// `schemars`-derived schema of the verb's input type.
    pub input_schema: Value,
}

#[async_trait::async_trait]
impl pmcp::server::ToolHandler for VerbToolAdapter {
    /// A faithful copy of `PforgeToolAdapter::handle`: JSON in, bytes to the
    /// registry, JSON back out. See the module header for why this is not
    /// `(v.invoke)(args)`.
    async fn handle(&self, args: Value, _extra: pmcp::RequestHandlerExtra) -> pmcp::Result<Value> {
        let params = serde_json::to_vec(&args)
            .map_err(|e| pmcp::Error::protocol_msg(format!("cannot serialise args: {e}")))?;

        let out = self
            .registry
            .read()
            .await
            .dispatch(&self.name, &params)
            .await
            .map_err(|e| pmcp::Error::protocol_msg(e.to_string()))?;

        serde_json::from_slice(&out)
            .map_err(|e| pmcp::Error::protocol_msg(format!("cannot deserialise result: {e}")))
    }

    /// The tool object a connected client receives.
    ///
    /// `ToolInfo` and `ToolAnnotations` are `#[non_exhaustive]`, so this is
    /// `new()` plus field assignment rather than a struct literal.
    ///
    /// `output_schema` is deliberately left `None` — see the module header: it
    /// is a promise pmcp 1.20 cannot keep, and the official client rejects every
    /// call to a tool that makes it and does not.
    fn metadata(&self) -> Option<ToolInfo> {
        let mut info = ToolInfo::new(
            self.name.clone(),
            Some(self.description.clone()),
            self.input_schema.clone(),
        );
        info.annotations = Some(ToolAnnotations::new().with_read_only(self.read_only));
        Some(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter(read_only: bool) -> VerbToolAdapter {
        VerbToolAdapter {
            registry: Arc::new(RwLock::new(HandlerRegistry::new())),
            name: "forjar_probe".to_string(),
            description: "probe".to_string(),
            read_only,
            input_schema: serde_json::json!({"type": "object"}),
        }
    }

    /// The metadata pforge dropped is present, and it SERIALISES under the
    /// camelCase names the MCP spec uses — `read_only_hint` reaching the wire
    /// as `read_only_hint` would be just as unreadable as not sending it.
    #[test]
    fn metadata_carries_the_hint() {
        use pmcp::server::ToolHandler;
        let info = adapter(true).metadata().expect("metadata");
        let v = serde_json::to_value(&info).expect("ToolInfo serialises");

        assert_eq!(v["annotations"]["readOnlyHint"], serde_json::json!(true));
        assert_eq!(v["name"], serde_json::json!("forjar_probe"));
    }

    /// An `outputSchema` obligates `structuredContent` under MCP 2025-06-18, and
    /// pmcp 1.20 never sends it for a non-widget tool. Publishing one made the
    /// official TypeScript client reject EVERY call with
    /// "has an output schema but did not return structured content" — so the
    /// field stays absent until the runtime can fill the promise.
    #[test]
    fn metadata_makes_no_promise_the_runtime_cannot_keep() {
        use pmcp::server::ToolHandler;
        let info = adapter(true).metadata().expect("metadata");
        let v = serde_json::to_value(&info).expect("ToolInfo serialises");

        assert!(
            v.get("outputSchema").is_none_or(serde_json::Value::is_null),
            "an outputSchema on the wire promises structuredContent that pmcp \
             1.20 does not send, and the official client refuses the call: {v}"
        );
    }

    /// The hint is a FUNCTION of the row, not a constant. A literal `true`
    /// would publish a lie the day a `Mutating` verb joins the table.
    #[test]
    fn the_hint_follows_the_argument_rather_than_a_literal() {
        use pmcp::server::ToolHandler;
        let info = adapter(false).metadata().expect("metadata");
        let v = serde_json::to_value(&info).expect("ToolInfo serialises");
        assert_eq!(v["annotations"]["readOnlyHint"], serde_json::json!(false));
    }
}
