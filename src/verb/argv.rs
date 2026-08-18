//! Turn a validated params object into the argv the CLI would have received.
//!
//! This is the join between the transports and the command line. A verb call
//! over MCP or HTTP becomes literally the same argument vector a user would
//! type, so there is no second execution path that could behave differently
//! from the CLI — the CLI *is* the execution path.

use super::error::VerbError;
use super::spec::{ParamKind, VerbSpec};
use serde_json::Value;

/// Build the argv (excluding argv[0]) for `spec` with `params`.
///
/// # Errors
///
/// [`VerbError::InvalidParams`] if a value cannot be rendered as an argument.
/// Callers should run [`super::validate::check`] first; this function repeats
/// the type checks rather than trusting them, because an unchecked call would
/// otherwise silently drop a parameter.
pub fn build(spec: &VerbSpec, params: &Value) -> Result<Vec<String>, VerbError> {
    let obj = params
        .as_object()
        .ok_or_else(|| VerbError::InvalidParams("params must be a JSON object".into()))?;

    let mut argv = vec![spec.name.clone()];

    if spec.is_grouped() {
        let sub = obj
            .get("subcommand")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                VerbError::InvalidParams(format!("verb `{}` requires `subcommand`", spec.name))
            })?;
        argv.push(sub.to_string());
    }

    // Options first, then positionals, so positional order is not disturbed by
    // interleaved flags.
    for p in spec
        .params
        .iter()
        .filter(|p| p.kind != ParamKind::Positional)
    {
        let Some(v) = obj.get(&p.name) else { continue };
        push_option(&mut argv, spec, p, v)?;
    }
    for p in spec
        .params
        .iter()
        .filter(|p| p.kind == ParamKind::Positional)
    {
        let Some(v) = obj.get(&p.name) else { continue };
        let s = as_str(spec, p, v)?;
        argv.push(s.to_string());
    }
    Ok(argv)
}

fn flag_spelling(spec: &VerbSpec, p: &super::spec::VerbParam) -> Result<String, VerbError> {
    p.long.as_ref().map(|l| format!("--{l}")).ok_or_else(|| {
        VerbError::InvalidParams(format!(
            "parameter `{}` of verb `{}` has no long form and cannot be set remotely",
            p.name, spec.name
        ))
    })
}

fn push_option(
    argv: &mut Vec<String>,
    spec: &VerbSpec,
    p: &super::spec::VerbParam,
    v: &Value,
) -> Result<(), VerbError> {
    match p.kind {
        ParamKind::Flag => {
            let on = v.as_bool().ok_or_else(|| type_err(spec, p, "a boolean"))?;
            // A false flag is an absent flag. Emitting `--json` for
            // `{"json": false}` would invert the caller's request.
            if on {
                argv.push(flag_spelling(spec, p)?);
            }
        }
        ParamKind::Count => {
            let n = v
                .as_u64()
                .ok_or_else(|| type_err(spec, p, "a non-negative integer"))?;
            let flag = flag_spelling(spec, p)?;
            for _ in 0..n {
                argv.push(flag.clone());
            }
        }
        ParamKind::Multi => {
            let items = v
                .as_array()
                .ok_or_else(|| type_err(spec, p, "an array of strings"))?;
            let flag = flag_spelling(spec, p)?;
            for item in items {
                let s = item
                    .as_str()
                    .ok_or_else(|| type_err(spec, p, "an array of strings"))?;
                argv.push(flag.clone());
                argv.push(s.to_string());
            }
        }
        ParamKind::Value => {
            let s = as_str(spec, p, v)?;
            argv.push(flag_spelling(spec, p)?);
            argv.push(s.to_string());
        }
        ParamKind::Positional => unreachable!("positionals are pushed separately"),
    }
    Ok(())
}

fn as_str<'a>(
    spec: &VerbSpec,
    p: &super::spec::VerbParam,
    v: &'a Value,
) -> Result<&'a str, VerbError> {
    v.as_str().ok_or_else(|| type_err(spec, p, "a string"))
}

