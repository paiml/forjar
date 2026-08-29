//! Refs #358 — the canonical list of every [`ResourceType`], and the guard that
//! keeps it true.
//!
//! Its own module because the list plus its guard cost `resource_enums.rs` its
//! A+ (95.0 → 94.5, measured with `pmat tdg`) purely on size, and a 21-entry
//! array does not belong in the file that has to stay readable as the enum.

use super::resource_enums::ResourceType;

impl ResourceType {
    /// Every variant, in declaration order.
    ///
    /// Refs #358: `cli::plan_file` read a saved plan's `resource_type` through
    /// a hand-written string table that named 12 of these and mapped the other
    /// 9 to `File` — and the test that was supposed to cover it listed ten of
    /// the twelve, so the suite agreed with the reader about a schema neither
    /// of them had. A hand-written list goes stale silently, so this one is
    /// checked against the enum's own `Deserialize` derive by the test below.
    pub const ALL: [Self; 21] = [
        Self::Package,
        Self::File,
        Self::Service,
        Self::Mount,
        Self::User,
        Self::Docker,
        Self::Pepita,
        Self::Network,
        Self::Cron,
        Self::Recipe,
        Self::Model,
        Self::Gpu,
        Self::Task,
        Self::WasmBundle,
        Self::Image,
        Self::Build,
        Self::GithubRelease,
        Self::OverlayInterface,
        Self::DiskBudget,
        Self::BackupSync,
        Self::NasArchive,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ALL` must be the enum's variant list, not a snapshot of it.
    ///
    /// The list to check against comes from the DERIVE, not from a second
    /// hand-written table: serde's unknown-variant error enumerates every
    /// variant it accepts, so adding a variant to the enum and forgetting `ALL`
    /// fails here. The first version of this guard was a 21-arm exhaustive
    /// `match`, which said the same thing and cost the file its A+ (structural
    /// 25.0 → 20.5, measured with `pmat tdg`).
    #[test]
    fn all_lists_exactly_the_variants_serde_accepts() {
        let err = serde_json::from_value::<ResourceType>(serde_json::json!("__not_a_type__"))
            .expect_err("an unknown resource type must not deserialize");
        let message = err.to_string();
        // `unknown variant `x`, expected one of `package`, `file`, …`
        let listed: std::collections::BTreeSet<String> = message
            .split('`')
            .skip(3)
            .step_by(2)
            .map(str::to_string)
            .collect();
        assert!(
            listed.len() > 1,
            "serde no longer enumerates variants in this error, so this guard is blind: {message}"
        );
        let ours: std::collections::BTreeSet<String> =
            ResourceType::ALL.iter().map(ToString::to_string).collect();
        assert_eq!(ours, listed, "ResourceType::ALL is not the enum's variants");
    }
}
