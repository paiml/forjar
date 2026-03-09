//! FJ-202/3301/3405: Provider chain, shell provider, conditions falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-3301: Secret provider trait (env, file, exec providers)
//! - FJ-3301: Provider chain resolution (first-match, fallback)
//! - FJ-3405: Shell provider manifest loading and script validation
//! - FJ-3405: Shell provider listing and type parsing
//! - FJ-202: Conditional resource evaluation (when expressions)
//!
//! Usage: cargo test --test falsification_providers_conditions

use forjar::core::conditions::evaluate_when;
use forjar::core::secret_provider::{EnvProvider, ExecProvider, FileProvider, ProviderChain};
use forjar::core::shell_provider::{
    is_shell_type, list_shell_providers, load_manifest, parse_shell_type, validate_provider,
    validate_provider_script,
};
use forjar::core::types::Machine;
use std::collections::HashMap;

// ============================================================================
// FJ-3301: Environment Provider
// ============================================================================

#[test]
fn env_provider_resolves_path() {
    use forjar::core::secret_provider::SecretProvider;
    let provider = EnvProvider;
    let result = provider.resolve("PATH").unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().provider, "env");
}

#[test]
fn env_provider_missing_key() {
    use forjar::core::secret_provider::SecretProvider;
    let provider = EnvProvider;
    let result = provider.resolve("FORJAR_NONEXISTENT_KEY_999").unwrap();
    assert!(result.is_none());
}

#[test]
fn env_provider_name() {
    use forjar::core::secret_provider::SecretProvider;
    assert_eq!(EnvProvider.name(), "env");
}

// ============================================================================
// FJ-3301: File Provider
// ============================================================================

#[test]
fn file_provider_resolves() {
    use forjar::core::secret_provider::SecretProvider;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("db-password"), "s3cret\n").unwrap();

    let provider = FileProvider::new(dir.path());
    let result = provider.resolve("db-password").unwrap();
    assert!(result.is_some());
    let sv = result.unwrap();
    assert_eq!(sv.value, "s3cret"); // trimmed
    assert_eq!(sv.provider, "file");
}

#[test]
fn file_provider_missing() {
    use forjar::core::secret_provider::SecretProvider;
    let dir = tempfile::tempdir().unwrap();
    let provider = FileProvider::new(dir.path());
    let result = provider.resolve("nonexistent").unwrap();
    assert!(result.is_none());
}

#[test]
fn file_provider_name() {
    use forjar::core::secret_provider::SecretProvider;
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(FileProvider::new(dir.path()).name(), "file");
}

// ============================================================================
// FJ-3301: Exec Provider
// ============================================================================

#[test]
fn exec_provider_resolves() {
    use forjar::core::secret_provider::SecretProvider;
    let provider = ExecProvider::new("echo");
    let result = provider.resolve("hello").unwrap();
    assert!(result.is_some());
    assert!(result.unwrap().value.contains("hello"));
}

#[test]
fn exec_provider_failure_returns_none() {
    use forjar::core::secret_provider::SecretProvider;
    let provider = ExecProvider::new("false");
    let result = provider.resolve("key").unwrap();
    assert!(result.is_none());
}

#[test]
fn exec_provider_name() {
    use forjar::core::secret_provider::SecretProvider;
    assert_eq!(ExecProvider::new("echo").name(), "exec");
}

// ============================================================================
// FJ-3301: Provider Chain
// ============================================================================

#[test]
fn chain_first_match_wins() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("api-key"), "from-file").unwrap();

    let chain = ProviderChain::new()
        .with(Box::new(EnvProvider))
        .with(Box::new(FileProvider::new(dir.path())));

    let result = chain.resolve("api-key").unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().provider, "file");
}

#[test]
fn chain_empty_returns_none() {
    let chain = ProviderChain::new();
    let result = chain.resolve("anything").unwrap();
    assert!(result.is_none());
}

#[test]
fn chain_default_empty() {
    let chain = ProviderChain::default();
    assert!(chain.resolve("test").unwrap().is_none());
}

#[test]
fn chain_multiple_providers() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("secret-key"), "file-value").unwrap();

    let chain = ProviderChain::new()
        .with(Box::new(EnvProvider))
        .with(Box::new(FileProvider::new(dir.path())))
        .with(Box::new(ExecProvider::new("echo")));

    // File should resolve (env won't have SECRET_KEY)
    let result = chain.resolve("secret-key").unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().provider, "file");
}

// ============================================================================
// FJ-3405: Shell Provider Type Parsing
// ============================================================================

#[test]
fn parse_shell_type_valid() {
    assert_eq!(parse_shell_type("shell:nginx"), Some("nginx"));
    assert_eq!(parse_shell_type("shell:my-provider"), Some("my-provider"));
}

