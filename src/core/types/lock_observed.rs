//! Reading and writing a lock entry's OBSERVED state.
//!
//! Split out of `state_types.rs` to keep that file under the 500-line budget.
//! The accessors live together because the whole point of forjar#337 Step 1 is
//! that there is exactly ONE place to ask what a target reported.

use super::state_types::ResourceLock;

impl ResourceLock {
    /// What the target reported, or `None` if it was never observed.
    ///
    /// THE ONLY WAY TO ASK. Every caller goes through here so the transitional
    /// fallback to `details["live_hash"]` lives in exactly one place and can be
    /// deleted in exactly one place.
    ///
    /// The fallback exists because every lock file currently on the fleet was
    /// written by a forjar that stored this in the details map. Reading it here
    /// means a 1.18.0 lock keeps working; it is a READ path only, so it cannot
    /// reintroduce the two-writers problem that caused forjar#305.
    ///
    /// REMOVE the fallback once no fleet machine holds a pre-1.19 lock — the
    /// point of this field is that there is one place to look.
    ///
    /// (The `#[serde(default)]` on the field above is REDUNDANT: serde already
    /// deserializes a missing `Option<T>` to `None`. Measured, not assumed —
    /// removing it left `a_lock_without_the_field_still_loads` green. It stays
    /// for legibility. What actually makes that test pass is this fallback:
    /// break it and the test goes red.)
    /// A string value out of the untyped `details` map, or `None`.
    ///
    /// Exists so the `Some(Value::String(s)) => s.as_str(), _ => ...` match is
    /// written once. Note what it is NOT for: the observed state has its own
    /// typed field and [`observed_state`](Self::observed_state) — reaching for
    /// it by string key is the mistake forjar#305 was.
    pub fn detail_str(&self, key: &str) -> Option<&str> {
        match self.details.get(key) {
            Some(serde_yaml_ng::Value::String(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn observed_state(&self) -> Option<&str> {
        if let Some(ref o) = self.observed {
            return Some(o.as_str());
        }
        match self.details.get("live_hash") {
            Some(serde_yaml_ng::Value::String(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Record what the target reported.
    ///
    /// Writes both the typed field and the legacy `details` key for this
    /// release, so a rollback to 1.18.0 does not silently blind drift
    /// detection — its reader skips any resource with no `live_hash`, so a
    /// missing key is not a loud failure but a silent one.
    ///
    /// This is ONE value written to two places, which is not the shape of
    /// forjar#305. That was two DIFFERENT values, from different sources, with
    /// callers reading whichever they happened to name.
    pub fn set_observed_state(&mut self, digest: impl Into<String>) {
        let d = digest.into();
        self.details.insert(
            "live_hash".to_string(),
            serde_yaml_ng::Value::String(d.clone()),
        );
        self.observed = Some(d);
    }
}
