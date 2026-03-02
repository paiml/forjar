//\! Recipe integration tests, expand recipes, complex configs.

use super::*;

#[test]
fn test_expand_recipes_missing_recipe_file() {
    let dir = tempfile::tempdir().unwrap();
    let recipes_dir = dir.path().join("recipes");
    std::fs::create_dir_all(&recipes_dir).unwrap();

    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  setup:
    type: recipe
    machine: m1
    recipe: nonexistent
"#;
    let mut config = parse_config(yaml).unwrap();
    let result = expand_recipes(&mut config, Some(dir.path()));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_expand_recipes_preserves_non_recipe_resources() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_recipes(&mut config, Some(dir.path())).unwrap();

    assert!(config.resources.contains_key("pkg"));
    assert_eq!(config.resources.len(), 1);
}

#[test]
fn test_expand_recipes_external_deps_propagated() {
    let dir = tempfile::tempdir().unwrap();
    let recipes_dir = dir.path().join("recipes");
    std::fs::create_dir_all(&recipes_dir).unwrap();
    std::fs::write(
        recipes_dir.join("dep-test.yaml"),
        r#"
recipe:
  name: dep-test
resources:
  first:
    type: package
    provider: apt
    packages: [nginx]
  second:
    type: file
    path: /etc/test
    content: test
    depends_on: [first]
"#,
    )
    .unwrap();

    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  base:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  my-recipe:
    type: recipe
    machine: m1
    recipe: dep-test
    depends_on:
      - base
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_recipes(&mut config, Some(dir.path())).unwrap();

    assert_eq!(config.resources.len(), 3);
    let first = &config.resources["my-recipe/first"];
    assert!(first.depends_on.contains(&"base".to_string()));

    let second = &config.resources["my-recipe/second"];
    assert!(second.depends_on.contains(&"my-recipe/first".to_string()));
    assert!(!second.depends_on.contains(&"base".to_string()));
}

// ---- Sub-recipe expansion tests ----

#[test]
fn test_fj_subrecipe_expansion() {
    let dir = tempfile::tempdir().unwrap();
    let recipes_dir = dir.path().join("recipes");
    std::fs::create_dir_all(&recipes_dir).unwrap();

    // Inner recipe: a leaf resource
    std::fs::write(
        recipes_dir.join("inner.yaml"),
        r#"
recipe:
  name: inner
resources:
  leaf:
    type: file
    path: /etc/inner.conf
    content: "inner-content"
"#,
    )
    .unwrap();

    // Outer recipe: contains a sub-recipe reference
    std::fs::write(
        recipes_dir.join("outer.yaml"),
        r#"
recipe:
  name: outer
resources:
  setup:
    type: file
    path: /etc/outer.conf
    content: "outer-content"
  nested:
    type: recipe
    recipe: inner
"#,
    )
    .unwrap();

    let yaml = r#"
version: "1.0"
name: sub-recipe-test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
resources:
  top:
    type: recipe
    machine: m1
    recipe: outer
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_recipes(&mut config, Some(dir.path())).unwrap();

    // outer expanded: top/setup (file), top/nested expanded to top/nested/leaf (file)
    assert!(
        config.resources.contains_key("top/setup"),
        "outer file resource should be namespaced as top/setup"
    );
    assert!(
        config.resources.contains_key("top/nested/leaf"),
        "inner leaf should be namespaced as top/nested/leaf"
    );
    // No recipe resources should remain
    assert!(
        !config
            .resources
            .values()
            .any(|r| r.resource_type == ResourceType::Recipe),
        "no recipe resources should remain after expansion"
    );
}

#[test]
fn test_fj_subrecipe_cycle_detection() {
    let dir = tempfile::tempdir().unwrap();
    let recipes_dir = dir.path().join("recipes");
    std::fs::create_dir_all(&recipes_dir).unwrap();

    // Recipe A references recipe B
    std::fs::write(
        recipes_dir.join("cycle-a.yaml"),
        r#"
recipe:
  name: cycle-a
resources:
  step:
    type: recipe
    recipe: cycle-b
"#,
    )
    .unwrap();

    // Recipe B references recipe A
    std::fs::write(
        recipes_dir.join("cycle-b.yaml"),
        r#"
recipe:
  name: cycle-b
resources:
  step:
    type: recipe
    recipe: cycle-a
"#,
    )
    .unwrap();

    let yaml = r#"
version: "1.0"
name: cycle-test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
resources:
  root:
    type: recipe
    machine: m1
    recipe: cycle-a
"#;
    let mut config = parse_config(yaml).unwrap();
    let result = expand_recipes(&mut config, Some(dir.path()));
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("cycle"),
        "error should mention cycle"
    );
}

