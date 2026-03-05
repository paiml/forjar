//! Tests: FJ-1450 configuration complexity analysis.

#![allow(unused_imports)]
use super::commands::*;
use super::complexity_analysis::*;
use super::dispatch::*;
use super::helpers::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
        let file = dir.join("forjar.yaml");
        std::fs::write(&file, yaml).unwrap();
        file
    }

    #[test]
    fn test_complexity_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
        );
        cmd_complexity(&file, false).unwrap();
    }

    #[test]
    fn test_complexity_simple() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: simple
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
  conf:
    type: file
    machine: m
    path: /etc/app.conf
    content: "key=value"
"#,
        );
        cmd_complexity(&file, false).unwrap();
    }

    #[test]
    fn test_complexity_medium() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: medium
machines:
  web:
    hostname: web
    addr: 10.0.0.1
  db:
    hostname: db
    addr: 10.0.0.2
resources:
  db-pkg:
    type: package
    machine: db
    provider: apt
    packages: [postgresql]
  web-pkg:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
  web-conf:
    type: file
    machine: web
    path: /etc/nginx/conf.d/app.conf
    content: "upstream db { server {{params.db_host}}; }"
    depends_on: [web-pkg, db-pkg]
"#,
        );
        cmd_complexity(&file, false).unwrap();
    }

    #[test]
    fn test_complexity_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: json\nmachines: {}\nresources: {}\n",
        );
        cmd_complexity(&file, true).unwrap();
    }

    #[test]
    fn test_complexity_with_conditionals() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: conditional
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
    when: "{{params.install_curl}} == true"
"#,
        );
        cmd_complexity(&file, false).unwrap();
    }

    #[test]
    fn test_complexity_with_unresolvable_includes() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: with-includes
includes:
  - /nonexistent/path/base.yaml
  - /also/missing/overrides.yaml
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
        // Should succeed via fallback parsing (not crash)
        cmd_complexity(&file, false).unwrap();
    }

    #[test]
    fn test_complexity_with_unresolvable_includes_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: includes-json
includes:
  - /nonexistent/include.yaml
machines: {}
resources: {}
"#,
        );
        cmd_complexity(&file, true).unwrap();
    }

    #[test]
    fn test_complexity_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        dispatch(
            Commands::Complexity(ComplexityArgs {
                file,
                json: false,
            }),
            0,
            true,
        )
        .unwrap();
    }
}
