#[allow(unused_imports)]
use super::status_fleet_insight::*;
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

    // ── FJ-1069: Staleness Report ─────────────────────────────────────────

    #[test]
    fn test_staleness_report_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_staleness_report(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_staleness_report_recent_data() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-02-28T10:00:00Z",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_staleness_report(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_staleness_report_stale_data() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2024-01-01T00:00:00Z",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_staleness_report(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_staleness_report_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2024-01-01T00:00:00Z",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_staleness_report(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_staleness_report_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2024-01-01T00:00:00Z",
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
                "2026-02-28T10:00:00Z",
                vec![(
                    "svc",
                    types::ResourceType::Service,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_staleness_report(d.path(), Some("web"), false).is_ok());
    }

    // ── FJ-1072: Type Distribution ────────────────────────────────────────

    #[test]
    fn test_type_distribution_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_type_distribution(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_type_distribution_mixed_types() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg1",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "pkg2",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "cfg",
                        types::ResourceType::File,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_machine_resource_type_distribution(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_type_distribution_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "f1",
                        types::ResourceType::File,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "f2",
                        types::ResourceType::File,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "s1",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                ],
            ),
        );
        assert!(cmd_status_machine_resource_type_distribution(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_type_distribution_filter() {
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
                    types::ResourceStatus::Drifted,
                )],
            ),
        );
        assert!(cmd_status_machine_resource_type_distribution(d.path(), Some("db"), false).is_ok());
    }

    #[test]
    fn test_type_distribution_multiple_machines() {
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
                        "cfg",
                        types::ResourceType::File,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "mnt",
                        types::ResourceType::Mount,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "net",
                        types::ResourceType::Network,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_machine_resource_type_distribution(d.path(), None, false).is_ok());
    }

    // ── FJ-1075: Health Score ─────────────────────────────────────────────

    #[test]
    fn test_health_score_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_machine_health_score(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_health_score_all_converged() {
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
        assert!(cmd_status_fleet_machine_health_score(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_health_score_mixed_status() {
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
        assert!(cmd_status_fleet_machine_health_score(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_health_score_json() {
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
        assert!(cmd_status_fleet_machine_health_score(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_health_score_filter() {
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
        assert!(cmd_status_fleet_machine_health_score(d.path(), Some("web"), false).is_ok());
    }

    #[test]
    fn test_health_score_empty_resources() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![]));
        assert!(cmd_status_fleet_machine_health_score(d.path(), None, false).is_ok());
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    #[test]
    fn test_classify_resources_all_statuses() {
        let lock = mk(
            "m",
            "2026-01-15T10:00:00Z",
            vec![
                (
                    "a",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                ),
                (
                    "b",
                    types::ResourceType::Service,
                    types::ResourceStatus::Drifted,
                ),
                (
                    "c",
                    types::ResourceType::File,
                    types::ResourceStatus::Failed,
                ),
                (
                    "d",
                    types::ResourceType::File,
                    types::ResourceStatus::Unknown,
                ),
            ],
        );
        assert_eq!(classify_resources(&lock), (1, 1, 1, 1));
    }

    #[test]
    fn test_classify_resources_empty() {
        let lock = mk("m", "2026-01-15T10:00:00Z", vec![]);
        assert_eq!(classify_resources(&lock), (0, 0, 0, 0));
    }

    #[test]
    fn test_parse_rfc3339_valid() {
        let e = parse_rfc3339_to_epoch("2024-01-01T00:00:00Z");
        assert!(e.is_some());
        assert!(e.unwrap() > 1_700_000_000 && e.unwrap() < 1_800_000_000);
    }

    #[test]
    fn test_parse_rfc3339_invalid() {
        assert!(parse_rfc3339_to_epoch("").is_none());
        assert!(parse_rfc3339_to_epoch("short").is_none());
        assert!(parse_rfc3339_to_epoch("not-a-timestamp!!").is_none());
    }
}
