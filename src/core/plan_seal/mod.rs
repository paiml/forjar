//! Refs #356 / #358 — bind a saved plan to the world it was planned in.
//!
//! # What this is, and what it is deliberately not
//!
//! A seal is an INTEGRITY device. It detects that the inputs a plan was built
//! from have moved, or that the plan body itself was edited, before that plan
//! is allowed to drive an apply. It is an unkeyed BLAKE3 hash, so it is NOT an
//! authenticator: anyone who can run `forjar plan` can compute a seal, and
//! nothing here proves *forjar issued this plan*. Calling it a "cryptographic
//! plan token" and building an authorization decision on top of it would be a
//! category error — if two-phase authorization is ever wanted it needs a keyed
//! hash or the signing machinery in `cli::pq_signing`, and that is a different
//! feature.
//!
//! What it honestly delivers is the guarantee the saved-plan feature exists
//! for: the delta that executes is the delta that was reviewed.
//!
//! # What nothing in this module can do (Refs #358)
//!
//! It cannot stop an adversary who can run `forjar`. Copy `config_hash` and
//! `state_hash` verbatim out of an honest plan — neither leg has moved — rewrite
//! the body, recompute [`digest::diff_leg`] and [`digest::compose`] through this
//! module's own public API, and the result passes every check below. That is
//! not a gap to be closed with more hashing; it is what "unkeyed" means.
//!
//! An earlier version of this doc claimed [`check_body_partition`] "still
//! refuses a zero-the-counters edit whose author ALSO recomputed the seal".
//! That was false and is deleted: `0/0/0/0` over an EMPTY change list
//! partitions perfectly well. The claim is repaired where it can be — not here,
//! but in `cli::apply_from_plan_checks::check_plan_still_holds`, which RE-PLANS
//! from the live config and the live locks and refuses a plan the planner
//! contradicts. No adversary can make the real planner return `NoOp` while a
//! create is pending, and that check needs no key, no clock and no trust.
//!
//! What this module contributes to that check is
//! [`crate::core::plan_selectors::PlanSelectors`], sealed into the diff leg. The
//! re-plan needs to know which plan the document claims to be — a whole-stack
//! plan or a `-r bravo` one — because those two produce byte-identical bodies
//! over a partially converged stack. An adversary can still re-seal a forged
//! selector record, but they can no longer forge one INVISIBLY: the document
//! now has to declare itself narrow, and the apply prints that declaration.
//!
//! # Three legs
//!
//! See [`digest`]. Only the config leg shipped before; a plan whose config hash
//! matched could still have been planned against a lock that has since moved
//! (TOCTOU), or have had its change list and counters hand-edited.
//!
//! # Time is liveness hygiene, not safety
//!
//! `ttl_secs` bounds how long a forgotten plan stays usable. It is wall-clock
//! and forjar has no trusted clock, so it is not a security control — a plan 16
//! minutes old whose three legs still match is strictly safer to apply than a
//! one-minute-old plan whose legs do not. It is therefore OPT-IN: `ttl_secs`
//! of 0 means "no wall-clock expiry", which is what `forjar plan --out` writes,
//! so sealing does not silently break a CI job that plans and applies in
//! separate stages. A caller that wants an expiry passes one and it is clamped
//! to [`MIN_TTL_SECS`]..=[`MAX_TTL_SECS`].
//!
//! `now` is injected everywhere, so no test sleeps or reads the system clock.

pub mod digest;

#[cfg(test)]
#[path = "tests_digest.rs"]
mod tests_digest;
#[cfg(test)]
#[path = "tests_verify.rs"]
mod tests_verify;

use crate::core::plan_selectors::PlanSelectors;
use crate::core::types::{ExecutionPlan, ForjarConfig, PlanAction};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

/// Schema tag stored in every seal. A seal carrying anything else is refused
/// rather than best-effort interpreted.
pub const SEAL_VERSION: &str = "forjar-plan-seal-v1";

