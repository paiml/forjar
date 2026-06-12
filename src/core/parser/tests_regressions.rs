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