#[test]
fn parse_shell_type_invalid() {
    assert_eq!(parse_shell_type("plugin:foo"), None);
    assert_eq!(parse_shell_type("file"), None);
    assert_eq!(parse_shell_type(""), None);
}

#[test]
fn is_shell_type_check() {
    assert!(is_shell_type("shell:nginx"));
    assert!(!is_shell_type("plugin:nginx"));
    assert!(!is_shell_type("package"));
}

// ============================================================================
// FJ-3405: Shell Provider Script Validation
// ============================================================================

#[test]
fn validate_clean_provider_script() {
    let script = "#!/bin/bash\nset -euo pipefail\necho 'checking resource'\n";
    assert!(validate_provider_script(script).is_ok());
}

#[test]
fn validate_script_with_secret_leak() {
    let script = "#!/bin/bash\necho $PASSWORD\n";
    let result = validate_provider_script(script);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("secret leakage"));
}

// ============================================================================
// FJ-3405: Shell Provider Manifest
// ============================================================================

#[test]
fn load_manifest_success() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = r#"
name: test-provider
version: "0.1.0"
description: "A test shell provider"
check: check.sh
apply: apply.sh
destroy: destroy.sh
"#;
    std::fs::write(dir.path().join("provider.yaml"), yaml).unwrap();
    let loaded = load_manifest(dir.path()).unwrap();
    assert_eq!(loaded.name, "test-provider");
    assert_eq!(loaded.version, "0.1.0");
    assert_eq!(loaded.check, "check.sh");
    assert_eq!(loaded.apply, "apply.sh");
    assert_eq!(loaded.destroy, "destroy.sh");
}

#[test]
fn load_manifest_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    assert!(load_manifest(dir.path()).is_err());
}

// ============================================================================
// FJ-3405: Shell Provider Validation
// ============================================================================

#[test]
fn validate_provider_all_clean() {
    let dir = tempfile::tempdir().unwrap();
    let pdir = dir.path().join("my-provider");
    std::fs::create_dir_all(&pdir).unwrap();
    std::fs::write(
        pdir.join("provider.yaml"),
        "name: my-provider\nversion: \"0.1.0\"\ncheck: check.sh\napply: apply.sh\ndestroy: destroy.sh\n",
    ).unwrap();
    std::fs::write(pdir.join("check.sh"), "#!/bin/bash\nexit 0\n").unwrap();
    std::fs::write(pdir.join("apply.sh"), "#!/bin/bash\nexit 0\n").unwrap();
    std::fs::write(pdir.join("destroy.sh"), "#!/bin/bash\nexit 0\n").unwrap();

    let result = validate_provider(&pdir);
    assert!(result.validated, "errors: {:?}", result.errors);
    assert_eq!(result.name, "my-provider");
}

#[test]
fn validate_provider_with_leak() {
    let dir = tempfile::tempdir().unwrap();
    let pdir = dir.path().join("leaky");
    std::fs::create_dir_all(&pdir).unwrap();
    std::fs::write(
        pdir.join("provider.yaml"),
        "name: leaky\nversion: \"0.1.0\"\ncheck: check.sh\napply: apply.sh\ndestroy: destroy.sh\n",
    )
    .unwrap();
    std::fs::write(pdir.join("check.sh"), "#!/bin/bash\nexit 0\n").unwrap();
    std::fs::write(pdir.join("apply.sh"), "#!/bin/bash\necho $PASSWORD\n").unwrap();
    std::fs::write(pdir.join("destroy.sh"), "#!/bin/bash\nexit 0\n").unwrap();

    let result = validate_provider(&pdir);
    assert!(!result.validated);
    assert!(!result.errors.is_empty());
}

#[test]
fn validate_provider_missing_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let result = validate_provider(dir.path());
    assert!(!result.validated);
    assert!(!result.errors.is_empty());
}

// ============================================================================
// FJ-3405: List Shell Providers
// ============================================================================

#[test]
fn list_providers_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let providers = list_shell_providers(dir.path());
    assert!(providers.is_empty());
}

#[test]
fn list_providers_found() {
    let dir = tempfile::tempdir().unwrap();
    let p1 = dir.path().join("nginx");
    std::fs::create_dir_all(&p1).unwrap();
    std::fs::write(
        p1.join("provider.yaml"),
        "name: nginx\nversion: \"1\"\ncheck: c.sh\napply: a.sh\ndestroy: d.sh\n",
    )
    .unwrap();

    let p2 = dir.path().join("postgres");
    std::fs::create_dir_all(&p2).unwrap();
    std::fs::write(
        p2.join("provider.yaml"),
        "name: postgres\nversion: \"1\"\ncheck: c.sh\napply: a.sh\ndestroy: d.sh\n",
    )
    .unwrap();

    std::fs::create_dir_all(dir.path().join("not-a-provider")).unwrap();

    let providers = list_shell_providers(dir.path());
    assert_eq!(providers, vec!["nginx", "postgres"]);
}

