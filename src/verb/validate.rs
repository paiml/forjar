//! Validate a params object against a verb's derived parameter list.
//!
//! The check reads [`VerbSpec::params`] rather than re-parsing the emitted JSON
//! Schema. Both are projections of the same derived list, so the validator
//! cannot accept something the published schema forbids — there is one
//! definition, consulted twice, not two definitions kept in step.

use super::error::VerbError;
use super::spec::{ParamKind, VerbSpec};
use serde_json::Value;

/// Validate `params` for `spec`, returning the first problem found.
///
/// # Errors
///
/// [`VerbError::InvalidParams`] when the object has an unknown key, is missing
/// a required key, has a value of the wrong JSON type, or supplies a value
/// outside a closed enum.
pub fn check(spec: &VerbSpec, params: &Value) -> Result<(), VerbError> {
    let obj = params
        .as_object()
        .ok_or_else(|| VerbError::InvalidParams("params must be a JSON object".into()))?;

    for key in obj.keys() {
        if key == "subcommand" {
            if !spec.is_grouped() {
                return Err(VerbError::InvalidParams(format!(
                    "verb `{}` takes no subcommand",
                    spec.name
                )));
            }
            continue;
        }
        if spec.param(key).is_none() {
            return Err(VerbError::InvalidParams(format!(
                "unknown parameter `{key}` for verb `{}`",
                spec.name
            )));
        }
    }

    if spec.is_grouped() {
        check_subcommand(spec, obj.get("subcommand"))?;
    }

    for p in &spec.params {
        match obj.get(&p.name) {
            None => {
                if p.required {
                    return Err(VerbError::InvalidParams(format!(
                        "missing required parameter `{}` for verb `{}`",
                        p.name, spec.name
                    )));
                }
            }
            Some(v) => check_value(spec, p, v)?,
        }
    }
    Ok(())
}

fn check_subcommand(spec: &VerbSpec, value: Option<&Value>) -> Result<(), VerbError> {
    let Some(v) = value else {
        return Err(VerbError::InvalidParams(format!(
            "verb `{}` requires `subcommand` (one of: {})",
            spec.name,
            spec.subcommands.join(", ")
        )));
    };
    let s = v.as_str().ok_or_else(|| {
        VerbError::InvalidParams(format!("`subcommand` for `{}` must be a string", spec.name))
    })?;
    if !spec.subcommands.iter().any(|c| c == s) {
        return Err(VerbError::InvalidParams(format!(
            "unknown subcommand `{s}` for verb `{}` (one of: {})",
            spec.name,
            spec.subcommands.join(", ")
        )));
    }
    Ok(())
}

fn check_value(spec: &VerbSpec, p: &super::spec::VerbParam, v: &Value) -> Result<(), VerbError> {
    let bad = |want: &str| {
        Err(VerbError::InvalidParams(format!(
            "parameter `{}` of verb `{}` must be {want}",
            p.name, spec.name
        )))
    };
    match p.kind {
        ParamKind::Flag => {
            if !v.is_boolean() {
                return bad("a boolean");
            }
        }
        ParamKind::Count => {
            let n = v.as_u64();
            if n.is_none() {
                return bad("a non-negative integer");
            }
        }
        ParamKind::Multi => {
            let Some(items) = v.as_array() else {
                return bad("an array of strings");
            };
            for item in items {
                let Some(s) = item.as_str() else {
                    return bad("an array of strings");
                };
                check_choice(spec, p, s)?;
            }
        }
        ParamKind::Value | ParamKind::Positional => {
            let Some(s) = v.as_str() else {
                return bad("a string");
            };
            check_choice(spec, p, s)?;
        }
    }
    Ok(())
}

