//! Refs #211 / #213: `template` must expand, or say why it cannot.
//!
//! On the published 1.12.3 every assertion below that is marked RED passed
//! trivially because `template` printed its input back unchanged and exited 0.

use super::*;

fn write(dir: &tempfile::TempDir, name: &str, body: &str) -> std::path::PathBuf {
    let p = dir.path().join(name);
    std::fs::write(&p, body).unwrap();
    p
}

const RECIPE: &str = r#"
recipe:
  name: demo
  inputs:
    greeting:
      type: string
      default: hi
resources:
  msg:
    type: file
    path: /tmp/msg.txt
    content: "{{inputs.greeting}} world"
"#;

const CONFIG: &str = r#"
version: '1.0'
name: cfg
params:
  root: /srv/app
machines:
  local:
    hostname: local
    addr: 127.0.0.1
    user: nobody
    arch: x86_64
resources:
  hello:
    type: file
    machine: local
    path: "{{params.root}}/hello.txt"
    content: hi
"#;

// ── recipes ───────────────────────────────────────────────────────────

#[test]
fn recipe_default_is_applied_without_any_var() {
    let d = tempfile::tempdir().unwrap();
    let p = write(&d, "recipe.yaml", RECIPE);
    let out = expand_for_test(&p, &[]).expect("expand");
    // RED on 1.12.3: the output still contained "{{inputs.greeting}}".
    assert!(!out.contains("{{inputs."), "left unexpanded: {out}");
    assert!(out.contains("hi world"), "default not applied: {out}");
}

#[test]
fn var_overrides_the_default() {
    let d = tempfile::tempdir().unwrap();
    let p = write(&d, "recipe.yaml", RECIPE);
    let out = expand_for_test(&p, &["greeting=HELLO".to_string()]).expect("expand");
    assert!(out.contains("HELLO world"), "override not applied: {out}");
}

#[test]
fn undeclared_input_is_rejected() {
    let d = tempfile::tempdir().unwrap();
    let p = write(&d, "recipe.yaml", RECIPE);
    let err = expand_for_test(&p, &["nosuch=1".to_string()]).unwrap_err();
    assert!(!err.is_empty(), "an undeclared input must be named");
}

// ── configs ───────────────────────────────────────────────────────────

#[test]
fn config_params_are_resolved_like_show_resolves_them() {
    let d = tempfile::tempdir().unwrap();
    let p = write(&d, "forjar.yaml", CONFIG);
    let out = expand_for_test(&p, &[]).expect("expand");
    // RED on 1.12.3: `template forjar.yaml` was the source file byte-for-byte.
    assert!(!out.contains("{{params."), "left unexpanded: {out}");
    assert!(out.contains("/srv/app/hello.txt"), "not resolved: {out}");
}

#[test]
fn var_overrides_a_config_param() {
    let d = tempfile::tempdir().unwrap();
    let p = write(&d, "forjar.yaml", CONFIG);
    let out = expand_for_test(&p, &["root=/opt/x".to_string()]).expect("expand");
    assert!(out.contains("/opt/x/hello.txt"), "-V ignored: {out}");
    assert!(!out.contains("/srv/app/hello.txt"), "-V not applied: {out}");
}

#[test]
fn unknown_variable_is_an_error_not_a_passthrough() {
    let d = tempfile::tempdir().unwrap();
    let body = CONFIG.replace("{{params.root}}", "{{greeting}}");
    let p = write(&d, "forjar.yaml", &body);
    let err = expand_for_test(&p, &["greeting=HELLO".to_string()]).unwrap_err();
    assert!(
        err.contains("greeting"),
        "the unresolvable variable must be named: {err}"
    );
}

// ── input rejection ───────────────────────────────────────────────────

#[test]
fn var_without_equals_is_refused() {
    assert!(parse_vars(&["novaluehere".to_string()]).is_err());
    assert!(parse_vars(&["=novalue".to_string()]).is_err());
    assert!(parse_vars(&["k=v".to_string()]).is_ok());
    // An empty value is legitimate — it is an assignment.
    assert!(parse_vars(&["k=".to_string()]).is_ok());
}

#[test]
fn clap_value_parser_refuses_the_same_input() {
    use crate::cli::commands::parse_var_assignment;
    assert!(parse_var_assignment("novaluehere").is_err());
    assert!(parse_var_assignment("k=v").is_ok());
}

#[test]
fn a_file_that_is_neither_recipe_nor_config_is_refused() {
    let d = tempfile::tempdir().unwrap();
    let p = write(&d, "junk.yaml", "just: a mapping\n");
    let err = cmd_template(&p, &[], false).unwrap_err();
    assert!(err.contains("neither a recipe"), "unexpected error: {err}");
}

// ── helper: exercise the expansion without capturing stdout ───────────

fn expand_for_test(path: &std::path::Path, vars: &[String]) -> Result<String, String> {
    let content = std::fs::read_to_string(path).unwrap();
    let map = parse_vars(vars)?;
    match classify(&content) {
        Kind::Recipe => expand_recipe_file(&content, &map),
        Kind::Config => expand_config_file(&content, &map),
        Kind::Unknown => Err("unknown".to_string()),
    }
}