/// The TTL a caller gets when it asks for the default lifetime.
pub const DEFAULT_TTL_SECS: u64 = 900;
/// Shortest honoured lifetime; anything smaller is a self-inflicted race.
pub const MIN_TTL_SECS: u64 = 60;
/// Longest honoured lifetime.
pub const MAX_TTL_SECS: u64 = 3600;
/// `ttl_secs` value that disables the wall-clock check entirely.
pub const TTL_NO_EXPIRY: u64 = 0;

/// Which binding moved. Named in the error so the operator does not have to
/// guess whether to re-plan, re-seal or investigate a host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leg {
    /// `forjar.yaml` changed.
    Config,
    /// A machine's lock file changed.
    State,
    /// The plan body changed.
    Diff,
    /// The seal does not match the fields it is supposed to bind.
    Seal,
}

impl Leg {
    /// Human name used in messages.
    pub fn name(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::State => "state",
            Self::Diff => "diff",
            Self::Seal => "seal",
        }
    }

    /// What the operator should do about it.
    fn remedy(self) -> &'static str {
        match self {
            Self::Config => "the config changed since the plan was sealed",
            Self::State => "a machine's state lock changed since the plan was sealed",
            Self::Diff => "the plan body was modified after it was sealed",
            Self::Seal => "the plan's seal does not match its own fields",
        }
    }
}

/// Why a sealed plan was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealError {
    /// The plan's wall-clock lifetime has elapsed.
    PlanExpired {
        /// Unix seconds at which the plan was sealed.
        sealed_at: u64,
        /// Unix seconds after which it is no longer honoured.
        expires_at: u64,
        /// Unix seconds now.
        now: u64,
    },
    /// One of the bindings no longer matches the live world.
    PlanHashMismatch {
        /// Which binding moved.
        leg: Leg,
        /// What the seal recorded.
        expected: String,
        /// What recomputing from live inputs produced.
        actual: String,
    },
    /// The plan document is not structurally usable.
    PlanMalformed(String),
    /// The seal carries a schema tag this build does not implement.
    PlanVersionUnknown(String),
}

impl SealError {
    /// Stable machine-readable code.
    ///
    /// `VerbSpec::invoke` and the CLI both carry errors as `String`, so the code
    /// is a message PREFIX rather than a typed field. Callers and tests match on
    /// the prefix; turning it into a typed envelope is a separate change that
    /// must not ride in on this one.
    pub fn code(&self) -> &'static str {
        match self {
            Self::PlanExpired { .. } => "PLAN_EXPIRED",
            Self::PlanHashMismatch { .. } => "PLAN_HASH_MISMATCH",
            Self::PlanMalformed(_) => "PLAN_MALFORMED",
            Self::PlanVersionUnknown(_) => "PLAN_VERSION_UNKNOWN",
        }
    }
}

impl fmt::Display for SealError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Self::PlanExpired {
                sealed_at,
                expires_at,
                now,
            } => write!(
                f,
                "plan sealed at {sealed_at} expired at {expires_at} (now {now}) \
                 — re-run `forjar plan`"
            ),
            Self::PlanHashMismatch {
                leg,
                expected,
                actual,
            } => write!(
                f,
                "{} ({} leg: expected {expected}, got {actual}) — re-run `forjar plan`",
                leg.remedy(),
                leg.name()
            ),
            Self::PlanMalformed(why) => write!(f, "{why}"),
            Self::PlanVersionUnknown(v) => write!(
                f,
                "plan seal version '{v}' is not understood by this forjar \
                 (expected '{SEAL_VERSION}') — re-run `forjar plan`"
            ),
        }
    }
}

impl std::error::Error for SealError {}

/// The seal fields carried alongside a plan body.
///
/// `deny_unknown_fields`: a seal document with a field this build does not know
/// is refused, not silently half-read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanSeal {
    /// Schema tag; always [`SEAL_VERSION`] when written by this build.
    pub version: String,
    /// Short content-derived handle for the sealed plan.
    pub plan_id: String,
    /// Leg 1 — canonical hash of the config.
    pub config_hash: String,
    /// Leg 2 — hash of the lock files the planner read.
    pub state_hash: String,
    /// Leg 3 — hash of the plan body.
    pub diff_hash: String,
    /// Unix seconds at which the plan was sealed.
    pub sealed_at_unix: u64,
    /// Wall-clock lifetime in seconds; 0 means no expiry.
    pub ttl_secs: u64,
    /// Composition over every field above.
    pub seal: String,
}