#[test]
fn test_fj_subrecipe_depth_limit() {
    let dir = tempfile::tempdir().unwrap();
    let recipes_dir = dir.path().join("recipes");
    std::fs::create_dir_all(&recipes_dir).unwrap();

    // Self-referencing recipe: always produces a new recipe resource
    std::fs::write(
        recipes_dir.join("self-ref.yaml"),
        r#"
recipe:
  name: self-ref
resources:
  again:
    type: recipe
    recipe: self-ref
"#,
    )
    .unwrap();

    let yaml = r#"
version: "1.0"
name: depth-test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
resources:
  start:
    type: recipe
    machine: m1
    recipe: self-ref
"#;
    let mut config = parse_config(yaml).unwrap();
    let result = expand_recipes(&mut config, Some(dir.path()));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("cycle") || err.contains("max depth"),
        "error should mention cycle or max depth, got: {}",
        err
    );
}

#[test]
fn test_fj_subrecipe_depends_on_threading() {
    let dir = tempfile::tempdir().unwrap();
    let recipes_dir = dir.path().join("recipes");
    std::fs::create_dir_all(&recipes_dir).unwrap();

    // Inner recipe with a file resource
    std::fs::write(
        recipes_dir.join("inner-dep.yaml"),
        r#"
recipe:
  name: inner-dep
resources:
  cfg:
    type: file
    path: /etc/inner.conf
    content: "hello"
"#,
    )
    .unwrap();

    // Outer recipe: file + sub-recipe with depends_on
    std::fs::write(
        recipes_dir.join("outer-dep.yaml"),
        r#"
recipe:
  name: outer-dep
resources:
  setup:
    type: file
    path: /etc/outer.conf
    content: "setup"
  nested:
    type: recipe
    recipe: inner-dep
    depends_on:
      - setup
"#,
    )
    .unwrap();

    let yaml = r#"
version: "1.0"
name: dep-thread-test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
resources:
  base:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  app:
    type: recipe
    machine: m1
    recipe: outer-dep
    depends_on:
      - base
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_recipes(&mut config, Some(dir.path())).unwrap();

    // base should remain
    assert!(config.resources.contains_key("base"));

    // outer's first resource should get the external dep
    let setup = &config.resources["app/setup"];
    assert!(
        setup.depends_on.contains(&"base".to_string()),
        "first resource of outer recipe should inherit external depends_on"
    );

    // inner's resource should be fully namespaced
    assert!(
        config.resources.contains_key("app/nested/cfg"),
        "inner resource should be at app/nested/cfg"
    );

    // No recipe resources remain
    assert!(
        !config
            .resources
            .values()
            .any(|r| r.resource_type == ResourceType::Recipe),
        "no recipe resources should remain"
    );
}

#[test]
fn test_fj_recipe_to_recipe_depends_on_resolves_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let recipes_dir = dir.path().join("recipes");
    std::fs::create_dir_all(&recipes_dir).unwrap();

    // Recipe A: two resources, second depends on first
    std::fs::write(
        recipes_dir.join("recipe-a.yaml"),
        r#"
recipe:
  name: recipe-a
resources:
  step1:
    type: package
    provider: apt
    packages: [curl]
  step2:
    type: file
    path: /etc/a.conf
    content: "done"
    depends_on: [step1]
"#,
    )
    .unwrap();

    // Recipe B: one resource
    std::fs::write(
        recipes_dir.join("recipe-b.yaml"),
        r#"
recipe:
  name: recipe-b
resources:
  only:
    type: file
    path: /etc/b.conf
    content: "b-content"
"#,
    )
    .unwrap();

    // Stack: recipe-b depends_on recipe-a (recipe-to-recipe dep)
    let yaml = r#"
version: "1.0"
name: recipe-dep-test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
resources:
  first:
    type: recipe
    machine: m1
    recipe: recipe-a
  second:
    type: recipe
    machine: m1
    recipe: recipe-b
    depends_on: [first]
"#;
    let mut config = parse_config(yaml).unwrap();
    expand_recipes(&mut config, Some(dir.path())).unwrap();

    // recipe-a expands to first/step1, first/step2
    // recipe-b expands to second/only
    // second/only should depend on first/step2 (terminal of recipe-a)
    assert!(config.resources.contains_key("first/step1"));
    assert!(config.resources.contains_key("first/step2"));
    assert!(config.resources.contains_key("second/only"));

    let second_only = &config.resources["second/only"];
    assert!(
        second_only.depends_on.contains(&"first/step2".to_string()),
        "recipe-to-recipe dep should resolve to terminal resource: got {:?}",
        second_only.depends_on
    );
}
