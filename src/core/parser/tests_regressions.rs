//! Regression pins for fixed GitHub issues (PMAT-076 / HYG-1).

use super::*;

// GH-88 regression: a NON-recipe resource whose depends_on points at a
// recipe-type resource must have that dep rewritten to the recipe's
// terminal expanded resource. Before the fix, only recipe-to-recipe deps
// were rewritten, leaving plain resources with dangling deps on recipe
// IDs that no longer exist after expansion.
#[test]
fn gh88_non_recipe_depends_on_recipe_rewritten_to_expanded() {
    let dir = tempfile::tempdir().unwrap();
    let recipes_dir = dir.path().join("recipes");
    std::fs::create_dir_all(&recipes_dir).unwrap();
    std::fs::write(
        recipes_dir.join("base-recipe.yaml"),
        r#"
recipe:
  name: base-recipe
resources:
  install:
    type: package
    provider: apt
    packages: [nginx]
  configure:
    type: file
    path: /etc/base.conf
    content: "configured"
    depends_on: [install]
"#,
    )
    .unwrap();

    let yaml = r#"
version: "1.0"
name: gh88-test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
resources:
  base:
    type: recipe
    machine: m1
    recipe: base-recipe
  app-conf:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "app"
    depends_on: [base]
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_recipes(&mut config, Some(dir.path())).unwrap();

    // Recipe expanded into namespaced resources; recipe ID 'base' is gone.
    assert!(config.resources.contains_key("base/install"));
    assert!(config.resources.contains_key("base/configure"));
    assert!(!config.resources.contains_key("base"));

    // The plain file resource's dep must be rewritten to the recipe's
    // terminal expanded resource — not left dangling on 'base'.
    let app_conf = &config.resources["app-conf"];
    assert!(
        !app_conf.depends_on.contains(&"base".to_string()),
        "dep on recipe ID must be rewritten, got {:?}",
        app_conf.depends_on
    );
    assert_eq!(
        app_conf.depends_on,
        vec!["base/configure".to_string()],
        "dep must point at the recipe's terminal expanded resource"
    );
}

// #335 regression: `lifecycle.ignore_drift` is a field list in the schema and
// a resource-wide off switch in the engine. Nothing validated the values, so
// `ignore_drift: [mode]` parsed clean and then disabled EVERY drift dimension
// for that resource. Narrowing the exemption widened it.

fn config_with_ignore_drift(entries: &[&str]) -> ForjarConfig {
    let list = entries
        .iter()
        .map(|e| format!("        - \"{e}\"\n"))
        .collect::<String>();
    parse_config(&format!(
        "version: \"1.0\"\nname: ignore-drift\nresources:\n  cfg:\n    type: file\n\
         \x20   machine: localhost\n    path: /etc/app.conf\n    content: hi\n\
         \x20   lifecycle:\n      ignore_drift:\n{list}"
    ))
    .expect("valid YAML")
}

/// forjar#360 re-based this from #335's "a narrowed `ignore_drift` is always
/// refused": a file resource's state query reports named fields, so `mode` is
/// now masked out of the observation and accepted.
#[test]
fn narrowed_ignore_drift_on_a_file_is_honoured() {
    let errors = validate_config(&config_with_ignore_drift(&["mode"]));
    assert!(errors.is_empty(), "expected no error, got {errors:?}");
}

/// The #335 refusal survives verbatim where it is still true: a resource type
/// whose state query reports no named fields cannot honour a narrowed list.
#[test]
fn narrowed_ignore_drift_without_a_field_vocabulary_is_a_validation_error() {
    let cfg = parse_config(
        "version: \"1.0\"\nname: ignore-drift\nresources:\n  tool:\n    type: package\n\
         \x20   machine: localhost\n    provider: apt\n    packages: [curl]\n\
         \x20   lifecycle:\n      ignore_drift:\n        - \"version\"\n",
    )
    .expect("valid YAML");
    let errors = validate_config(&cfg);
    assert_eq!(
        errors.len(),
        1,
        "expected exactly one error, got {errors:?}"
    );
    let msg = errors[0].message.clone();
    assert!(msg.contains("ignore_drift"), "{msg}");
    assert!(msg.contains("335"), "{msg}");
    assert!(msg.contains("version"), "{msg}");
}

#[test]
fn a_typo_in_ignore_drift_is_refused_not_treated_as_skip_all() {
    let errors = validate_config(&config_with_ignore_drift(&["modes"]));
    assert_eq!(
        errors.len(),
        1,
        "expected exactly one error, got {errors:?}"
    );
    assert!(errors[0].message.contains("modes"), "{}", errors[0].message);
}

#[test]
fn wildcard_ignore_drift_is_accepted() {
    let errors = validate_config(&config_with_ignore_drift(&["*"]));
    assert!(
        errors.is_empty(),
        "wildcard must stay legal, got {errors:?}"
    );
}