/// Clamp a requested lifetime into the honoured range.
///
/// `None` and an explicit 0 both mean "no wall-clock expiry". Everything else
/// is pulled into [`MIN_TTL_SECS`]..=[`MAX_TTL_SECS`] so a caller cannot ask for
/// a one-second window or a one-year one; the ACTUAL value is what gets sealed.
pub fn clamp_ttl(requested: Option<u64>) -> u64 {
    match requested {
        None | Some(TTL_NO_EXPIRY) => TTL_NO_EXPIRY,
        Some(secs) => secs.clamp(MIN_TTL_SECS, MAX_TTL_SECS),
    }
}

/// Seconds since the Unix epoch, or 0 if the clock is before it.
pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Seal a plan against the config and state it was planned from, at `now`.
///
/// Refs #358: `selectors` records what `forjar plan` was FILTERED by, and is
/// sealed with the body. `apply --plan-file` re-plans under it, so a document
/// that lies about its own filters is a `PLAN_HASH_MISMATCH` rather than a
/// re-plan that quietly agrees with a forgery.
pub fn seal_at(
    plan: &ExecutionPlan,
    selectors: &PlanSelectors,
    config: &ForjarConfig,
    state_dir: &Path,
    ttl_secs: Option<u64>,
    now: u64,
) -> Result<PlanSeal, String> {
    let config_hash = digest::config_leg(config)?;
    let state_hash = digest::state_leg(config, state_dir)?;
    let diff_hash = digest::diff_leg(plan, selectors)?;
    let ttl_secs = clamp_ttl(ttl_secs);
    let seal = digest::compose(&config_hash, &state_hash, &diff_hash, now, ttl_secs);
    Ok(PlanSeal {
        version: SEAL_VERSION.to_string(),
        plan_id: digest::plan_id(&seal),
        config_hash,
        state_hash,
        diff_hash,
        sealed_at_unix: now,
        ttl_secs,
        seal,
    })
}

/// Seal a plan using the system clock.
pub fn seal(
    plan: &ExecutionPlan,
    selectors: &PlanSelectors,
    config: &ForjarConfig,
    state_dir: &Path,
    ttl_secs: Option<u64>,
) -> Result<PlanSeal, String> {
    seal_at(plan, selectors, config, state_dir, ttl_secs, now_unix())
}

/// Verify a seal against LIVE inputs at `now`.
///
/// # Order is deliberate
///
/// 1. version — a seal this build cannot interpret is refused outright;
/// 2. body partition — a plan whose counters contradict its own change list is
///    malformed regardless of any hash. It is a STRUCTURAL check, not a defence
///    against a re-sealing editor: see [`check_body_partition`] for the edit it
///    does not catch;
/// 3. self-consistency — recompose from the STORED fields. This is what catches
///    an edited `sealed_at_unix` or `ttl_secs`: moving the expiry breaks the
///    seal instead of extending the life;
/// 4. per-leg — recompute each leg from live inputs so the error can NAME which
///    binding moved. Reported before expiry, because "your config changed" is
///    the stronger and more actionable reason than "your plan is old";
/// 5. expiry — last, and only when a lifetime was requested.
pub fn verify_at(
    sealed: &PlanSeal,
    plan: &ExecutionPlan,
    selectors: &PlanSelectors,
    config: &ForjarConfig,
    state_dir: &Path,
    now: u64,
) -> Result<(), SealError> {
    check_version(sealed)?;
    check_body_partition(plan)?;
    check_self_consistency(sealed)?;
    check_legs(sealed, plan, selectors, config, state_dir)?;
    check_expiry(sealed, now)
}

/// Verify a seal using the system clock.
pub fn verify(
    sealed: &PlanSeal,
    plan: &ExecutionPlan,
    selectors: &PlanSelectors,
    config: &ForjarConfig,
    state_dir: &Path,
) -> Result<(), SealError> {
    verify_at(sealed, plan, selectors, config, state_dir, now_unix())
}

