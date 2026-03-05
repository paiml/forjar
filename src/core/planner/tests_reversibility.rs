//! Tests: FJ-1382 Reversibility classification.

#[cfg(test)]
mod tests {
    use crate::core::planner::reversibility::*;
    use crate::core::types::*;
    use std::collections::HashMap;

    fn minimal_resource(rtype: ResourceType) -> Resource {
        Resource {
            resource_type: rtype,
            machine: MachineTarget::Single("local".to_string()),
            ..Resource::default()
        }
    }

    #[test]
    fn test_create_is_reversible() {
        let r = minimal_resource(ResourceType::File);
        assert_eq!(classify(&r, &PlanAction::Create), Reversibility::Reversible);
    }

    #[test]
    fn test_noop_is_reversible() {
        let r = minimal_resource(ResourceType::Package);
        assert_eq!(classify(&r, &PlanAction::NoOp), Reversibility::Reversible);
    }

    #[test]
    fn test_update_is_reversible() {
        let r = minimal_resource(ResourceType::Service);
        assert_eq!(classify(&r, &PlanAction::Update), Reversibility::Reversible);
    }

    #[test]
    fn test_file_destroy_with_content_reversible() {
        let mut r = minimal_resource(ResourceType::File);
        r.content = Some("hello".to_string());
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Reversible
        );
    }

    #[test]
    fn test_file_destroy_no_content_irreversible() {
        let r = minimal_resource(ResourceType::File);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Irreversible
        );
    }

    #[test]
    fn test_user_destroy_irreversible() {
        let r = minimal_resource(ResourceType::User);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Irreversible
        );
    }

    #[test]
    fn test_service_destroy_reversible() {
        let r = minimal_resource(ResourceType::Service);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Reversible
        );
    }

    #[test]
    fn test_package_destroy_reversible() {
        let r = minimal_resource(ResourceType::Package);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Reversible
        );
    }

    #[test]
    fn test_model_destroy_irreversible() {
        let r = minimal_resource(ResourceType::Model);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Irreversible
        );
    }

    #[test]
    fn test_count_irreversible_empty_plan() {
        let config = ForjarConfig {
            version: "1.0".to_string(),
            name: "test".to_string(),
            description: None,
            machines: indexmap::IndexMap::new(),
            resources: indexmap::IndexMap::new(),
            params: std::collections::HashMap::new(),
            outputs: indexmap::IndexMap::new(),
            policy: Policy::default(),
            policies: vec![],
            moved: vec![],
            secrets: Default::default(),
            includes: vec![],
            include_provenance: HashMap::new(),
            data: indexmap::IndexMap::new(),
            checks: indexmap::IndexMap::new(),
        };
        let plan = ExecutionPlan {
            name: "test".to_string(),
            changes: vec![],
            execution_order: vec![],
            to_create: 0,
            to_update: 0,
            to_destroy: 0,
            unchanged: 0,
        };
        assert_eq!(count_irreversible(&config, &plan), 0);
    }
}
