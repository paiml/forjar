//! What a forjar verb IS, independent of the transport carrying it.
//!
//! A [`VerbSpec`] is the single declaration of a capability: its name, what it
//! does, whether it mutates anything, and the schemas its params and result
//! must satisfy. Every transport derives its own view from this — the CLI its
//! flags, MCP its `tools/list` entry and annotations, the manifest its rows.
//!
//! The rule that makes it worth having: **nothing here may be stated twice.**
//! `src/mcp/registry.rs` used to declare the same 9 tools in four separate
//! places (`export_schema`, `build_registry`, `build_forge_config`, and again
//! inside `serve`). The literal `forjar_validate` appeared four times in one
//! file, so adding a tenth tool meant editing four lists and the compiler
//! would not notice if you edited three. Worse, `build_registry` was reachable
//! only from tests — production registered its handlers inside `serve`, so the
//! test asserting "the registry has all tools" was asserting it about a
//! registry no user ever touched.

use serde::Serialize;

/// Whether invoking a verb can change anything.
///
/// This exists once, here, because it is published to MCP as `readOnlyHint`.
/// Stating it a second time next to the transport would let the two drift, and
/// a wrong `readOnlyHint` is worse than a missing one: an agent trusts it
/// before deciding whether it may call the tool unattended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Effects {
    /// Reads state and reports. Safe for an agent to call unattended.
    ///
    /// forjar#372 pins what that has to mean, because it was once false:
    /// a `ReadOnly` verb must not run anything the CONFIG declares. `plan`
    /// reached `bash -c` through `ambient_inputs`, `sops`/`op` through
    /// `secrets.provider`, and `bash -c` again through an
    /// `output_equivalence` normaliser — so "call it on an untrusted repo"
    /// meant "execute that repo". `core::unattended::sanitize_config` removes
    /// all three before an unattended plan, and the plan discloses it.
    /// Reading the filesystem and the lock is still `ReadOnly`; spawning a
    /// process a config author chose is not.
    ReadOnly,
    /// May change the host, the lock file, or the config.
    Mutating,
}

impl Effects {
    /// The value published to MCP as `readOnlyHint`. Derived, never restated.
    pub const fn read_only(self) -> bool {
        matches!(self, Effects::ReadOnly)
    }
}

/// One capability, declared once, rendered by every transport.
pub struct VerbSpec {
    /// Transport-neutral name (`validate`), NOT the MCP name (`forjar_validate`).
    pub name: &'static str,
    /// One line, shown by every transport.
    pub description: &'static str,
    /// Whether invoking it can change anything.
    pub effects: Effects,
    /// Upper bound for a transport that needs one.
    pub timeout_ms: u64,
    /// JSON Schema for params, from the handler's own input type.
    pub input_schema: fn() -> serde_json::Value,
    /// JSON Schema for a successful result.
    pub output_schema: fn() -> serde_json::Value,
    /// Invoke the verb. Transport-neutral: JSON in, JSON out.
    ///
    /// Every transport routes through this, so a verb cannot be reachable on
    /// one surface and missing on another — the failure mode that let
    /// `build_registry` and `serve` register different sets.
    pub invoke: fn(serde_json::Value) -> Result<serde_json::Value, String>,
}

impl VerbSpec {
    /// The MCP tool name. DERIVED from `name` so the two cannot disagree —
    /// the prefix was previously typed out at all four declaration sites.
    pub fn mcp_name(&self) -> String {
        format!("forjar_{}", self.name)
    }
}

impl std::fmt::Debug for VerbSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerbSpec")
            .field("name", &self.name)
            .field("effects", &self.effects)
            .field("timeout_ms", &self.timeout_ms)
            .finish_non_exhaustive()
    }
}
