//! FJ-063: MCP integration via pforge.
//!
//! Exposes forjar operations as MCP tools: validate, plan, drift,
//! lint, graph, show, status, trace, anomaly. Uses pforge-runtime HandlerRegistry for
//! O(1) dispatch and pforge McpServer for protocol handling.

pub mod handlers;
pub mod registry;
pub mod types;

#[cfg(test)]
mod tests_handlers;
#[cfg(test)]
mod tests_handlers_more;
#[cfg(test)]
mod tests_registry;

// Re-export public API
pub use handlers::*;
pub use registry::{build_registry, export_schema, serve};
pub use types::*;
