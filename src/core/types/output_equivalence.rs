//! GH-246: how an output artifact is compared, not merely whether it changed.
//!
//! Byte-identity is exactly right for source and object files — the probe's own
//! doc makes the point that recompiling to identical bytes correctly does not
//! relink. It is wrong for producers that *cannot* reach byte-identity, and
//! wrong in the dangerous direction: an artifact keyed by a hash it can never
//! reproduce is not "uncached", it is content-addressed with the wrong key.
//!
//! The sharpest case is a human-corrected artifact — an ASR transcript a person
//! then edits. Under byte-equivalence the edit reads as staleness, so the next
//! apply regenerates the machine draft over the human's work.

use serde::{Deserialize, Serialize};

/// How to compare one declared output artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutputEquivalence {
    /// Raw bytes. The default, and correct for anything reproducible.
    #[default]
    Bytes,
    /// Excluded from the staleness predicate entirely.
    ///
    /// A missing artifact is still staleness — this says "do not key on my
    /// content", not "do not track me at all".
    None,
    /// Human-authoritative: a modified output is an IMPROVEMENT, not staleness.
    ///
    /// The producer must not overwrite it; re-running requires explicit force.
    External,
    /// Compare the stdout of a declared normaliser instead of the file bytes.
    ///
    /// Covers structural equivalence — strip a timestamp, sort a manifest,
    /// canonicalise an SVG — and keeps forjar out of the media-format business.
    Command(String),
}

impl OutputEquivalence {
    /// Whether this artifact's CONTENT participates in the staleness hash.
    ///
    /// `None` and `External` both drop out of the content hash; they differ in
    /// what a modification MEANS, which is a separate question from whether the
    /// bytes are keyed.
    #[must_use]
    pub fn contributes_content(&self) -> bool {
        matches!(self, Self::Bytes | Self::Command(_))
    }

    /// Whether the producer may overwrite this artifact on an UNFORCED apply.
    ///
    /// `external` achieves this by dropping out of the staleness hash: a human
    /// edit is not a change, so nothing re-runs. `--force` deliberately still
    /// overwrites — the issue specifies force as the escape, and a guard that
    /// blocked it would make the flag mean something different here than
    /// everywhere else in forjar.
    #[must_use]
    pub fn producer_may_overwrite(&self) -> bool {
        !matches!(self, Self::External)
    }

    /// A short, stable token for diagnostics.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bytes => "bytes",
            Self::None => "none",
            Self::External => "external",
            Self::Command(_) => "command",
        }
    }
}
