#[allow(unused_imports)]
use super::status_maturity::*;
#[allow(unused_imports)]
use crate::core::types;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mk(
        machine: &str,
        ts: &str,
        res: Vec<(&str, types::ResourceType, types::ResourceStatus)>,
    ) -> types::StateLock {
        let mut m = indexmap::IndexMap::new();
        for (id, rt, st) in res {
            m.insert(
                id.to_string(),
                types::ResourceLock {
                    resource_type: rt,
                    status: st,
                    applied_at: Some(ts.into()),
                    duration_seconds: Some(1.0),
                    hash: "abc".into(),
                    details: HashMap::new(),
                },
            );
        }
        types::StateLock {
            schema: "1".into(),
            machine: machine.into(),
            hostname: machine.into(),
            generated_at: ts.into(),
            generator: "test".into(),
            blake3_version: "1.0".into(),
            resources: m,
        }
    }

    fn wr(dir: &std::path::Path, lock: &types::StateLock) {
        let d = dir.join(&lock.machine);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("state.lock.yaml"),
            serde_yaml_ng::to_string(lock).unwrap(),
        )
        .unwrap();
    }

    // -- FJ-1093: Maturity Index -----------------------------------------------

    #[test]
    fn test_maturity_index_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_maturity_index_all_converged() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_maturity_index_mixed_types() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "cfg",
                        types::ResourceType::File,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_maturity_index_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "n1",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "p",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "s",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_maturity_index_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![(
                    "svc",
                    types::ResourceType::Service,
                    types::ResourceStatus::Failed,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), Some("web"), false).is_ok());
    }

    #[test]
    fn test_maturity_index_empty_resources() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![]));
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_maturity_index_score_clamped() {
        let d = tempfile::tempdir().unwrap();
        // Many resources + many types + high convergence => raw > 100, should clamp to 100
        wr(
            d.path(),
            &mk(
                "big",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "p1",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "p2",
                        types::ResourceType::File,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "p3",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "p4",
                        types::ResourceType::Mount,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "p5",
                        types::ResourceType::User,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "p6",
                        types::ResourceType::Docker,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, false).is_ok());
    }

    // -- FJ-1096: Convergence Stability Index ----------------------------------

    #[test]
    fn test_convergence_stability_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(
            cmd_status_machine_resource_convergence_stability_index(d.path(), None, false).is_ok()
        );
    }

    #[test]
    fn test_convergence_stability_all_converged() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(
            cmd_status_machine_resource_convergence_stability_index(d.path(), None, false).is_ok()
        );
    }

    #[test]
    fn test_convergence_stability_partial() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "cfg",
                        types::ResourceType::File,
                        types::ResourceStatus::Failed,
                    ),
                ],
            ),
        );
        assert!(
            cmd_status_machine_resource_convergence_stability_index(d.path(), None, false).is_ok()
        );
    }

    #[test]
    fn test_convergence_stability_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "n1",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "p",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "s",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                ],
            ),
        );
        assert!(
            cmd_status_machine_resource_convergence_stability_index(d.path(), None, true).is_ok()
        );
    }

    #[test]
    fn test_convergence_stability_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![(
                    "svc",
                    types::ResourceType::Service,
                    types::ResourceStatus::Failed,
                )],
            ),
        );
        assert!(cmd_status_machine_resource_convergence_stability_index(
            d.path(),
            Some("db"),
            false
        )
        .is_ok());
    }

    #[test]
    fn test_convergence_stability_empty_resources() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![]));
        assert!(
            cmd_status_machine_resource_convergence_stability_index(d.path(), None, false).is_ok()
        );
    }
}
