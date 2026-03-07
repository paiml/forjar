//! Tests: FJ-2300 operator authorization via check_operator_auth.

#[cfg(test)]
mod tests {
    use super::super::dispatch_apply::check_operator_auth;
    use std::path::Path;

    fn write_config(dir: &Path, machine_block: &str) -> std::path::PathBuf {
        let target = dir.join("test.txt");
        let yaml = format!(
            "name: test\nversion: \"1.0\"\nmachines:\n{machine_block}\nresources:\n  f1:\n    type: file\n    path: {}\n    content: hello\n    machine: web\n",
            target.display()
        );
        let file = dir.join("forjar.yaml");
        std::fs::write(&file, yaml).unwrap();
        file
    }

    #[test]
    fn operator_auth_no_restrictions() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "  web:\n    hostname: localhost\n    addr: 127.0.0.1\n    transport: local",
        );
        let r = check_operator_auth(&file, Some("anyone"));
        assert!(r.is_ok(), "Expected ok, got: {r:?}");
        assert!(check_operator_auth(&file, None).is_ok());
    }

    #[test]
    fn operator_auth_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "  web:\n    hostname: localhost\n    addr: 127.0.0.1\n    transport: local\n    allowed_operators:\n      - deploy-bot\n      - noah",
        );
        assert!(check_operator_auth(&file, Some("deploy-bot")).is_ok());
        assert!(check_operator_auth(&file, Some("noah")).is_ok());
    }

    #[test]
    fn operator_auth_denied() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "  web:\n    hostname: localhost\n    addr: 127.0.0.1\n    transport: local\n    allowed_operators:\n      - deploy-bot",
        );
        let result = check_operator_auth(&file, Some("attacker"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("attacker"), "error should mention operator: {err}");
        assert!(err.contains("web"), "error should mention machine: {err}");
    }

    #[test]
    fn operator_auth_multi_machine_partial_deny() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("test.txt");
        let yaml = format!(
            "name: test\nversion: \"1.0\"\nmachines:\n  web:\n    hostname: localhost\n    addr: 127.0.0.1\n    transport: local\n  prod:\n    hostname: localhost\n    addr: 127.0.0.1\n    transport: local\n    allowed_operators:\n      - ci-bot\nresources:\n  f1:\n    type: file\n    path: {}\n    content: hello\n    machine: web\n",
            target.display()
        );
        let file = dir.path().join("forjar.yaml");
        std::fs::write(&file, yaml).unwrap();

        // ci-bot is allowed on prod (explicit) and web (no restrictions)
        assert!(check_operator_auth(&file, Some("ci-bot")).is_ok());
        // random-user is allowed on web but denied on prod
        let result = check_operator_auth(&file, Some("random-user"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("prod"));
    }

    #[test]
    fn operator_auth_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(&file, "invalid yaml content: [[[").unwrap();
        assert!(check_operator_auth(&file, Some("anyone")).is_err());
    }
}
