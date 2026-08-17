//! GH-247: regenerate a task's outputs into a scratch tree and compare, without
//! ever writing the declared output path.
//!
//! Every regeneration path forjar had before this was a restore-in-place, so
//! there was no way to ask "does this artifact still reproduce?" without risking
//! the artifact. That matters most for expensive, human-corrected outputs — an
//! LLM-filled SVG, a corrected `.srt`, a rendered mp4 — where overwriting on
//! mismatch destroys the very thing you were checking.
//!
//! The hard requirement is the negative one: **on match or mismatch, the
//! declared output path is never written.** It is enforced structurally rather
//! than by discipline — the command runs with its working directory set to a
//! scratch copy, so a relative artifact path resolves inside the scratch tree,
//! and the real tree is never the command's cwd.
//!
//! What this deliberately does NOT claim: that the task declared all its inputs.
//! A task reading an undeclared ambient file (GH-244) may reproduce here and
//! still be stale in reality. This answers "did the recorded output come from
//! re-running this recipe", which is the question `staleness_reason` cannot
//! answer at all.

use crate::core::task::hash_outputs_in;
use crate::core::types::Resource;
use std::path::{Path, PathBuf};

/// Why a resource could not be verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// Not a task, or no command to re-run.
    NoCommand,
    /// Nothing declared to compare.
    NoOutputArtifacts,
    /// No recorded hash to compare against — apply has never run.
    NoRecordedHash,
    /// `working_dir` is absent or does not exist.
    WorkingDirUnavailable,
}

impl SkipReason {
    /// A short, stable token for machine-readable output.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoCommand => "no-command",
            Self::NoOutputArtifacts => "no-output-artifacts",
            Self::NoRecordedHash => "no-recorded-hash",
            Self::WorkingDirUnavailable => "working-dir-unavailable",
        }
    }
}

/// The verdict for one resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Regenerated outputs hash identically to the recorded ones.
    Reproduced,
    /// Regenerated outputs differ from the recorded ones.
    Diverged {
        /// Hash recorded by the last apply.
        recorded: String,
        /// Hash of the freshly regenerated outputs.
        regenerated: Option<String>,
    },
    /// The command failed; nothing can be concluded about reproducibility.
    CommandFailed {
        /// Exit status text.
        status: String,
    },
    /// Not applicable.
    Skipped(SkipReason),
}

impl Verdict {
    /// A short, stable token for machine-readable output.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Reproduced => "reproduced",
            Self::Diverged { .. } => "diverged",
            Self::CommandFailed { .. } => "command-failed",
            Self::Skipped(_) => "skipped",
        }
    }

    /// Whether this verdict should fail a CI gate.
    ///
    /// A skip is not a failure — it means the question did not apply. A command
    /// failure IS one: an artifact whose recipe no longer runs is not
    /// reproducible, whatever the stored hash says.
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Diverged { .. } | Self::CommandFailed { .. })
    }
}

/// Outcome for a single resource.
#[derive(Debug, Clone)]
pub struct VerifyOutcome {
    /// Resource that was verified.
    pub resource_id: String,
    /// What was concluded.
    pub verdict: Verdict,
}

/// Decide whether a resource can be verified at all.
///
/// Split out so the preconditions are testable without running anything, and so
/// the reasons are enumerable rather than a bare `None`.
#[must_use]
pub fn verifiability(resource: &Resource, recorded_hash: Option<&str>) -> Option<SkipReason> {
    if resource.command.is_none() {
        return Some(SkipReason::NoCommand);
    }
    if resource.output_artifacts.is_empty() {
        return Some(SkipReason::NoOutputArtifacts);
    }
    if recorded_hash.is_none() {
        return Some(SkipReason::NoRecordedHash);
    }
    match resource.working_dir.as_deref() {
        Some(d) if Path::new(d).is_dir() => None,
        _ => Some(SkipReason::WorkingDirUnavailable),
    }
}

/// Copy a directory tree, skipping paths that would defeat the purpose.
///
/// `.git` is skipped because it is large and never an input to an artifact
/// build. The declared output artifacts are skipped so the regenerated run
/// cannot be satisfied by the previous run's leftovers — otherwise a recipe
/// that silently no-ops on an existing file would "reproduce" perfectly.
fn copy_tree_excluding(src: &Path, dst: &Path, exclude: &[PathBuf]) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("scratch mkdir {}: {e}", dst.display()))?;
    let entries = std::fs::read_dir(src).map_err(|e| format!("read_dir {}: {e}", src.display()))?;
    for entry in entries.filter_map(Result::ok) {
        let from = entry.path();
        if from.file_name().is_some_and(|n| n == ".git") {
            continue;
        }
        if exclude.iter().any(|e| e == &from) {
            continue;
        }
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_tree_excluding(&from, &to, exclude)?;
        } else {
            std::fs::copy(&from, &to)
                .map_err(|e| format!("copy {} -> {}: {e}", from.display(), to.display()))?;
        }
    }
    Ok(())
}

/// Absolute paths of the declared artifacts under `base`.
fn artifact_paths(resource: &Resource, base: &Path) -> Vec<PathBuf> {
    resource
        .output_artifacts
        .iter()
        .map(|a| {
            let p = Path::new(a);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                base.join(p)
            }
        })
        .collect()
}

/// Regenerate `resource` in a scratch copy and compare against `recorded_hash`.
///
/// `resource_id` is passed separately because resources are keyed by id in the
/// config map rather than carrying it as a field.
///
/// `scratch_root` must be a directory the caller owns and is willing to have
/// populated; the caller is responsible for removing it. Nothing under the
/// resource's own `working_dir` is written.
pub fn verify_resource(
    resource_id: &str,
    resource: &Resource,
    recorded_hash: Option<&str>,
    scratch_root: &Path,
) -> VerifyOutcome {
    let id = resource_id.to_string();
    if let Some(reason) = verifiability(resource, recorded_hash) {
        return VerifyOutcome {
            resource_id: id,
            verdict: Verdict::Skipped(reason),
        };
    }

    // Both unwraps are discharged by `verifiability` above.
    let command = resource.command.as_deref().unwrap_or_default();
    let recorded = recorded_hash.unwrap_or_default();
    let work = PathBuf::from(resource.working_dir.as_deref().unwrap_or_default());

    let exclude = artifact_paths(resource, &work);
    if let Err(e) = copy_tree_excluding(&work, scratch_root, &exclude) {
        return VerifyOutcome {
            resource_id: id,
            verdict: Verdict::CommandFailed { status: e },
        };
    }

    let out = std::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir(scratch_root)
        .output();

    let out = match out {
        Ok(o) => o,
        Err(e) => {
            return VerifyOutcome {
                resource_id: id,
                verdict: Verdict::CommandFailed {
                    status: format!("spawn: {e}"),
                },
            }
        }
    };
    if !out.status.success() {
        return VerifyOutcome {
            resource_id: id,
            verdict: Verdict::CommandFailed {
                status: format!(
                    "exit {}: {}",
                    out.status,
                    String::from_utf8_lossy(&out.stderr).trim()
                ),
            },
        };
    }

    let regenerated = hash_outputs_in(&resource.output_artifacts, scratch_root).unwrap_or(None);
    let verdict = if regenerated.as_deref() == Some(recorded) {
        Verdict::Reproduced
    } else {
        Verdict::Diverged {
            recorded: recorded.to_string(),
            regenerated,
        }
    };
    VerifyOutcome {
        resource_id: id,
        verdict,
    }
}

#[cfg(test)]
#[path = "tests_verify.rs"]
mod tests_verify;
