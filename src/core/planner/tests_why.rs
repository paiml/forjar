//! Tests: FJ-1379 --why change explanation.

#[cfg(test)]
mod tests {
    use crate::core::planner::why::{explain_why, format_why};
    use crate::core::types::*;
    use indexmap::IndexMap;
    use std::collections::HashMap;

    fn minimal_resource(rtype: ResourceType) -> Resource {
        Resource {
            resource_type: rtype,
            machine: MachineTarget::Single("local".to_string()),
            ..Resource::default()
        }
    }

    fn make_lock(
        machine: &str,
        resources: IndexMap<String, ResourceLock>,
    ) -> HashMap<String, StateLock> {
        let mut locks = HashMap::new();
        locks.insert(
            machine.to_string(),
            StateLock {
                schema: "v1".to_string(),
                machine: machine.to_string(),
                hostname: "localhost".to_string(),
                generated_at: "2026-03-03T12:00:00Z".to_string(),
                generator: "test".to_string(),
                blake3_version: "1.5.5".to_string(),
                resources,
            },
        );
        locks
    }

    #[test]
    fn test_why_new_resource_no_lock() {
        let resource = minimal_resource(ResourceType::File);
        let locks = HashMap::new(); // No locks at all
        let reason = explain_why("test-file", &resource, "local", &locks);
        assert_eq!(reason.action, PlanAction::Create);
        assert!(reason.reasons.iter().any(|r| r.contains("first apply")));
    }

    #[test]
    fn test_why_new_resource_not_in_lock() {
        let resource = minimal_resource(ResourceType::File);
        let locks = make_lock("local", IndexMap::new()); // Lock exists but empty
        let reason = explain_why("test-file", &resource, "local", &locks);
        assert_eq!(reason.action, PlanAction::Create);
        assert!(reason.reasons.iter().any(|r| r.contains("new resource")));
    }

    #[test]
    fn test_why_no_change() {
        let mut resource = minimal_resource(ResourceType::File);
        resource.path = Some("/tmp/test.txt".to_string());
        resource.content = Some("hello".to_string());

        let desired_hash = crate::core::planner::hash_desired_state(&resource);

        let mut rl_resources = IndexMap::new();
        rl_resources.insert(
            "test-file".to_string(),
            ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: Some("2026-03-03T12:00:00Z".to_string()),
                duration_seconds: Some(0.01),
                hash: desired_hash,
                details: HashMap::new(),
            },
        );

        let locks = make_lock("local", rl_resources);
        let reason = explain_why("test-file", &resource, "local", &locks);
        assert_eq!(reason.action, PlanAction::NoOp);
        assert!(reason.reasons.iter().any(|r| r.contains("hash unchanged")));
    }

    #[test]
    fn test_why_hash_changed() {
        let mut resource = minimal_resource(ResourceType::File);
        resource.path = Some("/tmp/test.txt".to_string());
        resource.content = Some("new content".to_string());

        let mut rl_resources = IndexMap::new();
        rl_resources.insert(
            "test-file".to_string(),
            ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: Some("2026-03-03T12:00:00Z".to_string()),
                duration_seconds: Some(0.01),
                hash: "blake3:old_hash_here".to_string(),
                details: HashMap::new(),
            },
        );

        let locks = make_lock("local", rl_resources);
        let reason = explain_why("test-file", &resource, "local", &locks);
        assert_eq!(reason.action, PlanAction::Update);
        assert!(reason.reasons.iter().any(|r| r.contains("hash changed")));
    }

    #[test]
    fn test_why_previously_failed() {
        let resource = minimal_resource(ResourceType::Package);

        let mut rl_resources = IndexMap::new();
        rl_resources.insert(
            "pkg".to_string(),
            ResourceLock {
                resource_type: ResourceType::Package,
                status: ResourceStatus::Failed,
                applied_at: Some("2026-03-03T12:00:00Z".to_string()),
                duration_seconds: Some(0.5),
                hash: "blake3:xxx".to_string(),
                details: HashMap::new(),
            },
        );

        let locks = make_lock("local", rl_resources);
        let reason = explain_why("pkg", &resource, "local", &locks);
        assert_eq!(reason.action, PlanAction::Update);
        assert!(reason
            .reasons
            .iter()
            .any(|r| r.contains("failed") && r.contains("retry")));
    }

    #[test]
    fn test_why_absent_destroy() {
        let mut resource = minimal_resource(ResourceType::File);
        resource.state = Some("absent".to_string());

        let mut rl_resources = IndexMap::new();
        rl_resources.insert(
            "rm-file".to_string(),
            ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: Some("2026-03-03T12:00:00Z".to_string()),
                duration_seconds: Some(0.01),
                hash: "blake3:xxx".to_string(),
                details: HashMap::new(),
            },
        );

        let locks = make_lock("local", rl_resources);
        let reason = explain_why("rm-file", &resource, "local", &locks);
        assert_eq!(reason.action, PlanAction::Destroy);
        assert!(reason.reasons.iter().any(|r| r.contains("absent")));
    }

    #[test]
    fn test_why_format_output() {
        let mut resource = minimal_resource(ResourceType::File);
        resource.path = Some("/tmp/test.txt".to_string());
        resource.content = Some("hello".to_string());

        let locks = HashMap::new();
        let reason = explain_why("test-file", &resource, "local", &locks);
        let output = format_why(&reason);
        assert!(output.contains("test-file"));
        assert!(output.contains("local"));
    }

    #[test]
    fn test_why_content_field_diff() {
        let mut resource = minimal_resource(ResourceType::File);
        resource.path = Some("/tmp/test.txt".to_string());
        resource.content = Some("new content".to_string());

        let old_content_hash = crate::tripwire::hasher::hash_string("old content");

        let mut details = HashMap::new();
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String(old_content_hash),
        );
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String("/tmp/test.txt".to_string()),
        );

        let mut rl_resources = IndexMap::new();
        rl_resources.insert(
            "test-file".to_string(),
            ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: Some("2026-03-03T12:00:00Z".to_string()),
                duration_seconds: Some(0.01),
                hash: "blake3:old_hash_here".to_string(),
                details,
            },
        );

        let locks = make_lock("local", rl_resources);
        let reason = explain_why("test-file", &resource, "local", &locks);
        assert_eq!(reason.action, PlanAction::Update);
        assert!(reason.reasons.iter().any(|r| r.contains("content changed")));
    }
}
