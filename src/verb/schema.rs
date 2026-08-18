//! Derive JSON Schema from clap's own argument model.
//!
//! Every fact here comes from the [`clap::Arg`] that the CLI parses with, so a
//! schema cannot describe a parameter the CLI does not accept, nor omit one it
//! does.

use super::spec::{ParamKind, VerbParam};
use clap::{Arg, ArgAction};
use serde_json::{json, Map, Value};

/// Classify a clap argument by how it is spelled on the command line.
///
/// The action is authoritative and must be consulted *before* possible values:
/// clap reports `["true", "false"]` as the possible values of every `SetTrue`
/// flag, so a schema built from possible values alone turns every boolean flag
/// into a string enum.
#[must_use]
pub fn param_kind(arg: &Arg) -> ParamKind {
    if arg.is_positional() {
        return ParamKind::Positional;
    }
    match arg.get_action() {
        ArgAction::SetTrue | ArgAction::SetFalse => ParamKind::Flag,
        ArgAction::Count => ParamKind::Count,
        ArgAction::Append => ParamKind::Multi,
        _ => {
            // `Set` with a range that admits more than one value is multi-valued.
            let multi = arg
                .get_num_args()
                .is_some_and(|r| r.max_values() > 1 || r.min_values() > 1);
            if multi {
                ParamKind::Multi
            } else {
                ParamKind::Value
            }
        }
    }
}

/// The closed value set for a parameter, empty when the parameter is open.
///
/// Returns nothing for flags and counts: their `possible_values` are clap's
/// internal `true`/`false`, not a domain the caller chooses from.
#[must_use]
pub fn choices(arg: &Arg, kind: ParamKind) -> Vec<String> {
    if matches!(kind, ParamKind::Flag | ParamKind::Count) {
        return Vec::new();
    }
    arg.get_possible_values()
        .iter()
        .map(|p| p.get_name().to_string())
        .collect()
}

/// Build the [`VerbParam`] for one clap argument.
#[must_use]
pub fn param_of(arg: &Arg) -> VerbParam {
    let kind = param_kind(arg);
    VerbParam {
        name: arg.get_id().to_string(),
        long: arg.get_long().map(str::to_string),
        description: arg
            .get_help()
            .map(|h| h.to_string())
            .unwrap_or_else(|| arg.get_id().to_string()),
        required: arg.is_required_set(),
        kind,
        choices: choices(arg, kind),
        default: arg
            .get_default_values()
            .first()
            .map(|v| v.to_string_lossy().into_owned()),
    }
}

/// The JSON Schema fragment describing a single parameter's value.
#[must_use]
pub fn param_schema(p: &VerbParam) -> Value {
    let mut node = Map::new();
    node.insert("description".into(), json!(p.description));
    match p.kind {
        ParamKind::Flag => {
            node.insert("type".into(), json!("boolean"));
        }
        ParamKind::Count => {
            node.insert("type".into(), json!("integer"));
            node.insert("minimum".into(), json!(0));
        }
        ParamKind::Multi => {
            let mut items = Map::new();
            items.insert("type".into(), json!("string"));
            if !p.choices.is_empty() {
                items.insert("enum".into(), json!(p.choices));
            }
            node.insert("type".into(), json!("array"));
            node.insert("items".into(), Value::Object(items));
        }
        ParamKind::Value | ParamKind::Positional => {
            node.insert("type".into(), json!("string"));
            if !p.choices.is_empty() {
                node.insert("enum".into(), json!(p.choices));
            }
        }
    }
    if let Some(d) = &p.default {
        node.insert("default".into(), json!(d));
    }
    Value::Object(node)
}

/// The JSON Schema for a verb's whole params object.
///
/// `additionalProperties: false` is deliberate. An unknown key is a caller
/// error — usually a misspelled parameter — and silently dropping it would let
/// `{"stat_dir": "/tmp"}` run against the default state directory.
#[must_use]
pub fn params_schema(params: &[VerbParam], subcommands: &[String]) -> Value {
    let mut props = Map::new();
    let mut required: Vec<String> = Vec::new();

    if !subcommands.is_empty() {
        props.insert(
            "subcommand".into(),
            json!({
                "type": "string",
                "description": "Which nested operation to run.",
                "enum": subcommands,
            }),
        );
        required.push("subcommand".into());
    }

    for p in params {
        props.insert(p.name.clone(), param_schema(p));
        if p.required {
            required.push(p.name.clone());
        }
    }

    let mut schema = Map::new();
    schema.insert(
        "$schema".into(),
        json!("https://json-schema.org/draft/2020-12/schema"),
    );
    schema.insert("type".into(), json!("object"));
    schema.insert("additionalProperties".into(), json!(false));
    schema.insert("properties".into(), Value::Object(props));
    schema.insert("required".into(), json!(required));
    Value::Object(schema)
}

