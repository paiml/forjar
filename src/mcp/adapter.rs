//! The pmcp tool adapter forjar serves over stdio (paiml/forjar#375).
//!
//! # Why forjar owns this and pforge does not
//!
//! `forjar mcp --schema` published `annotations.readOnlyHint` for every tool.
//! The running server published nothing. Measured on 1.24.0 over real stdio,
//! the key set of every one of the twelve tool objects was exactly
//! `['description', 'inputSchema', 'name']` — no `annotations`, no
//! `outputSchema`.
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
    /// `schemars`-derived schema of the verb's output type.
    pub output_schema: Value,
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
    fn metadata(&self) -> Option<ToolInfo> {
        let mut info = ToolInfo::new(
            self.name.clone(),
            Some(self.description.clone()),
            self.input_schema.clone(),
        );
        info.annotations = Some(ToolAnnotations::new().with_read_only(self.read_only));
        info.output_schema = Some(self.output_schema.clone());
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
            output_schema: serde_json::json!({"type": "object", "title": "Out"}),
        }
    }

    /// The metadata pforge dropped is present, and it SERIALISES under the
    /// camelCase names the MCP spec uses — `read_only_hint` reaching the wire
    /// as `read_only_hint` would be just as unreadable as not sending it.
    #[test]
    fn metadata_carries_the_hint_and_the_output_schema() {
        use pmcp::server::ToolHandler;
        let info = adapter(true).metadata().expect("metadata");
        let v = serde_json::to_value(&info).expect("ToolInfo serialises");

        assert_eq!(v["annotations"]["readOnlyHint"], serde_json::json!(true));
        assert_eq!(v["outputSchema"]["title"], serde_json::json!("Out"));
        assert_eq!(v["name"], serde_json::json!("forjar_probe"));
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
