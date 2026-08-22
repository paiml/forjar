//! FJ-038: `forjar dogfood` — exercise generated artifacts against reality.
//!
//! # Why this exists
//!
//! Between 2026-08-15 and 08-16, forjar shipped three releases in a row, each
//! fixing a bug the previous one introduced. Every one had passed 12,904 unit
//! tests, a five-gate clean room, and a full 19-check CI run:
//!
//!   1.13.0  `backup_sync` read rclone's `--combined` status characters
//!           backwards, so files that were NOT backed up left the coverage
//!           denominator and a backup missing data reported HIGHER coverage
//!           than one that had all of it.
//!   1.13.2  `disk_budget` required both `CACHEDIR.TAG` and `.rustc_info.json`
//!           on a cargo target dir. Measured across a real 4.6 TB tree: zero of
//!           sixteen marker-bearing directories carried the pair. The reaper
//!           matched nothing and reported `health=inert` at 94% used.
//!
//! Both are the same failure, and it is not one more tests would have caught.
//! The test fixtures were written by the same person as the code, so they
//! encoded the same assumption and faithfully confirmed it. The rclone stub
//! emitted whichever characters the author believed in. The cargo fixture had
//! both markers because the author believed both were present.
//!
//! **A test you author cannot falsify a premise you hold.** Only the real tool
//! and real data can. That is what this module runs.
//!
//! # Why it is exhaustive
//!
//! [`coverage`] matches every [`ResourceType`] variant with no wildcard arm, so
//! adding a resource type **fails to compile** until its dogfood status is
//! declared. The alternative — a list somebody remembers to extend — is what
//! let `dogfood-use` cover only `file` resources while two new types shipped
//! with no coverage at all, and still return GO.
//!
//! An honest `NotApplicable` with a reason is acceptable. Silence is not.

use crate::core::types::ResourceType;

mod exercises;
#[cfg(test)]
mod tests;

/// Whether a resource type is exercised against reality, and if not, why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Coverage {
    /// Exercised against the real external tool and/or real on-disk shapes.
    Exercised,
    /// Cannot be exercised on a developer machine. Must carry a real reason —
    /// this is a debt statement, not a way to opt out.
    NotApplicable(&'static str),
}

/// Outcome of one dogfood exercise.
#[derive(Debug)]
pub struct Outcome {
    /// Resource type exercised.
    pub resource_type: String,
    /// Whether it passed.
    pub passed: bool,
    /// Human-readable detail — what was proven, or what broke.
    pub detail: String,
}

/// Declare how each resource type is dogfooded.
///
/// EXHAUSTIVE ON PURPOSE. Do not add a `_ =>` arm: the compiler refusing to
/// build a new resource type until this is answered is the entire mechanism.
pub const fn coverage(t: &ResourceType) -> Coverage {
    match t {
        // Generated shell whose correctness depends on an external tool's
        // output format, or on real on-disk layouts. These are the ones that
        // shipped broken, and they are exercised for real.
        ResourceType::DiskBudget | ResourceType::BackupSync | ResourceType::NasArchive => {
            Coverage::Exercised
        }

        // Emit shell, but against interfaces that are stable, self-evident from
        // the emitted text, and already pinned by falsification suites.
        ResourceType::File | ResourceType::Cron => Coverage::Exercised,

        // Require privileged, host-mutating or networked state that a dogfood
        // run must not touch. Covered by the clean room and by convergence
        // tests instead.
        ResourceType::Package => Coverage::NotApplicable("mutates system packages"),
        ResourceType::Service => Coverage::NotApplicable("mutates systemd on the host"),
        ResourceType::Mount => Coverage::NotApplicable("requires mount privileges"),
        ResourceType::User => Coverage::NotApplicable("mutates system accounts"),
        ResourceType::Docker => Coverage::NotApplicable("requires a docker daemon"),
        ResourceType::Pepita => Coverage::NotApplicable("requires namespace privileges"),
        ResourceType::Network => Coverage::NotApplicable("mutates firewall rules"),
        ResourceType::Gpu => Coverage::NotApplicable("requires GPU hardware"),
        ResourceType::Model => Coverage::NotApplicable("downloads multi-GB weights"),
        ResourceType::OverlayInterface => Coverage::NotApplicable("binds a host IP"),
        ResourceType::GithubRelease => {
            Coverage::NotApplicable("network + writes to /usr/local/bin")
        }
        ResourceType::Image | ResourceType::WasmBundle => {
            Coverage::NotApplicable("requires an OCI/wasm toolchain")
        }
        ResourceType::Build => Coverage::NotApplicable("cross-compiles to another machine"),
        ResourceType::Task => Coverage::NotApplicable("runs arbitrary user commands"),
        ResourceType::Recipe => Coverage::NotApplicable("expanded before codegen; not a leaf type"),
    }
}

