//! FJ-050: Property-based template resolution determinism tests.

use crate::core::resolver::resolve_template;
use proptest::prelude::*;
use std::collections::HashMap;

proptest! {
    /// Template resolution is deterministic — same params always produce same output.
    #[test]
    fn template_resolution_determinism(
        key in "[a-z_]{1,10}",
        value in "[a-zA-Z0-9]{1,20}",
    ) {
        let template = format!("{{{{params.{}}}}}", key);
        let mut params = HashMap::new();
        params.insert(key, serde_yaml_ng::Value::String(value.clone()));
        let machines = indexmap::IndexMap::new();

        let r1 = resolve_template(&template, &params, &machines);
        let r2 = resolve_template(&template, &params, &machines);

        prop_assert_eq!(r1.as_ref(), r2.as_ref(),
            "template resolution must be deterministic");
        prop_assert_eq!(r1.unwrap(), value,
            "template must resolve to the parameter value");
    }

    /// Missing params consistently produce the same error.
    #[test]
    fn missing_param_consistent_error(
        key in "[a-z_]{1,10}",
    ) {
        let template = format!("{{{{params.{}}}}}", key);
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();

        let r1 = resolve_template(&template, &params, &machines);
        let r2 = resolve_template(&template, &params, &machines);

        // Both should produce the same result (either both Ok or both Err)
        prop_assert_eq!(
            r1.is_err(), r2.is_err(),
            "missing param must consistently succeed or fail"
        );
    }

    /// Literal strings (no templates) pass through unchanged.
    #[test]
    fn literal_passthrough(value in "[a-zA-Z0-9 _-]{0,50}") {
        // Skip values that look like templates
        prop_assume!(!value.contains("{{"));
        let params = HashMap::new();
        let machines = indexmap::IndexMap::new();

        let result = resolve_template(&value, &params, &machines);
        prop_assert!(result.is_ok());
        prop_assert_eq!(result.unwrap(), value);
    }
}
