//! The `verify_bindings` test that needs aprender's binding registry (#452).
//!
//! Split out of `build_helper.rs` for one mechanical reason: that file is over
//! the repo's 500-line ceiling and the pre-commit ratchet forbids growth, so
//! the `ignore` attribute could not be added in place. The body is unchanged.
//!
//! It reads `<crate>/../../contracts/aprender/binding.yaml`, which forjar does
//! not vendor. See VENDORED.md.

#[cfg(test)]
mod tests {
    #![allow(clippy::all)]
    use super::super::*;

    #[test]
    #[cfg_attr(
        not(feature = "aprender-corpus"),
        ignore = "needs contracts/aprender/binding.yaml — forjar vendors the crate, not aprender's binding registry (#452)"
    )]
    fn verify_bindings_warn_on_gaps_real_file() {
        let binding_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../contracts/aprender/binding.yaml");
        let result = verify_bindings(binding_path.to_str().unwrap(), BindingPolicy::WarnOnGaps);
        assert!(
            result.bound_count > 0,
            "Should have some implemented bindings"
        );
    }
}
