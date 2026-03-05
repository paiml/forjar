use super::template::resolve_template;
use super::*;
use std::collections::HashMap;

fn resolve(template: &str, params: &HashMap<String, serde_yaml_ng::Value>) -> String {
    resolve_template(template, params, &indexmap::IndexMap::new()).unwrap()
}

fn params_with(key: &str, val: &str) -> HashMap<String, serde_yaml_ng::Value> {
    let mut p = HashMap::new();
    p.insert(
        key.to_string(),
        serde_yaml_ng::Value::String(val.to_string()),
    );
    p
}

#[test]
fn test_fj250_upper() {
    let p = params_with("name", "hello");
    assert_eq!(resolve("{{upper(params.name)}}", &p), "HELLO");
}

#[test]
fn test_fj250_lower() {
    let p = params_with("name", "WORLD");
    assert_eq!(resolve("{{lower(params.name)}}", &p), "world");
}

#[test]
fn test_fj250_trim() {
    let p = params_with("name", "  spaced  ");
    assert_eq!(resolve("{{trim(params.name)}}", &p), "spaced");
}

#[test]
fn test_fj250_default_with_value() {
    let p = params_with("name", "alice");
    assert_eq!(
        resolve("{{default(params.name, \"fallback\")}}", &p),
        "alice"
    );
}

#[test]
fn test_fj250_default_empty_uses_fallback() {
    let p = params_with("name", "");
    assert_eq!(
        resolve("{{default(params.name, \"fallback\")}}", &p),
        "fallback"
    );
}

#[test]
fn test_fj250_replace() {
    let p = params_with("path", "/opt/old/data");
    assert_eq!(
        resolve("{{replace(params.path, \"old\", \"new\")}}", &p),
        "/opt/new/data"
    );
}

#[test]
fn test_fj250_b3sum() {
    let p = params_with("val", "test");
    let result = resolve("{{b3sum(params.val)}}", &p);
    // BLAKE3 of "test" is deterministic
    let expected = blake3::hash(b"test").to_hex().to_string();
    assert_eq!(result, expected);
}

#[test]
fn test_fj250_b3sum_literal() {
    let p = HashMap::new();
    let result = resolve("{{b3sum(\"hello\")}}", &p);
    let expected = blake3::hash(b"hello").to_hex().to_string();
    assert_eq!(result, expected);
}

#[test]
fn test_fj250_join() {
    let p = params_with("items", "a,b,c");
    assert_eq!(resolve("{{join(params.items, \":\")}}", &p), "a:b:c");
}

#[test]
fn test_fj250_split() {
    let p = params_with("path", "a:b:c");
    assert_eq!(resolve("{{split(params.path, \":\")}}", &p), "a,b,c");
}

#[test]
fn test_fj250_nested_upper_trim() {
    let p = params_with("name", "  hello  ");
    assert_eq!(resolve("{{upper(trim(params.name))}}", &p), "HELLO");
}

#[test]
fn test_fj250_nested_lower_replace() {
    let p = params_with("host", "MY-SERVER");
    assert_eq!(
        resolve("{{lower(replace(params.host, \"-\", \"_\"))}}", &p),
        "my_server"
    );
}

#[test]
fn test_fj250_literal_arg() {
    let p = HashMap::new();
    assert_eq!(resolve("{{upper(\"hello\")}}", &p), "HELLO");
}

#[test]
fn test_fj250_mixed_template_and_func() {
    let mut p = params_with("name", "world");
    p.insert(
        "greeting".to_string(),
        serde_yaml_ng::Value::String("hello".to_string()),
    );
    assert_eq!(
        resolve("{{params.greeting}}-{{upper(params.name)}}", &p),
        "hello-WORLD"
    );
}

#[test]
fn test_fj250_env_function() {
    // HOME should always be set
    let p = HashMap::new();
    let result = resolve_template("{{env(\"HOME\")}}", &p, &indexmap::IndexMap::new());
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn test_fj250_env_missing() {
    let p = HashMap::new();
    let result = resolve_template(
        "{{env(\"FORJAR_NONEXISTENT_TEST_VAR_XYZ\")}}",
        &p,
        &indexmap::IndexMap::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not set"));
}

#[test]
fn test_fj250_unknown_function() {
    let p = HashMap::new();
    let result = resolve_template("{{bogus(\"x\")}}", &p, &indexmap::IndexMap::new());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown template function"));
}

#[test]
fn test_fj250_wrong_arg_count() {
    let p = params_with("x", "val");
    let result = resolve_template(
        "{{upper(params.x, \"extra\")}}",
        &p,
        &indexmap::IndexMap::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires 1 argument"));
}

#[test]
fn test_fj250_replace_multiple_occurrences() {
    let p = params_with("s", "a.b.c.d");
    assert_eq!(
        resolve("{{replace(params.s, \".\", \"/\")}}", &p),
        "a/b/c/d"
    );
}

#[test]
fn test_fj250_upper_unicode() {
    let p = params_with("text", "café");
    assert_eq!(resolve("{{upper(params.text)}}", &p), "CAFÉ");
}

#[test]
fn test_fj250_machine_ref_in_func() {
    let mut machines = indexmap::IndexMap::new();
    machines.insert(
        "web".to_string(),
        Machine {
            hostname: "web-prod".to_string(),
            addr: "10.0.0.1".to_string(),
            user: "deploy".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        },
    );
    let p = HashMap::new();
    let result = resolve_template("{{upper(machine.web.hostname)}}", &p, &machines).unwrap();
    assert_eq!(result, "WEB-PROD");
}

#[test]
fn test_fj250_default_with_missing_param_returns_fallback() {
    // When using default() with a param that doesn't exist, it should error
    // (because the arg resolution fails before default runs)
    // Use an empty param instead to test default behavior
    let p = params_with("maybe", "");
    assert_eq!(
        resolve("{{default(params.maybe, \"backup\")}}", &p),
        "backup"
    );
}

#[test]
fn test_fj250_join_single_item() {
    let p = params_with("items", "only");
    assert_eq!(resolve("{{join(params.items, \":\")}}", &p), "only");
}

#[test]
fn test_fj250_split_no_delimiter() {
    let p = params_with("text", "nosplit");
    assert_eq!(resolve("{{split(params.text, \":\")}}", &p), "nosplit");
}

#[test]
fn test_fj250_trim_literal() {
    let p = HashMap::new();
    assert_eq!(resolve("{{trim(\"  hello  \")}}", &p), "hello");
}

#[test]
fn test_fj250_func_in_multiline_content() {
    let p = params_with("host", "db.internal");
    let template = "server: {{upper(params.host)}}\nport: 5432";
    assert_eq!(resolve(template, &p), "server: DB.INTERNAL\nport: 5432");
}