/// The result envelope every verb returns, on every transport.
///
/// # Why one envelope rather than 159 typed outputs
///
/// forjar's verbs emit human-readable text, and about a third of them accept
/// `--json` to emit a document instead. There is no typed output value in the
/// codebase to derive a per-verb schema *from*, so writing 159 of them would
/// mean inventing 159 new hand-maintained definitions — the exact thing this
/// registry exists to remove. The envelope is derived, total, and honest about
/// what a forjar verb actually produces.
#[must_use]
pub fn output_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "additionalProperties": false,
        "required": ["verb", "ok", "exit_code", "stdout", "stderr"],
        "properties": {
            "verb": { "type": "string", "description": "The verb that ran." },
            "ok": { "type": "boolean", "description": "True when exit_code is 0." },
            "exit_code": { "type": "integer", "description": "The process exit code." },
            "stdout": { "type": "string", "description": "Captured standard output." },
            "stderr": { "type": "string", "description": "Captured standard error." },
            "json": {
                "description": "stdout parsed as JSON, present only when it parses.",
            },
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Arg;

    #[test]
    fn set_true_flags_are_booleans_not_string_enums() {
        // The regression this pins: clap reports possible_values ["true","false"]
        // for every SetTrue flag. Reading possible values before the action turns
        // `--json` into {"type":"string","enum":["true","false"]}.
        let arg = Arg::new("json").long("json").action(ArgAction::SetTrue);
        let p = param_of(&arg);
        assert_eq!(p.kind, ParamKind::Flag);
        assert!(p.choices.is_empty(), "flags have no caller-facing choices");
        let s = param_schema(&p);
        assert_eq!(s["type"], "boolean");
        assert!(s.get("enum").is_none());
    }

    #[test]
    fn value_enums_keep_their_domain() {
        let arg = Arg::new("min_purity")
            .long("min-purity")
            .value_parser(["pure", "pinned", "impure"]);
        let p = param_of(&arg);
        assert_eq!(p.kind, ParamKind::Value);
        assert_eq!(p.choices, vec!["pure", "pinned", "impure"]);
        assert_eq!(
            param_schema(&p)["enum"],
            json!(["pure", "pinned", "impure"])
        );
    }

    #[test]
    fn append_args_become_arrays() {
        let arg = Arg::new("what_if")
            .long("what-if")
            .action(ArgAction::Append);
        let p = param_of(&arg);
        assert_eq!(p.kind, ParamKind::Multi);
        let s = param_schema(&p);
        assert_eq!(s["type"], "array");
        assert_eq!(s["items"]["type"], "string");
    }

    #[test]
    fn count_args_become_non_negative_integers() {
        let arg = Arg::new("verbose").long("verbose").action(ArgAction::Count);
        let p = param_of(&arg);
        assert_eq!(p.kind, ParamKind::Count);
        let s = param_schema(&p);
        assert_eq!(s["type"], "integer");
        assert_eq!(s["minimum"], 0);
    }

    #[test]
    fn positionals_are_detected() {
        let arg = Arg::new("path");
        assert_eq!(param_kind(&arg), ParamKind::Positional);
    }

    #[test]
    fn defaults_are_carried_into_the_schema() {
        let arg = Arg::new("file").long("file").default_value("forjar.yaml");
        let p = param_of(&arg);
        assert_eq!(p.default.as_deref(), Some("forjar.yaml"));
        assert_eq!(param_schema(&p)["default"], "forjar.yaml");
    }

    #[test]
    fn required_params_are_listed_and_unknown_keys_rejected() {
        let arg = Arg::new("name").long("name").required(true);
        let p = param_of(&arg);
        let s = params_schema(&[p], &[]);
        assert_eq!(s["required"], json!(["name"]));
        assert_eq!(s["additionalProperties"], json!(false));
    }

    #[test]
    fn grouped_verbs_require_a_subcommand_key() {
        let s = params_schema(&[], &["new".into(), "list".into()]);
        assert_eq!(s["required"], json!(["subcommand"]));
        assert_eq!(
            s["properties"]["subcommand"]["enum"],
            json!(["new", "list"])
        );
    }

    #[test]
    fn help_text_falls_back_to_the_id_never_empty() {
        let arg = Arg::new("undocumented").long("undocumented");
        assert_eq!(param_of(&arg).description, "undocumented");
    }

    #[test]
    fn output_envelope_declares_the_five_mandatory_fields() {
        let s = output_schema();
        assert_eq!(
            s["required"],
            json!(["verb", "ok", "exit_code", "stdout", "stderr"])
        );
    }
}