/// Every resource type, for exhaustive iteration.
pub const ALL_TYPES: &[ResourceType] = &[
    ResourceType::Package,
    ResourceType::File,
    ResourceType::Service,
    ResourceType::Mount,
    ResourceType::User,
    ResourceType::Docker,
    ResourceType::Pepita,
    ResourceType::Network,
    ResourceType::Cron,
    ResourceType::Recipe,
    ResourceType::Model,
    ResourceType::Gpu,
    ResourceType::Task,
    ResourceType::WasmBundle,
    ResourceType::Image,
    ResourceType::Build,
    ResourceType::GithubRelease,
    ResourceType::OverlayInterface,
    ResourceType::DiskBudget,
    ResourceType::BackupSync,
    ResourceType::NasArchive,
];

/// Run every exercised resource type against reality.
///
/// # Errors
///
/// Returns `Err` with the failing outcomes when any exercise fails, or when a
/// required external tool is missing — a dogfood run that silently skips the
/// real tool proves nothing and must not report success.
pub fn run() -> Result<Vec<Outcome>, Vec<Outcome>> {
    let mut outcomes = Vec::new();
    for t in ALL_TYPES {
        if coverage(t) != Coverage::Exercised {
            continue;
        }
        outcomes.push(exercises::run_for(t));
    }
    if outcomes.iter().all(|o| o.passed) {
        Ok(outcomes)
    } else {
        Err(outcomes)
    }
}

/// Types declared `NotApplicable`, with reasons — printed so the debt is
/// visible on every run rather than buried in source.
pub fn not_applicable() -> Vec<(String, &'static str)> {
    ALL_TYPES
        .iter()
        .filter_map(|t| match coverage(t) {
            Coverage::NotApplicable(why) => Some((t.to_string(), why)),
            Coverage::Exercised => None,
        })
        .collect()
}

/// Run the gate and report it, for the CLI.
///
/// Lives here rather than in the dispatcher: presentation of a feature belongs
/// with the feature, and threading it through `dispatch_misc` pushed that file
/// from A- to B+.
///
/// # Errors
///
/// Returns `Err` when any exercise failed.
pub fn report(json: bool) -> Result<(), String> {
    let result = run();
    let outcomes = match &result {
        Ok(o) | Err(o) => o,
    };
    if json {
        let items: Vec<String> = outcomes
            .iter()
            .map(|o| {
                format!(
                    "{{\"type\":\"{}\",\"passed\":{},\"detail\":{:?}}}",
                    o.resource_type, o.passed, o.detail
                )
            })
            .collect();
        println!("{{\"dogfood\":[{}]}}", items.join(","));
    } else {
        for o in outcomes {
            let mark = if o.passed { "PASS" } else { "FAIL" };
            println!("{mark}  {:<18} {}", o.resource_type, o.detail);
        }
        let debt = not_applicable();
        if !debt.is_empty() {
            println!("\nnot exercised ({} types):", debt.len());
            for (name, why) in debt {
                println!("  {name:<18} {why}");
            }
        }
    }
    match result {
        Ok(_) => Ok(()),
        Err(o) => Err(format!(
            "{} dogfood exercise(s) failed",
            o.iter().filter(|x| !x.passed).count()
        )),
    }
}