fn check_version(sealed: &PlanSeal) -> Result<(), SealError> {
    if sealed.version != SEAL_VERSION {
        return Err(SealError::PlanVersionUnknown(sealed.version.clone()));
    }
    Ok(())
}

/// The action counters MUST partition the change list.
///
/// `planner::plan_with_probes` guarantees this by construction (it even
/// `debug_assert!`s it), so any plan document where it does not hold has been
/// edited — including one whose editor recomputed the seal, since the partition
/// is a property of the body rather than of any hash over it.
///
/// # What it does NOT catch (Refs #358)
///
/// Exactly what it says: a plan that claims "0 changes" WHILE LISTING SEVERAL.
/// An editor who also EMPTIES the change list passes trivially — `0/0/0/0`
/// partitions an empty list — and so does one who relabels every listed change
/// to `NoOp` and moves the count to `unchanged`. Neither of those is a
/// structural contradiction, so nothing here can see it; both are refused by
/// `cli::apply_from_plan::check_plan_still_holds`, which re-plans from live
/// inputs and compares.
pub fn check_body_partition(plan: &ExecutionPlan) -> Result<(), SealError> {
    let tally = |want: PlanAction| plan.changes.iter().filter(|c| c.action == want).count() as u32;
    let expected = [
        ("to_create", plan.to_create, tally(PlanAction::Create)),
        ("to_update", plan.to_update, tally(PlanAction::Update)),
        ("to_destroy", plan.to_destroy, tally(PlanAction::Destroy)),
        ("unchanged", plan.unchanged, tally(PlanAction::NoOp)),
    ];
    for (field, stated, actual) in expected {
        if stated != actual {
            return Err(SealError::PlanMalformed(format!(
                "plan body is inconsistent: '{field}' says {stated} but the change \
                 list contains {actual} — the counters do not partition the changes"
            )));
        }
    }
    Ok(())
}

fn check_self_consistency(sealed: &PlanSeal) -> Result<(), SealError> {
    let recomposed = digest::compose(
        &sealed.config_hash,
        &sealed.state_hash,
        &sealed.diff_hash,
        sealed.sealed_at_unix,
        sealed.ttl_secs,
    );
    if recomposed != sealed.seal {
        return Err(SealError::PlanHashMismatch {
            leg: Leg::Seal,
            expected: sealed.seal.clone(),
            actual: recomposed,
        });
    }
    Ok(())
}

fn check_legs(
    sealed: &PlanSeal,
    plan: &ExecutionPlan,
    selectors: &PlanSelectors,
    config: &ForjarConfig,
    state_dir: &Path,
) -> Result<(), SealError> {
    let malformed = |e: String| SealError::PlanMalformed(e);
    compare(
        Leg::Config,
        &sealed.config_hash,
        digest::config_leg(config).map_err(malformed)?,
    )?;
    compare(
        Leg::State,
        &sealed.state_hash,
        digest::state_leg(config, state_dir).map_err(malformed)?,
    )?;
    compare(
        Leg::Diff,
        &sealed.diff_hash,
        digest::diff_leg(plan, selectors).map_err(malformed)?,
    )
}

fn compare(leg: Leg, expected: &str, actual: String) -> Result<(), SealError> {
    if expected != actual {
        return Err(SealError::PlanHashMismatch {
            leg,
            expected: expected.to_string(),
            actual,
        });
    }
    Ok(())
}

/// Expiry is INCLUSIVE of `sealed_at + ttl`: a plan is still valid at exactly
/// its expiry second, and refused from the next one on.
fn check_expiry(sealed: &PlanSeal, now: u64) -> Result<(), SealError> {
    if sealed.ttl_secs == TTL_NO_EXPIRY {
        return Ok(());
    }
    let expires_at = sealed.sealed_at_unix.saturating_add(sealed.ttl_secs);
    if now > expires_at {
        return Err(SealError::PlanExpired {
            sealed_at: sealed.sealed_at_unix,
            expires_at,
            now,
        });
    }
    Ok(())
}
