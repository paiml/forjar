//! The verb specification — what every transport sees, and nothing more.

use serde::{Deserialize, Serialize};

/// What invoking a verb may do to the world.
///
/// # Why the default is the unsafe one
///
/// Effects are the one property of a verb that clap does not know: nothing in
/// an `#[arg]` attribute says whether `apply` mutates a host. It therefore has
/// to be declared somewhere, and a declaration can drift from the code.
///
/// The drift is made harmless by choosing the direction of the default. A verb
/// with no classification is [`Effects::Mutating`], so *forgetting* to classify
/// a new verb makes it more restricted, never less: it is excluded from
/// read-only surfaces rather than silently admitted to them. Only the explicit
/// read-only allowlist in [`super::effects`] can relax that, and
/// [`super::effects::allowlist_is_live`] fails if it names a verb that no
/// longer exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Effects {
    /// Reads configuration or state; writes nothing outside stdout/stderr.
    ReadOnly,
    /// May mutate local files, state directories, or remote machines.
    #[default]
    Mutating,
    /// Runs a server or long-lived process; not invocable through a transport.
    ///
    /// These verbs are in the registry — they are part of the surface and must
    /// be described — but [`super::exec::dispatch`] refuses them, because
    /// serving a transport from inside that transport is unbounded recursion.
    Transport,
}

impl Effects {
    /// The wire spelling, used in the manifest and in `/v1/verbs`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Effects::ReadOnly => "read-only",
            Effects::Mutating => "mutating",
            Effects::Transport => "transport",
        }
    }

    /// Whether a transport may invoke this verb at all.
    #[must_use]
    pub fn is_invocable(self) -> bool {
        match self {
            Effects::ReadOnly | Effects::Mutating => true,
            Effects::Transport => false,
        }
    }
}

/// How a parameter is spelled on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParamKind {
    /// `--flag` — presence is the value.
    Flag,
    /// `--flag` repeated; the count is the value.
    Count,
    /// `--name VALUE` — a single value.
    Value,
    /// `--name VALUE` repeated, or a multi-valued option.
    Multi,
    /// A bare positional argument.
    Positional,
}

/// One parameter of a verb, derived from a [`clap::Arg`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerbParam {
    /// The JSON object key, matching clap's arg id (e.g. `state_dir`).
    pub name: String,
    /// The command-line spelling without dashes (e.g. `state-dir`), when the
    /// parameter is an option. `None` for positionals.
    pub long: Option<String>,
    /// Help text, taken from the field's doc comment.
    pub description: String,
    /// Whether clap requires the parameter.
    pub required: bool,
    /// How the parameter is spelled on the command line.
    pub kind: ParamKind,
    /// The closed set of accepted values, for `ValueEnum` parameters.
    pub choices: Vec<String>,
    /// clap's default, if it declares one.
    pub default: Option<String>,
}

/// A verb: one callable operation, identical on every transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerbSpec {
    /// The wire name, e.g. `lock-verify`. Matches the CLI subcommand exactly.
    pub name: String,
    /// One-line description, taken from the variant's doc comment.
    pub description: String,
    /// JSON Schema (draft 2020-12) for the params object.
    pub params_schema: serde_json::Value,
    /// JSON Schema for the result envelope.
    pub output_schema: serde_json::Value,
    /// What invoking this verb may do.
    pub effects: Effects,
    /// The parameters, in clap declaration order.
    pub params: Vec<VerbParam>,
    /// Nested subcommand names for grouped verbs (e.g. `workspace` →
    /// `new`, `list`, …). Empty for leaf verbs.
    pub subcommands: Vec<String>,
}

impl VerbSpec {
    /// Look up a parameter by its JSON key.
    #[must_use]
    pub fn param(&self, name: &str) -> Option<&VerbParam> {
        self.params.iter().find(|p| p.name == name)
    }

    /// Whether this verb takes a nested subcommand as its first argument.
    #[must_use]
    pub fn is_grouped(&self) -> bool {
        !self.subcommands.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_effects_are_mutating_not_read_only() {
        // This is the safety property, not a formatting detail: if the default
        // ever flips, an unclassified verb becomes reachable from surfaces that
        // promise not to change anything.
        assert_eq!(Effects::default(), Effects::Mutating);
    }

    #[test]
    fn effects_wire_names_are_stable() {
        assert_eq!(Effects::ReadOnly.as_str(), "read-only");
        assert_eq!(Effects::Mutating.as_str(), "mutating");
        assert_eq!(Effects::Transport.as_str(), "transport");
    }

    #[test]
    fn transport_verbs_are_not_invocable() {
        assert!(Effects::ReadOnly.is_invocable());
        assert!(Effects::Mutating.is_invocable());
        assert!(!Effects::Transport.is_invocable());
    }

    #[test]
    fn param_lookup_matches_by_json_key() {
        let spec = VerbSpec {
            name: "x".into(),
            description: "d".into(),
            params_schema: serde_json::json!({}),
            output_schema: serde_json::json!({}),
            effects: Effects::ReadOnly,
            params: vec![VerbParam {
                name: "state_dir".into(),
                long: Some("state-dir".into()),
                description: "d".into(),
                required: false,
                kind: ParamKind::Value,
                choices: vec![],
                default: None,
            }],
            subcommands: vec![],
        };
        assert!(spec.param("state_dir").is_some());
        // The CLI spelling is NOT a valid JSON key; accepting both would make
        // two spellings for one parameter.
        assert!(spec.param("state-dir").is_none());
        assert!(!spec.is_grouped());
    }
}
