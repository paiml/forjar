//! Tests: FJ-1384 stack extraction.

#[cfg(test)]
mod tests {
    use crate::cli::extract::*;
    
    use std::io::Write;

    fn write_test_config(dir: &std::path::Path) -> std::path::PathBuf {
        let path = dir.join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test-config
machines:
  web-server:
    hostname: web-server
    addr: 10.0.0.1
    user: root
  db-server:
    hostname: db-server
    addr: 10.0.0.2
    user: root
  cache-server:
    hostname: cache-server
    addr: 10.0.0.3
    user: root
resources:
  nginx-conf:
    type: file
    machine: web-server
    path: /etc/nginx/nginx.conf
    content: "server {}"
    tags: [web, networking]
    resource_group: frontend
  redis-conf:
    type: file
    machine: cache-server
    path: /etc/redis/redis.conf
    content: "bind 0.0.0.0"
    tags: [cache, backend]
    resource_group: backend
  pg-conf:
    type: file
    machine: db-server
    path: /etc/postgresql/pg.conf
    content: "listen_addresses = '*'"
    tags: [database, backend]
    resource_group: backend
  web-site:
    type: file
    machine: web-server
    path: /var/www/index.html
    content: "<html>hello</html>"
    tags: [web, networking]
    resource_group: frontend
"#;
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_extract_by_tag() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_test_config(dir.path());
        let out_path = dir.path().join("extracted.yaml");
        let result = cmd_extract(
            &config_path,
            Some("web"),
            None,
            None,
            Some(&out_path),
            false,
        );
        result.unwrap();
        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(content.contains("nginx-conf"));
        assert!(content.contains("web-site"));
        assert!(!content.contains("redis-conf"));
        assert!(!content.contains("pg-conf"));
    }

    #[test]
    fn test_extract_by_group() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_test_config(dir.path());
        let out_path = dir.path().join("extracted.yaml");
        let result = cmd_extract(
            &config_path,
            None,
            Some("backend"),
            None,
            Some(&out_path),
            false,
        );
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(content.contains("redis-conf"));
        assert!(content.contains("pg-conf"));
        assert!(!content.contains("nginx-conf"));
    }

    #[test]
    fn test_extract_by_glob() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_test_config(dir.path());
        let out_path = dir.path().join("extracted.yaml");
        let result = cmd_extract(
            &config_path,
            None,
            None,
            Some("web-*"),
            Some(&out_path),
            false,
        );
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(content.contains("web-site"));
        assert!(!content.contains("redis-conf"));
    }

    #[test]
    fn test_extract_machines_filtered() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_test_config(dir.path());
        let out_path = dir.path().join("extracted.yaml");
        let result = cmd_extract(
            &config_path,
            Some("database"),
            None,
            None,
            Some(&out_path),
            false,
        );
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(content.contains("db-server"));
        assert!(!content.contains("web-server"));
        assert!(!content.contains("cache-server"));
    }

    #[test]
    fn test_extract_no_filter_fails() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_test_config(dir.path());
        let result = cmd_extract(&config_path, None, None, None, None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least one"));
    }

    #[test]
    fn test_extract_no_match_fails() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_test_config(dir.path());
        let result = cmd_extract(
            &config_path,
            Some("nonexistent-tag"),
            None,
            None,
            None,
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no resources match"));
    }

    #[test]
    fn test_extract_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_test_config(dir.path());
        let out_path = dir.path().join("extracted.json");
        let result = cmd_extract(
            &config_path,
            Some("web"),
            None,
            None,
            Some(&out_path),
            true,
        );
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&out_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.get("resources").is_some());
    }

    #[test]
    fn test_extract_name_updated() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_test_config(dir.path());
        let out_path = dir.path().join("extracted.yaml");
        let result = cmd_extract(
            &config_path,
            Some("web"),
            None,
            None,
            Some(&out_path),
            false,
        );
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&out_path).unwrap();
        assert!(content.contains("extract: tags=web"));
    }
}
