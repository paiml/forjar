//! The Unified Verb Surface — one registry, three derived transports.
//!
//! # The problem this replaces
//!
//! forjar had 159 CLI subcommands and, separately, nine hand-written MCP tools
//! in [`crate::mcp`]. The nine restated what the CLI already knew: the tool
//! name, its description, its parameters, and — in a second implementation —
//! what it does. `src/mcp/tests_parity.rs` records what that cost: two
//! divergences found only by driving the *published* binary over stdio, where
//! the MCP tool and the CLI command of the same name gave different answers
//! about the same project. Neither was visible from the schema, from
//! `tools/list`, or from a handler test asserting "returns Ok".
//!
//! That is the structural failure of a second definition, not a bug in those
//! nine handlers.
//!
//! # The shape
//!
//! ```text
//!            src/cli/commands/mod.rs        <- the ONLY declaration
//!                     |  #[derive(Subcommand)]
//!                     v
//!               clap::Command tree           <- what main parses with
//!                     |  derive::registry()
//!                     v
//!                Vec<VerbSpec>               <- the registry
//!                /     |      \
//!             CLI     MCP     HTTP           <- projections, not definitions
//! ```
//!
//! Every verb, its description, its parameters, their types, their defaults and
//! their closed value sets are read from the clap tree at runtime. No verb is
//! written down twice, so no verb can disagree with itself.
//!
//! Invocation reduces to the same thing on every transport:
//! params → argv → the shipped binary ([`exec::dispatch`]). A transport cannot
//! drift from the CLI because it has no execution path of its own.
//!
//! # What is *not* derived
//!
//! [`spec::Effects`] — whether a verb changes the world. clap does not know
//! that, so it is declared in [`effects`], with the default set to `Mutating`
//! so that forgetting to classify a verb is safe rather than dangerous.

pub mod argv;
pub mod derive;
pub mod effects;
pub mod error;
pub mod exec;
pub mod manifest;
pub mod schema;
pub mod spec;
pub mod validate;

pub use derive::{find, registry, verb_names};
pub use error::VerbError;
pub use exec::{dispatch, VerbCtx};
pub use spec::{Effects, ParamKind, VerbParam, VerbSpec};

/// The registry rendered as a JSON catalogue, as served by `GET /v1/verbs` and
/// by MCP `tools/list`.
#[must_use]
pub fn catalogue() -> serde_json::Value {
    serde_json::json!({
        "server": "forjar",
        "version": env!("CARGO_PKG_VERSION"),
        "verb_count": registry().len(),
        "verbs": registry()
            .iter()
            .map(|v| serde_json::json!({
                "name": v.name,
                "description": v.description,
                "effects": v.effects.as_str(),
                "params_schema": v.params_schema,
                "output_schema": v.output_schema,
            }))
            .collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_counts_what_it_lists() {
        let c = catalogue();
        let listed = c["verbs"].as_array().unwrap().len();
        assert_eq!(c["verb_count"].as_u64().unwrap() as usize, listed);
        assert_eq!(listed, registry().len());
    }

    #[test]
    fn the_catalogue_carries_a_schema_for_every_verb() {
        for v in catalogue()["verbs"].as_array().unwrap() {
            assert!(v["params_schema"]["properties"].is_object(), "{v}");
            assert!(!v["description"].as_str().unwrap().is_empty(), "{v}");
            assert!(
                ["read-only", "mutating", "transport"].contains(&v["effects"].as_str().unwrap()),
                "{v}"
            );
        }
    }

    #[test]
    fn the_catalogue_reports_the_running_version() {
        assert_eq!(catalogue()["version"], env!("CARGO_PKG_VERSION"));
    }
}