fn check_choice(spec: &VerbSpec, p: &super::spec::VerbParam, s: &str) -> Result<(), VerbError> {
    if !p.choices.is_empty() && !p.choices.iter().any(|c| c == s) {
        return Err(VerbError::InvalidParams(format!(
            "parameter `{}` of verb `{}` must be one of: {} (got `{s}`)",
            p.name,
            spec.name,
            p.choices.join(", ")
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verb::derive::find;
    use serde_json::json;

    #[test]
    fn a_valid_params_object_passes() {
        let plan = find("plan").unwrap();
        assert!(check(plan, &json!({"file": "forjar.yaml", "json": true})).is_ok());
        assert!(check(plan, &json!({})).is_ok());
    }

    #[test]
    fn a_misspelled_parameter_is_rejected_not_ignored() {
        // The whole reason for additionalProperties:false. `stat_dir` silently
        // dropped would run `plan` against the default state directory and
        // report a confident, wrong answer.
        let plan = find("plan").unwrap();
        let e = check(plan, &json!({"stat_dir": "/tmp"})).unwrap_err();
        assert_eq!(e.kind(), "invalid_params");
        assert!(e.to_string().contains("stat_dir"), "{e}");
    }

    #[test]
    fn a_non_object_params_value_is_rejected() {
        let plan = find("plan").unwrap();
        for v in [json!("x"), json!(3), json!([]), json!(null)] {
            assert!(check(plan, &v).is_err(), "{v} must be rejected");
        }
    }

    #[test]
    fn wrong_types_are_rejected_per_kind() {
        let plan = find("plan").unwrap();
        // --json is a flag; a string is not a boolean.
        assert!(check(plan, &json!({"json": "true"})).is_err());
        // --file takes a value; a boolean is not a string.
        assert!(check(plan, &json!({"file": true})).is_err());
        // --what-if is Append; a bare string is not an array.
        assert!(check(plan, &json!({"what_if": "a=b"})).is_err());
        assert!(check(plan, &json!({"what_if": ["a=b"]})).is_ok());
    }

    #[test]
    fn closed_enums_reject_values_outside_the_domain() {
        let validate = find("validate").unwrap();
        assert!(check(validate, &json!({"min_purity": "pure"})).is_ok());
        let e = check(validate, &json!({"min_purity": "extremely-pure"})).unwrap_err();
        assert!(e.to_string().contains("must be one of"), "{e}");
    }

    #[test]
    fn grouped_verbs_require_a_known_subcommand() {
        let ws = find("workspace").unwrap();
        assert!(check(ws, &json!({})).is_err(), "missing subcommand");
        assert!(check(ws, &json!({"subcommand": "list"})).is_ok());
        let e = check(ws, &json!({"subcommand": "nope"})).unwrap_err();
        assert!(e.to_string().contains("unknown subcommand"), "{e}");
        assert!(check(ws, &json!({"subcommand": 1})).is_err());
    }

    #[test]
    fn leaf_verbs_reject_a_subcommand_key() {
        let plan = find("plan").unwrap();
        let e = check(plan, &json!({"subcommand": "list"})).unwrap_err();
        assert!(e.to_string().contains("takes no subcommand"), "{e}");
    }

    #[test]
    fn missing_required_parameters_are_named() {
        // Find any verb with a required parameter and confirm omission fails.
        let with_required = crate::verb::derive::registry()
            .iter()
            .find(|v| v.params.iter().any(|p| p.required) && !v.is_grouped());
        if let Some(v) = with_required {
            let name = v.params.iter().find(|p| p.required).unwrap().name.clone();
            let e = check(v, &json!({})).unwrap_err();
            assert!(e.to_string().contains(&name), "{e} should name {name}");
        }
    }

    #[test]
    fn count_parameters_take_non_negative_integers() {
        let spec = VerbSpec {
            name: "t".into(),
            description: "t".into(),
            params_schema: json!({}),
            output_schema: json!({}),
            effects: crate::verb::spec::Effects::ReadOnly,
            params: vec![super::super::spec::VerbParam {
                name: "n".into(),
                long: Some("n".into()),
                description: "n".into(),
                required: false,
                kind: ParamKind::Count,
                choices: vec![],
                default: None,
            }],
            subcommands: vec![],
        };
        assert!(check(&spec, &json!({"n": 3})).is_ok());
        assert!(check(&spec, &json!({"n": -1})).is_err());
        assert!(check(&spec, &json!({"n": "3"})).is_err());
    }

    #[test]
    fn every_registry_verb_accepts_an_empty_object_or_names_what_is_missing() {
        // Totality: validation never panics and never returns a non-actionable
        // error, for all ~159 verbs.
        for v in crate::verb::derive::registry() {
            match check(v, &json!({})) {
                Ok(()) => {}
                Err(e) => assert!(
                    e.to_string().contains(&v.name),
                    "{}: error must name the verb: {e}",
                    v.name
                ),
            }
        }
    }
}