// ============================================================================
// FJ-202: Conditional Evaluation (when expressions)
// ============================================================================

fn make_machine(arch: &str) -> Machine {
    Machine {
        hostname: "test-host".to_string(),
        addr: "10.0.0.1".to_string(),
        user: "root".to_string(),
        arch: arch.to_string(),
        ssh_key: None,
        roles: vec!["web".to_string(), "gpu".to_string()],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

fn make_params() -> HashMap<String, serde_yaml_ng::Value> {
    let mut params = HashMap::new();
    params.insert(
        "env".to_string(),
        serde_yaml_ng::Value::String("production".to_string()),
    );
    params.insert("feature_flag".to_string(), serde_yaml_ng::Value::Bool(true));
    params
}

#[test]
fn when_literal_true() {
    assert!(evaluate_when("true", &HashMap::new(), &make_machine("x86_64")).unwrap());
}

#[test]
fn when_literal_false() {
    assert!(!evaluate_when("false", &HashMap::new(), &make_machine("x86_64")).unwrap());
}

#[test]
fn when_literal_case_insensitive() {
    assert!(evaluate_when("TRUE", &HashMap::new(), &make_machine("x86_64")).unwrap());
    assert!(!evaluate_when("FALSE", &HashMap::new(), &make_machine("x86_64")).unwrap());
}

#[test]
fn when_arch_equals() {
    let m = make_machine("x86_64");
    assert!(evaluate_when(r#"{{machine.arch}} == "x86_64""#, &HashMap::new(), &m).unwrap());
}

#[test]
fn when_arch_not_equals() {
    let m = make_machine("aarch64");
    assert!(!evaluate_when(r#"{{machine.arch}} == "x86_64""#, &HashMap::new(), &m).unwrap());
}

#[test]
fn when_param_equals() {
    let m = make_machine("x86_64");
    let p = make_params();
    assert!(evaluate_when(r#"{{params.env}} == "production""#, &p, &m).unwrap());
}

#[test]
fn when_param_not_equals() {
    let m = make_machine("x86_64");
    let p = make_params();
    assert!(evaluate_when(r#"{{params.env}} != "staging""#, &p, &m).unwrap());
}

#[test]
fn when_roles_contains() {
    let m = make_machine("x86_64");
    assert!(evaluate_when(r#"{{machine.roles}} contains "gpu""#, &HashMap::new(), &m).unwrap());
}

#[test]
fn when_roles_not_contains() {
    let m = make_machine("x86_64");
    assert!(!evaluate_when(
        r#"{{machine.roles}} contains "storage""#,
        &HashMap::new(),
        &m
    )
    .unwrap());
}

#[test]
fn when_hostname_equals() {
    let m = make_machine("x86_64");
    assert!(evaluate_when(
        r#"{{machine.hostname}} == "test-host""#,
        &HashMap::new(),
        &m
    )
    .unwrap());
}

#[test]
fn when_addr_equals() {
    let m = make_machine("x86_64");
    assert!(evaluate_when(r#"{{machine.addr}} == "10.0.0.1""#, &HashMap::new(), &m).unwrap());
}

#[test]
fn when_user_equals() {
    let m = make_machine("x86_64");
    assert!(evaluate_when(r#"{{machine.user}} == "root""#, &HashMap::new(), &m).unwrap());
}

#[test]
fn when_unknown_param_error() {
    let m = make_machine("x86_64");
    let result = evaluate_when(r#"{{params.missing}} == "x""#, &HashMap::new(), &m);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown param"));
}

#[test]
fn when_unknown_machine_field_error() {
    let m = make_machine("x86_64");
    let result = evaluate_when(r#"{{machine.nonexistent}} == "x""#, &HashMap::new(), &m);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown machine field"));
}

#[test]
fn when_invalid_operator_error() {
    let m = make_machine("x86_64");
    let result = evaluate_when("something without operator", &HashMap::new(), &m);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid when expression"));
}

#[test]
fn when_unclosed_template_error() {
    let m = make_machine("x86_64");
    let result = evaluate_when(r#"{{machine.arch == "x""#, &HashMap::new(), &m);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unclosed template"));
}

#[test]
fn when_whitespace_trimmed() {
    assert!(evaluate_when("  true  ", &HashMap::new(), &make_machine("x86_64")).unwrap());
}

#[test]
fn when_single_quoted_values() {
    let m = make_machine("x86_64");
    assert!(evaluate_when("{{machine.arch}} == 'x86_64'", &HashMap::new(), &m).unwrap());
}

#[test]
fn when_bool_param_as_string() {
    let m = make_machine("x86_64");
    let p = make_params();
    assert!(evaluate_when(r#"{{params.feature_flag}} == "true""#, &p, &m).unwrap());
}
