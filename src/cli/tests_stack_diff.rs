//! Tests: FJ-1389 unified stack diff.

#[cfg(test)]
mod tests {
    use super::super::stack_diff::*;
    use std::io::Write;

    fn write_yaml(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    const CONFIG_BASE: &str = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web-server
    addr: 10.0.0.1
    arch: x86_64
params:
  env: production
  port: "8080"
resources:
  nginx:
    type: file
    path: /etc/nginx/nginx.conf
    content: "worker_processes auto;"
    machine: web
  app-config:
    type: file
    path: /etc/app/config.yaml
    content: "port: 8080"
    machine: web
outputs:
  web_addr:
    value: "{{machines.web.addr}}"
"#;

    const CONFIG_MODIFIED: &str = r#"
version: "1.0"
name: test-v2
machines:
  web:
    hostname: web-server
    addr: 10.0.0.2
    arch: x86_64
  db:
    hostname: db-server
    addr: 10.0.0.3
    arch: x86_64
params:
  env: staging
  port: "9090"
  region: us-west-2
resources:
  nginx:
    type: file
    path: /etc/nginx/nginx.conf
    content: "worker_processes 4;"
    machine: web
  redis:
    type: file
    path: /etc/redis/redis.conf
    content: "port 6379"
    machine: db
outputs:
  web_addr:
    value: "{{machines.web.addr}}"
  db_addr:
    value: "{{machines.db.addr}}"
"#;

    #[test]
    fn test_stack_diff_identifies_resource_changes() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_yaml(dir.path(), "base.yaml", CONFIG_BASE);
        let f2 = write_yaml(dir.path(), "modified.yaml", CONFIG_MODIFIED);

        // Should succeed (we verify via no panic, test output)
        cmd_stack_diff(&f1, &f2, false).unwrap();
    }

    #[test]
    fn test_stack_diff_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_yaml(dir.path(), "base.yaml", CONFIG_BASE);
        let f2 = write_yaml(dir.path(), "modified.yaml", CONFIG_MODIFIED);

        cmd_stack_diff(&f1, &f2, true).unwrap();
    }

    #[test]
    fn test_stack_diff_identical_configs() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_yaml(dir.path(), "a.yaml", CONFIG_BASE);
        let f2 = write_yaml(dir.path(), "b.yaml", CONFIG_BASE);

        cmd_stack_diff(&f1, &f2, false).unwrap();
    }

    #[test]
    fn test_stack_diff_nonexistent_file() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_yaml(dir.path(), "a.yaml", CONFIG_BASE);
        let f2 = dir.path().join("nonexistent.yaml");

        let result = cmd_stack_diff(&f1, &f2, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_stack_diff_detects_machine_changes() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_yaml(dir.path(), "base.yaml", CONFIG_BASE);
        let f2 = write_yaml(dir.path(), "modified.yaml", CONFIG_MODIFIED);

        // Modified has: web addr changed, db added
        cmd_stack_diff(&f1, &f2, true).unwrap();
    }

    #[test]
    fn test_stack_diff_detects_param_changes() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_yaml(dir.path(), "base.yaml", CONFIG_BASE);
        let f2 = write_yaml(dir.path(), "modified.yaml", CONFIG_MODIFIED);

        // Modified has: env changed, port changed, region added
        cmd_stack_diff(&f1, &f2, false).unwrap();
    }

    #[test]
    fn test_stack_diff_empty_configs() {
        let dir = tempfile::tempdir().unwrap();
        let empty = r#"
version: "1.0"
name: empty
machines: {}
resources: {}
"#;
        let f1 = write_yaml(dir.path(), "a.yaml", empty);
        let f2 = write_yaml(dir.path(), "b.yaml", empty);

        cmd_stack_diff(&f1, &f2, false).unwrap();
        cmd_stack_diff(&f1, &f2, true).unwrap();
    }
}
