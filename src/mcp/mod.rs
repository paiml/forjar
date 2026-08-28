//! FJ-063: MCP integration via pforge.
//!
//! Exposes forjar operations as MCP tools. The set is NOT listed here — it is
//! `crate::verb::verbs()`, and a second list in a doc comment is the drift this
//! module was restructured to make impossible. Uses pforge-runtime
//! HandlerRegistry for O(1) dispatch and pforge McpServer for protocol
//! handling.

pub mod handlers;
pub mod handlers_ops;
pub mod handlers_state;
pub mod paths;
pub mod registry;
pub mod types;
pub mod types_ops;

#[cfg(test)]
mod tests_dogfood;
#[cfg(test)]
mod tests_handlers;
#[cfg(test)]
mod tests_handlers_more;
#[cfg(test)]
mod tests_parity;
#[cfg(test)]
mod tests_registry;

// Re-export public API
pub use handlers::*;
pub use handlers_ops::*;
pub use registry::{build_registry, export_schema, serve};
pub use types::*;