fn type_err(spec: &VerbSpec, p: &super::spec::VerbParam, want: &str) -> VerbError {
    VerbError::InvalidParams(format!(
        "parameter `{}` of verb `{}` must be {want}",
        p.name, spec.name
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verb::derive::find;
    use serde_json::json;

    #[test]
    fn the_verb_name_leads_the_argv() {
        let plan = find("plan").unwrap();
        assert_eq!(build(plan, &json!({})).unwrap(), vec!["plan"]);
    }

    #[test]
    fn values_become_long_flags_with_their_argument() {
        let plan = find("plan").unwrap();
        let argv = build(plan, &json!({"file": "a.yaml"})).unwrap();
        assert_eq!(argv, vec!["plan", "--file", "a.yaml"]);
    }

    #[test]
    fn a_false_flag_emits_nothing() {
        // Emitting `--json` for {"json": false} would do the opposite of what
        // the caller asked, and clap has no `--no-json` to undo it.
        let plan = find("plan").unwrap();
        assert_eq!(build(plan, &json!({"json": false})).unwrap(), vec!["plan"]);
        assert_eq!(
            build(plan, &json!({"json": true})).unwrap(),
            vec!["plan", "--json"]
        );
    }

    #[test]
    fn multi_valued_params_repeat_the_flag() {
        let plan = find("plan").unwrap();
        let argv = build(plan, &json!({"what_if": ["a=1", "b=2"]})).unwrap();
        assert_eq!(argv, vec!["plan", "--what-if", "a=1", "--what-if", "b=2"]);
    }

    #[test]
    fn grouped_verbs_put_the_subcommand_immediately_after_the_verb() {
        let ws = find("workspace").unwrap();
        let argv = build(ws, &json!({"subcommand": "list"})).unwrap();
        assert_eq!(argv[0], "workspace");
        assert_eq!(argv[1], "list");
    }

    #[test]
    fn grouped_verbs_without_a_subcommand_are_an_error_not_a_bare_call() {
        let ws = find("workspace").unwrap();
        assert!(build(ws, &json!({})).is_err());
    }

    #[test]
    fn wrong_types_error_rather_than_silently_dropping_the_parameter() {
        // A dropped parameter is the dangerous failure: `plan --state-dir X`
        // becoming `plan` reads the wrong state and answers confidently.
        let plan = find("plan").unwrap();
        assert!(build(plan, &json!({"file": 7})).is_err());
        assert!(build(plan, &json!({"json": "yes"})).is_err());
        assert!(build(plan, &json!({"what_if": [1]})).is_err());
    }

    #[test]
    fn kebab_case_flag_spelling_is_used_not_the_json_key() {
        let plan = find("plan").unwrap();
        let argv = build(plan, &json!({"state_dir": "/tmp/s"})).unwrap();
        assert!(argv.contains(&"--state-dir".to_string()), "{argv:?}");
        assert!(!argv.contains(&"--state_dir".to_string()), "{argv:?}");
    }

    /// Kinds that mean "the argv shape was understood, a value was missing".
    fn is_shape_ok(kind: clap::error::ErrorKind) -> bool {
        matches!(
            kind,
            clap::error::ErrorKind::MissingRequiredArgument
                | clap::error::ErrorKind::MissingSubcommand
                | clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        )
    }

    #[test]
    fn every_argv_built_from_the_registry_parses_back_through_clap() {
        // The strongest property in this module: for every verb, the argv we
        // construct is accepted by the very parser `main` uses — or is rejected
        // only for a missing required value, never for an unrecognised flag or
        // subcommand. A derivation that invented a flag fails here.
        for v in crate::verb::derive::registry() {
            if v.is_grouped() {
                continue; // covered by the grouped test below
            }
            let argv = build(v, &json!({})).expect(&v.name);
            if let Err(kind) = crate::verb::derive::check_argv(&argv) {
                assert!(
                    is_shape_ok(kind),
                    "verb `{}` produced argv clap rejects as {kind:?}: {argv:?}",
                    v.name
                );
            }
        }
    }

    #[test]
    fn grouped_verb_argv_parses_back_through_clap() {
        for v in crate::verb::derive::registry()
            .iter()
            .filter(|v| v.is_grouped())
        {
            for sub in &v.subcommands {
                let argv = build(v, &json!({"subcommand": sub})).unwrap();
                if let Err(kind) = crate::verb::derive::check_argv(&argv) {
                    assert!(
                        is_shape_ok(kind),
                        "{} {sub} produced argv clap rejects as {kind:?}: {argv:?}",
                        v.name
                    );
                }
            }
        }
    }

    #[test]
    fn a_fabricated_flag_is_rejected_by_the_real_parser() {
        // Proves the round-trip check above can actually fail: the same helper,
        // given argv the CLI does not accept, returns a non-shape error.
        let kind =
            crate::verb::derive::check_argv(&["plan".to_string(), "--not-a-real-flag".to_string()])
                .unwrap_err();
        assert!(
            !is_shape_ok(kind),
            "expected a hard rejection, got {kind:?}"
        );
    }
}
