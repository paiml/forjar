//! Tests: Platform dispatch — registry, catalog, multi-config,
//! stack graph, preservation, SBOM with models/files, signing.

use super::commands::*;
use super::dispatch_platform::*;
use std::path::PathBuf;

fn make_config(dir: &std::path::Path, yaml: &str) -> PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

#[test]
fn dispatch_registry_list_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = Commands::RegistryList(RegistryListArgs {
        registry_dir: Some(dir.path().to_path_buf()),
        json: false,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_registry_list_json() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = Commands::RegistryList(RegistryListArgs {
        registry_dir: Some(dir.path().to_path_buf()),
        json: true,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_catalog_list_empty() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = Commands::CatalogList(CatalogListArgs {
        catalog_dir: Some(dir.path().to_path_buf()),
        category: None,
        json: false,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_catalog_list_json_with_category() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = Commands::CatalogList(CatalogListArgs {
        catalog_dir: Some(dir.path().to_path_buf()),
        category: Some("web".to_string()),
        json: true,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_agent_registry_empty() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = Commands::AgentRegistry(AgentRegistryArgs {
        registry_dir: Some(dir.path().to_path_buf()),
        category: None,
        json: false,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_agent_registry_json() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = Commands::AgentRegistry(AgentRegistryArgs {
        registry_dir: Some(dir.path().to_path_buf()),
        category: Some("inference".to_string()),
        json: true,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_state_backend_empty() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let cmd = Commands::StateBackend(StateBackendArgs {
        state_dir,
        prefix: None,
        json: false,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_state_backend_json_with_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let cmd = Commands::StateBackend(StateBackendArgs {
        state_dir,
        prefix: Some("prod".to_string()),
        json: true,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_preservation_check() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        r#"version: "1.0"
name: pres-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#,
    );
    let cmd = Commands::Preservation(PreservationArgs {
        file,
        json: false,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_preservation_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        r#"version: "1.0"
name: pres-json
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#,
    );
    let cmd = Commands::Preservation(PreservationArgs { file, json: true });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_recipe_sign_no_verify() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(
        &recipe,
        r#"recipe:
  name: test
  version: "1.0"
resources:
  f:
    type: file
    path: /tmp/test
    content: hello
"#,
    )
    .unwrap();
    // Sign mode (not verify) — generates a signature
    let cmd = Commands::RecipeSign(RecipeSignArgs {
        recipe,
        verify: false,
        signer: None,
        json: false,
        pq: false,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_recipe_sign_verify_missing_sig() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "recipe:\n  name: test\nresources: {}\n").unwrap();
    let cmd = Commands::RecipeSign(RecipeSignArgs {
        recipe,
        verify: true,
        signer: None,
        json: false,
        pq: false,
    });
    // Verify fails when no signature exists
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_err());
}

#[test]
fn dispatch_saga_plan() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    // Create two stack files
    let stack_a = dir.path().join("stack-a.yaml");
    std::fs::write(
        &stack_a,
        "version: \"1.0\"\nname: stack-a\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m\n    provider: apt\n    packages: [curl]\n",
    ).unwrap();
    let cmd = Commands::Saga(SagaArgs {
        file: vec![stack_a],
        state_dir,
        json: false,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_multi_apply() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        r#"version: "1.0"
name: multi
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#,
    );
    let cmd = Commands::MultiApply(MultiConfigArgs {
        file: vec![file],
        json: false,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_multi_apply_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        "version: \"1.0\"\nname: multi-j\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
    );
    let cmd = Commands::MultiApply(MultiConfigArgs {
        file: vec![file],
        json: true,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_stack_graph() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        r#"version: "1.0"
name: graph
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  a:
    type: file
    machine: m
    path: /tmp/a
    content: a
  b:
    type: file
    machine: m
    path: /tmp/b
    content: b
    depends_on: [a]
"#,
    );
    let cmd = Commands::StackGraph(StackGraphArgs {
        file: vec![file],
        json: false,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_stack_graph_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        "version: \"1.0\"\nname: sg-j\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
    );
    let cmd = Commands::StackGraph(StackGraphArgs {
        file: vec![file],
        json: true,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_infra_query() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = make_config(
        dir.path(),
        r#"version: "1.0"
name: query
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [vim]
"#,
    );
    let cmd = Commands::InfraQuery(InfraQueryArgs {
        file,
        state_dir,
        pattern: Some("pkg".to_string()),
        resource_type: None,
        machine: None,
        tag: None,
        details: false,
        live: false,
        json: false,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_infra_query_json_with_filters() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = make_config(
        dir.path(),
        "version: \"1.0\"\nname: q-j\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
    );
    let cmd = Commands::InfraQuery(InfraQueryArgs {
        file,
        state_dir,
        pattern: None,
        resource_type: Some("file".to_string()),
        machine: Some("m".to_string()),
        tag: Some("web".to_string()),
        details: true,
        live: false,
        json: true,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_parallel_apply() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        "version: \"1.0\"\nname: par\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
    );
    let cmd = Commands::ParallelApply(ParallelStackArgs {
        file: vec![file],
        max_parallel: 2,
        json: false,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_recipe_sign_pq() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "recipe:\n  name: pq-test\nresources: {}\n").unwrap();
    let cmd = Commands::RecipeSign(RecipeSignArgs {
        recipe,
        verify: false,
        signer: Some("ci".to_string()),
        json: true,
        pq: true,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_pull_agent_push_mode() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = make_config(
        dir.path(),
        "version: \"1.0\"\nname: pull\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
    );
    let cmd = Commands::PullAgent(PullAgentArgs {
        file,
        state_dir,
        interval: 1,
        auto_apply: false,
        max_iterations: Some(1),
        pull: false,
        json: false,
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_ok());
}

#[test]
fn dispatch_unknown_command_returns_error() {
    let cmd = Commands::Init(InitArgs {
        path: PathBuf::from("/tmp/test-init"),
    });
    let result = dispatch_platform_cmd(cmd);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown"));
}
