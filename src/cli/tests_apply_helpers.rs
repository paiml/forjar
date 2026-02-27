//! Tests: Apply lifecycle helpers (hooks, notify, params, git).

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::apply_helpers::*;
use super::test_fixtures::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj054_run_hook_success() {
        run_hook("test", "echo hello", false).unwrap();
    }


    #[test]
    fn test_fj054_run_hook_failure() {
        let result = run_hook("test", "exit 1", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("failed"));
    }


    #[test]
    fn test_fj054_run_hook_nonzero_exit() {
        let result = run_hook("pre_apply", "exit 42", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exit 42"));
    }


    #[test]
    fn test_fj132_apply_param_overrides_basic() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#;
        let mut config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let overrides = vec!["env=production".to_string(), "port=8080".to_string()];
        apply_param_overrides(&mut config, &overrides).unwrap();
        assert_eq!(
            config.params.get("env").unwrap(),
            &serde_yaml_ng::Value::String("production".to_string())
        );
        assert_eq!(
            config.params.get("port").unwrap(),
            &serde_yaml_ng::Value::String("8080".to_string())
        );
    }


    #[test]
    fn test_fj132_apply_param_overrides_invalid() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#;
        let mut config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let overrides = vec!["no-equals-sign".to_string()];
        let result = apply_param_overrides(&mut config, &overrides);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected KEY=VALUE"));
    }


    #[test]
    fn test_fj132_run_hook_success() {
        let result = run_hook("test", "true", false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj132_run_hook_failure() {
        let result = run_hook("test", "false", false);
        assert!(result.is_err());
    }


    #[test]
    fn test_fj132_apply_param_overrides_with_equals_in_value() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#;
        let mut config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let overrides = vec!["conn=host=db port=5432".to_string()];
        apply_param_overrides(&mut config, &overrides).unwrap();
        // split_once only splits on first =, so value contains "host=db port=5432"
        assert_eq!(
            config.params.get("conn").unwrap(),
            &serde_yaml_ng::Value::String("host=db port=5432".to_string())
        );
    }

    // ── FJ-036 tests ────────────────────────────────────────────


    #[test]
    fn test_fj211_load_env_params_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_env_config(dir.path());
        let env = dir.path().join("prod.env.yaml");
        std::fs::write(&env, "data_dir: /prod/data\nlog_level: warn\n").unwrap();

        let mut config = parse_and_validate(&file).unwrap();
        load_env_params(&mut config, &env).unwrap();

        assert_eq!(
            config.params.get("data_dir").unwrap(),
            &serde_yaml_ng::Value::String("/prod/data".to_string())
        );
        assert_eq!(
            config.params.get("log_level").unwrap(),
            &serde_yaml_ng::Value::String("warn".to_string())
        );
    }


    #[test]
    fn test_fj211_load_env_params_partial_override() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_env_config(dir.path());
        let env = dir.path().join("staging.env.yaml");
        std::fs::write(&env, "log_level: debug\n").unwrap();

        let mut config = parse_and_validate(&file).unwrap();
        load_env_params(&mut config, &env).unwrap();

        // data_dir retains default from config
        assert_eq!(
            config.params.get("data_dir").unwrap(),
            &serde_yaml_ng::Value::String("/default/data".to_string())
        );
        // log_level overridden from env
        assert_eq!(
            config.params.get("log_level").unwrap(),
            &serde_yaml_ng::Value::String("debug".to_string())
        );
    }


    #[test]
    fn test_fj211_load_env_params_adds_new_keys() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_env_config(dir.path());
        let env = dir.path().join("extra.env.yaml");
        std::fs::write(&env, "new_param: hello\n").unwrap();

        let mut config = parse_and_validate(&file).unwrap();
        load_env_params(&mut config, &env).unwrap();

        assert_eq!(
            config.params.get("new_param").unwrap(),
            &serde_yaml_ng::Value::String("hello".to_string())
        );
    }


    #[test]
    fn test_fj211_load_env_params_file_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_env_config(dir.path());
        let env = dir.path().join("missing.yaml");

        let mut config = parse_and_validate(&file).unwrap();
        let result = load_env_params(&mut config, &env);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot read env file"));
    }


    #[test]
    fn test_fj211_load_env_params_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_env_config(dir.path());
        let env = dir.path().join("bad.yaml");
        std::fs::write(&env, "[ not a mapping ]").unwrap();

        let mut config = parse_and_validate(&file).unwrap();
        let result = load_env_params(&mut config, &env);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid YAML in env file"));
    }


    #[test]
    fn test_fj225_notify_config_parse() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m1
    path: /tmp/fj225.txt
    content: "hello"
policy:
  notify:
    on_success: "echo success {{machine}} {{converged}}"
    on_failure: "echo failure {{machine}} {{failed}}"
    on_drift: "echo drift {{machine}} {{drift_count}}"
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(
            config.policy.notify.on_success.as_deref(),
            Some("echo success {{machine}} {{converged}}")
        );
        assert_eq!(
            config.policy.notify.on_failure.as_deref(),
            Some("echo failure {{machine}} {{failed}}")
        );
        assert_eq!(
            config.policy.notify.on_drift.as_deref(),
            Some("echo drift {{machine}} {{drift_count}}")
        );
    }


    #[test]
    fn test_fj225_run_notify_template_expansion() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("notify-template.txt");
        run_notify(
            &format!(
                "echo '{{{{machine}}}}:{{{{converged}}}}:{{{{failed}}}}' > {}",
                marker.display()
            ),
            &[("machine", "web01"), ("converged", "5"), ("failed", "0")],
        );
        assert!(marker.exists());
        let content = std::fs::read_to_string(&marker).unwrap();
        assert!(content.contains("web01:5:0"), "content: {}", content);
    }


    #[test]
    fn test_fj225_run_notify_failure_silent() {
        // A failing notify hook should not panic
        run_notify("exit 1", &[]);
    }

}
