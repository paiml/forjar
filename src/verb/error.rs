//! The total error taxonomy.
//!
//! # Why this type exists
//!
//! Before the Unified Verb Surface, `main` chose an exit code by matching
//! substrings against a `String` error — `error.contains("SSH")` meant exit 4.
//! Every new transport would have had to reproduce that guess, and any two of
//! them would have disagreed the first time an error message was reworded.
//!
//! [`VerbError`] is that decision made once. Each variant maps to exactly one
//! exit code, one JSON-RPC error code, and one HTTP status, through total
//! `match` expressions with no wildcard arm — so a new variant fails to compile
//! until all three mappings are given, rather than silently inheriting a
//! neighbour's code.

use std::fmt;

/// Every way a verb invocation can fail, on any transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerbError {
    /// No verb by that name is in the registry.
    UnknownVerb(String),
    /// The params object did not satisfy the verb's schema.
    InvalidParams(String),
    /// The verb exists but this transport refuses to invoke it.
    NotInvocable(String),
    /// The verb ran and exited non-zero.
    Failed { verb: String, exit_code: i32 },
    /// The verb could not be started at all.
    Spawn(String),
    /// The request itself was malformed (bad JSON, bad framing, bad route).
    Malformed(String),
}

impl VerbError {
    /// The process exit code this error produces on the CLI.
    ///
    /// Preserves forjar's documented codes (FJ-2301): 1 general, 2 partial,
    /// 3 configuration, 4 connection, 10 drift.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            VerbError::UnknownVerb(_) => 1,
            VerbError::InvalidParams(_) => 3,
            VerbError::NotInvocable(_) => 1,
            VerbError::Failed { exit_code, .. } => *exit_code,
            VerbError::Spawn(_) => 1,
            VerbError::Malformed(_) => 1,
        }
    }

    /// The JSON-RPC 2.0 error code (§5.1).
    ///
    /// `-32601` method not found, `-32602` invalid params, `-32603` internal,
    /// `-32700`/`-32600` parse and invalid-request.
    #[must_use]
    pub fn jsonrpc_code(&self) -> i32 {
        match self {
            VerbError::UnknownVerb(_) => -32601,
            VerbError::InvalidParams(_) => -32602,
            VerbError::NotInvocable(_) => -32601,
            VerbError::Failed { .. } => -32603,
            VerbError::Spawn(_) => -32603,
            VerbError::Malformed(_) => -32600,
        }
    }

    /// The HTTP status code.
    #[must_use]
    pub fn http_status(&self) -> u16 {
        match self {
            VerbError::UnknownVerb(_) => 404,
            VerbError::InvalidParams(_) => 400,
            VerbError::NotInvocable(_) => 403,
            VerbError::Failed { .. } => 200,
            VerbError::Spawn(_) => 500,
            VerbError::Malformed(_) => 400,
        }
    }

    /// A stable machine-readable tag, so a client can branch without parsing
    /// prose.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            VerbError::UnknownVerb(_) => "unknown_verb",
            VerbError::InvalidParams(_) => "invalid_params",
            VerbError::NotInvocable(_) => "not_invocable",
            VerbError::Failed { .. } => "failed",
            VerbError::Spawn(_) => "spawn",
            VerbError::Malformed(_) => "malformed",
        }
    }

    /// Every variant, for exhaustiveness tests over the mappings.
    #[must_use]
    pub fn all_kinds() -> Vec<VerbError> {
        vec![
            VerbError::UnknownVerb("v".into()),
            VerbError::InvalidParams("p".into()),
            VerbError::NotInvocable("v".into()),
            VerbError::Failed {
                verb: "v".into(),
                exit_code: 7,
            },
            VerbError::Spawn("s".into()),
            VerbError::Malformed("m".into()),
        ]
    }
}

impl fmt::Display for VerbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerbError::UnknownVerb(v) => write!(f, "unknown verb: {v}"),
            VerbError::InvalidParams(m) => write!(f, "invalid params: {m}"),
            VerbError::NotInvocable(v) => {
                write!(
                    f,
                    "verb `{v}` is a transport and cannot be invoked remotely"
                )
            }
            VerbError::Failed { verb, exit_code } => {
                write!(f, "verb `{verb}` exited {exit_code}")
            }
            VerbError::Spawn(m) => write!(f, "could not run verb: {m}"),
            VerbError::Malformed(m) => write!(f, "malformed request: {m}"),
        }
    }
}

impl std::error::Error for VerbError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// The number of variants. A new variant must update this, which is the
    /// prompt to check its three mappings.
    const VARIANTS: usize = 6;

    #[test]
    fn all_kinds_covers_every_variant() {
        assert_eq!(VerbError::all_kinds().len(), VARIANTS);
        let tags: HashSet<_> = VerbError::all_kinds().iter().map(VerbError::kind).collect();
        assert_eq!(tags.len(), VARIANTS, "kind() must be injective");
    }

    #[test]
    fn every_variant_maps_to_a_valid_exit_code() {
        for e in VerbError::all_kinds() {
            let c = e.exit_code();
            assert!((0..=255).contains(&c), "{e:?} -> {c}");
        }
    }

    #[test]
    fn every_variant_maps_to_a_reserved_or_server_jsonrpc_code() {
        for e in VerbError::all_kinds() {
            let c = e.jsonrpc_code();
            assert!(
                (-32768..=-32000).contains(&c),
                "{e:?} -> {c} is outside the JSON-RPC reserved range"
            );
        }
    }

    #[test]
    fn every_variant_maps_to_a_real_http_status() {
        for e in VerbError::all_kinds() {
            let s = e.http_status();
            assert!((100..600).contains(&s), "{e:?} -> {s}");
        }
    }

    #[test]
    fn a_failing_verb_is_a_200_carrying_its_exit_code() {
        // A verb that runs and reports a problem is a successful *transport*
        // exchange. Mapping it to 500 would make `plan` on a drifted config
        // indistinguishable from the server falling over.
        let e = VerbError::Failed {
            verb: "plan".into(),
            exit_code: 10,
        };
        assert_eq!(e.http_status(), 200);
        assert_eq!(e.exit_code(), 10, "the verb's own code must survive");
    }

    #[test]
    fn invalid_params_is_a_configuration_error_not_a_generic_one() {
        assert_eq!(VerbError::InvalidParams("x".into()).exit_code(), 3);
        assert_eq!(VerbError::InvalidParams("x".into()).http_status(), 400);
        assert_eq!(VerbError::InvalidParams("x".into()).jsonrpc_code(), -32602);
    }

    #[test]
    fn unknown_verb_and_not_invocable_differ_over_http() {
        // Both are "method not found" to JSON-RPC, which has no finer code, but
        // HTTP can and must distinguish "no such verb" from "exists, refused".
        let unknown = VerbError::UnknownVerb("nope".into());
        let refused = VerbError::NotInvocable("serve".into());
        assert_eq!(unknown.jsonrpc_code(), refused.jsonrpc_code());
        assert_ne!(unknown.http_status(), refused.http_status());
        assert_eq!(unknown.http_status(), 404);
        assert_eq!(refused.http_status(), 403);
    }

    #[test]
    fn display_names_the_verb_or_the_reason() {
        assert_eq!(
            VerbError::UnknownVerb("zzz".into()).to_string(),
            "unknown verb: zzz"
        );
        assert!(VerbError::NotInvocable("serve".into())
            .to_string()
            .contains("serve"));
        assert!(VerbError::Failed {
            verb: "apply".into(),
            exit_code: 2
        }
        .to_string()
        .contains("exited 2"));
    }
}
