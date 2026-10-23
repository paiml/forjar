//! The architecture rule, as functions over a parsed workflow so a fixture can
//! drive it (forjar#772). `main.rs` feeds these the real workflows; the
//! falsification cases below feed them planted ones.
//!
//! The rule is per LEG. `runner_labels` flattens a matrix, so judging the
//! flattened set let one leg's `X64` vouch for a sibling that named nothing.

use crate::scan::runner_legs;
use serde_yaml_ng::Value;

pub(crate) const ARCHES: [&str; 2] = ["X64", "ARM64"];

fn is_fleet(leg: &[String]) -> bool {
    leg.iter().any(|l| l == "self-hosted")
}

fn has_arch(leg: &[String]) -> bool {
    leg.iter().any(|l| ARCHES.contains(&l.as_str()))
}

/// Self-hosted legs in `doc` that name no architecture, in jobs not listed in
/// `neutral` (`(file, job)` pairs).
pub(crate) fn unpinned_in(file: &str, doc: &Value, neutral: &[(&str, &str)]) -> Vec<String> {
    let Some(Value::Mapping(jobs)) = doc.get("jobs") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (name, job) in jobs {
        let name = name.as_str().unwrap_or("?");
        if neutral.iter().any(|(f, j)| *f == file && *j == name) {
            continue;
        }
        for leg in runner_legs(job) {
            if is_fleet(&leg) && !has_arch(&leg) {
                out.push(format!("  {file}  job `{name}`  runs-on {leg:?}"));
            }
        }
    }
    out
}

/// Entries of `neutral` that no longer hold against `docs` (`(file, doc)`): the
/// job is gone, some leg carries an arch label, or NO leg is on the fleet (it
/// moved to a hosted runner, where "chip-neutral" no longer means anything).
pub(crate) fn stale_in(docs: &[(String, Value)], neutral: &[(&str, &str)]) -> Vec<String> {
    let mut stale = Vec::new();
    for (f, j) in neutral {
        let job = docs
            .iter()
            .find(|(file, _)| file == f)
            .and_then(|(_, doc)| doc.get("jobs"))
            .and_then(|jobs| jobs.get(*j));
        let Some(job) = job else {
            stale.push(format!("  {f}  job `{j}` does not exist"));
            continue;
        };
        let legs = runner_legs(job);
        if legs.iter().any(|l| is_fleet(l) && has_arch(l)) {
            stale.push(format!("  {f}  job `{j}` carries an arch label"));
        }
        if !legs.iter().any(|l| is_fleet(l)) {
            stale.push(format!("  {f}  job `{j}` is not on a self-hosted runner"));
        }
    }
    stale
}

#[cfg(test)]
mod falsification {
    use super::*;
    use crate::scan::fixture;

    /// Hole 1: `runner_labels` flattens a matrix, so ONE leg naming `X64`
    /// vouched for every other leg. Leg `b` below is unpinned and must be found.
    #[test]
    fn a_matrix_with_one_pinned_leg_and_one_unpinned_leg_fails() {
        let doc = fixture(
            r#"
jobs:
  build:
    strategy:
      matrix:
        include:
          - runner: [self-hosted, clean-room, X64]
          - runner: [self-hosted, clean-room]
    runs-on: ${{ matrix.runner }}
"#,
        );
        let found = unpinned_in("w.yml", &doc, &[]);
        assert_eq!(found.len(), 1, "the unpinned leg was hidden: {found:?}");
        assert!(found[0].contains("build"), "{found:?}");
    }

    /// The control: a matrix whose every leg names an arch is clean, and so is
    /// one whose hosted legs are not the fleet's concern.
    #[test]
    fn a_matrix_with_every_leg_pinned_passes() {
        let doc = fixture(
            r#"
jobs:
  build:
    strategy:
      matrix:
        include:
          - runner: [self-hosted, clean-room, X64]
          - runner: [self-hosted, clean-room, ARM64]
          - runner: macos-latest
    runs-on: ${{ matrix.runner }}
"#,
        );
        assert!(unpinned_in("w.yml", &doc, &[]).is_empty());
    }

    /// Hole 2: a listed ARCH_NEUTRAL job moved to a GitHub-hosted runner carries
    /// no arch label and used to pass the staleness check.
    #[test]
    fn a_neutral_job_moved_to_a_hosted_runner_is_stale() {
        let docs = vec![(
            "w.yml".to_string(),
            fixture("jobs:\n  j:\n    runs-on: ubuntu-latest\n"),
        )];
        let stale = stale_in(&docs, &[("w.yml", "j")]);
        assert_eq!(
            stale.len(),
            1,
            "a hosted-runner job stayed listed: {stale:?}"
        );
    }

    /// Controls for the staleness check: a self-hosted neutral job is fine, and
    /// so is the shipped shape of `clippy` (fleet legs plus hosted macOS legs).
    #[test]
    fn a_self_hosted_neutral_job_and_a_mixed_matrix_are_not_stale() {
        let docs = vec![(
            "w.yml".to_string(),
            fixture(
                r#"
jobs:
  j:
    runs-on: [self-hosted, clean-room]
  m:
    strategy:
      matrix:
        include:
          - runner: [self-hosted, clean-room]
          - runner: macos-latest
    runs-on: ${{ matrix.runner }}
"#,
            ),
        )];
        assert!(stale_in(&docs, &[("w.yml", "j"), ("w.yml", "m")]).is_empty());
    }
}
