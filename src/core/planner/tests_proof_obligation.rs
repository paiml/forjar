//! Tests: FJ-1385 Proof obligation taxonomy.

#[cfg(test)]
mod tests {
    use crate::core::planner::proof_obligation::*;
    use crate::core::types::*;

    #[test]
    fn test_noop_always_idempotent() {
        for rtype in &[
            ResourceType::File,
            ResourceType::Package,
            ResourceType::Service,
            ResourceType::User,
            ResourceType::Model,
        ] {
            assert_eq!(classify(rtype, &PlanAction::NoOp), ProofObligation::Idempotent);
        }
    }

    #[test]
    fn test_file_create_idempotent() {
        assert_eq!(
            classify(&ResourceType::File, &PlanAction::Create),
            ProofObligation::Idempotent
        );
    }

    #[test]
    fn test_model_create_monotonic() {
        assert_eq!(
            classify(&ResourceType::Model, &PlanAction::Create),
            ProofObligation::Monotonic
        );
    }

    #[test]
    fn test_service_create_convergent() {
        assert_eq!(
            classify(&ResourceType::Service, &PlanAction::Create),
            ProofObligation::Convergent
        );
    }

    #[test]
    fn test_file_update_idempotent() {
        assert_eq!(
            classify(&ResourceType::File, &PlanAction::Update),
            ProofObligation::Idempotent
        );
    }

    #[test]
    fn test_package_update_convergent() {
        assert_eq!(
            classify(&ResourceType::Package, &PlanAction::Update),
            ProofObligation::Convergent
        );
    }

    #[test]
    fn test_file_destroy_destructive() {
        assert_eq!(
            classify(&ResourceType::File, &PlanAction::Destroy),
            ProofObligation::Destructive
        );
    }

    #[test]
    fn test_service_destroy_convergent() {
        assert_eq!(
            classify(&ResourceType::Service, &PlanAction::Destroy),
            ProofObligation::Convergent
        );
    }

    #[test]
    fn test_user_destroy_destructive() {
        assert_eq!(
            classify(&ResourceType::User, &PlanAction::Destroy),
            ProofObligation::Destructive
        );
    }

    #[test]
    fn test_model_destroy_destructive() {
        assert_eq!(
            classify(&ResourceType::Model, &PlanAction::Destroy),
            ProofObligation::Destructive
        );
    }

    #[test]
    fn test_label_strings() {
        assert_eq!(label(&ProofObligation::Idempotent), "idempotent");
        assert_eq!(label(&ProofObligation::Monotonic), "monotonic");
        assert_eq!(label(&ProofObligation::Convergent), "convergent");
        assert_eq!(label(&ProofObligation::Destructive), "destructive");
    }

    #[test]
    fn test_is_safe() {
        assert!(is_safe(&ProofObligation::Idempotent));
        assert!(is_safe(&ProofObligation::Monotonic));
        assert!(is_safe(&ProofObligation::Convergent));
        assert!(!is_safe(&ProofObligation::Destructive));
    }
}
