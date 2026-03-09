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
    fn test_cron_destroy_reversible() {
        let r = minimal_resource(ResourceType::Cron);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Reversible
        );
    }

    #[test]
    fn test_mount_destroy_reversible() {
        let r = minimal_resource(ResourceType::Mount);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Reversible
        );
    }

    #[test]
    fn test_docker_destroy_reversible() {
        let r = minimal_resource(ResourceType::Docker);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Reversible
        );
    }

    #[test]
    fn test_pepita_destroy_reversible() {
        let r = minimal_resource(ResourceType::Pepita);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Reversible
        );
    }

    #[test]
    fn test_network_destroy_irreversible() {
        let r = minimal_resource(ResourceType::Network);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Irreversible
        );
    }

    #[test]
    fn test_gpu_destroy_reversible() {
        let r = minimal_resource(ResourceType::Gpu);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Reversible
        );
    }

    #[test]
    fn test_task_destroy_irreversible() {
        let r = minimal_resource(ResourceType::Task);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Irreversible
        );
    }

    #[test]
    fn test_recipe_destroy_irreversible() {
        let r = minimal_resource(ResourceType::Recipe);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Irreversible
        );
    }

    #[test]
    fn test_wasm_bundle_destroy_reversible() {
        let r = minimal_resource(ResourceType::WasmBundle);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Reversible
        );
    }

    #[test]
    fn test_image_destroy_reversible() {
        let r = minimal_resource(ResourceType::Image);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Reversible
        );
    }

    #[test]
    fn test_build_destroy_reversible() {
        let r = minimal_resource(ResourceType::Build);
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Reversible
        );
    }

    #[test]
    fn test_file_destroy_with_source_reversible() {
        let mut r = minimal_resource(ResourceType::File);
        r.source = Some("/path/to/source".to_string());
        assert_eq!(
            classify(&r, &PlanAction::Destroy),
            Reversibility::Reversible
        );
    }

    fn test_config() -> ForjarConfig {
        ForjarConfig {
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
            environments: indexmap::IndexMap::new(),
        }
    }

    #[test]
    fn test_count_irreversible_empty_plan() {
        let config = test_config();
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

    #[test]
    fn test_count_irreversible_with_destroys() {
        let mut config = test_config();
        config
            .resources
            .insert("user1".into(), minimal_resource(ResourceType::User));
        config
            .resources
            .insert("pkg1".into(), minimal_resource(ResourceType::Package));

        let plan = ExecutionPlan {
            name: "test".to_string(),
            changes: vec![
                PlannedChange {
                    resource_id: "user1".into(),
                    resource_type: ResourceType::User,
                    machine: "local".into(),
                    action: PlanAction::Destroy,
                    description: "destroy user1".into(),
                },
                PlannedChange {
                    resource_id: "pkg1".into(),
                    resource_type: ResourceType::Package,
                    machine: "local".into(),
                    action: PlanAction::Destroy,
                    description: "destroy pkg1".into(),
                },
            ],
            execution_order: vec![],
            to_create: 0,
            to_update: 0,
            to_destroy: 2,
            unchanged: 0,
        };
        // user destroy = irreversible, package destroy = reversible
        assert_eq!(count_irreversible(&config, &plan), 1);
    }

    #[test]
    fn test_count_irreversible_create_not_counted() {
        let mut config = test_config();
        config
            .resources
            .insert("user1".into(), minimal_resource(ResourceType::User));

        let plan = ExecutionPlan {
            name: "test".to_string(),
            changes: vec![PlannedChange {
                resource_id: "user1".into(),
                resource_type: ResourceType::User,
                machine: "local".into(),
                action: PlanAction::Create,
                description: "create user1".into(),
            }],
            execution_order: vec![],
            to_create: 1,
            to_update: 0,
            to_destroy: 0,
            unchanged: 0,
        };
        assert_eq!(count_irreversible(&config, &plan), 0);
    }

    #[test]
    fn test_warn_irreversible_returns_messages() {
        let mut config = test_config();
        config
            .resources
            .insert("user1".into(), minimal_resource(ResourceType::User));
        config
            .resources
            .insert("net1".into(), minimal_resource(ResourceType::Network));
        config
            .resources
            .insert("pkg1".into(), minimal_resource(ResourceType::Package));

        let plan = ExecutionPlan {
            name: "test".to_string(),
            changes: vec![
                PlannedChange {
                    resource_id: "user1".into(),
                    resource_type: ResourceType::User,
                    machine: "local".into(),
                    action: PlanAction::Destroy,
                    description: "destroy user1".into(),
                },
                PlannedChange {
                    resource_id: "net1".into(),
                    resource_type: ResourceType::Network,
                    machine: "local".into(),
                    action: PlanAction::Destroy,
                    description: "destroy net1".into(),
                },
                PlannedChange {
                    resource_id: "pkg1".into(),
                    resource_type: ResourceType::Package,
                    machine: "local".into(),
                    action: PlanAction::Destroy,
                    description: "destroy pkg1".into(),
                },
            ],
            execution_order: vec![],
            to_create: 0,
            to_update: 0,
            to_destroy: 3,
            unchanged: 0,
        };
        let warnings = warn_irreversible(&config, &plan);
        assert_eq!(warnings.len(), 2); // user + network
        assert!(warnings[0].contains("irreversible"));
        assert!(warnings[1].contains("irreversible"));
    }

    #[test]
    fn test_warn_irreversible_empty_plan() {
        let config = test_config();
        let plan = ExecutionPlan {
            name: "test".to_string(),
            changes: vec![],
            execution_order: vec![],
            to_create: 0,
            to_update: 0,
            to_destroy: 0,
            unchanged: 0,
        };
        assert!(warn_irreversible(&config, &plan).is_empty());
    }

    #[test]
    fn test_warn_irreversible_unknown_resource_defaults_irreversible() {
        let config = test_config(); // No resources in config

        let plan = ExecutionPlan {
            name: "test".to_string(),
            changes: vec![PlannedChange {
                resource_id: "gone".into(),
                resource_type: ResourceType::File,
                machine: "local".into(),
                action: PlanAction::Destroy,
                description: "destroy gone".into(),
            }],
            execution_order: vec![],
            to_create: 0,
            to_update: 0,
            to_destroy: 1,
            unchanged: 0,
        };
        // Resource "gone" not in config => defaults to irreversible
        assert_eq!(count_irreversible(&config, &plan), 1);
    }
}
