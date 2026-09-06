//! forjar#469 (PMAT-161): per-stack stamps in the global lock.
//!
//! The global lock has always been keyed by config name for its machine
//! sections, but the stamp — "which stack last applied here" — was a single
//! value. paiml/infra keeps ONE `state/` dir for six machine manifests
//! (`machines/<m>/forjar.yaml`, each with its own `name:`), so every apply
//! re-stamped the dir as a different stack and the GH-377 warning fired on all
//! of them, telling a fleet that was doing nothing wrong that `forjar undo`
//! would refuse.
//!
//! The stamp is now `stacks: {name -> StackStamp}`, written per name. What
//! makes two applies of one dir genuinely dangerous is [`stack_conflict`], the
//! one pure condition both `apply` (warns) and `undo` (refuses, phase 2) ask,
//! so they cannot disagree about what "wrong stack" means:
//!
//! * the SAME name applied from a DIFFERENT config file — GH-377's case; and
//! * a machine another stack owns in this dir, because generations and
//!   per-machine locks are keyed by machine name alone, so two stacks sharing
//!   a machine name overwrite each other's history.

mod rename;

use crate::core::types::{GlobalLock, MachineSummary};
use crate::tripwire::eventlog::now_iso8601;
use indexmap::IndexMap;
use rename::{next_stamp, retire_renamed};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// forjar#469: one config's record inside `GlobalLock::stacks`.
///
/// Before this, the stamp was a single `name`, so a dir shared by six machine
/// manifests (paiml/infra) was re-stamped by every apply and the GH-377
/// warning fired on all of them — each having touched only its own sections.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StackStamp {
    /// The config (`-f`) this stack was last applied from, stored RELATIVE to
    /// the state dir when the two sit in one tree (`../machines/mini/forjar.yaml`),
    /// and as the absolute canonical path when they share no prefix. The
    /// relative form is what makes the identity portable: the same layout in
    /// another checkout — a colleague's clone, a CI workspace — is the same
    /// stack, not a "wrong stack" warning on first apply.
    ///
    /// `None` for a stamp migrated from a 1.0 lock, which recorded no file.
    /// Such a stamp matches ANY file on the first post-upgrade apply and is
    /// pinned by it: one apply's worth of window in which a genuinely wrong
    /// `-f` goes unwarned, deliberately spent so the upgrade itself is quiet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,

    /// When this stack last applied.
    pub last_apply: String,

    /// Generator version that wrote this stamp.
    pub generator: String,

    /// Machine names this stack's last apply wrote — the ownership record
    /// [`stack_conflict`] reads. A machine drops out (is released) when this
    /// stack next applies without it.
    #[serde(default)]
    pub machines: Vec<String>,

    /// Output keys this stack owns in `GlobalLock::outputs`, so a second
    /// stack's apply merges into that map instead of replacing it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,
}

/// Schema version this forjar writes into the global lock.
pub const SCHEMA_CURRENT: &str = "1.1";

/// Schema versions this forjar can read. Anything else fails closed.
///
/// `"1"` is the same single-stamp format as `"1.0"` under a looser spelling
/// that predates the version string being normalised; it migrates like 1.0.
pub const SCHEMA_KNOWN: [&str; 3] = ["1", "1.0", SCHEMA_CURRENT];

impl GlobalLock {
    /// The stamp for one config name, if this dir has ever recorded it.
    ///
    /// Read the stamps through this rather than indexing `stacks`: the map is
    /// reconstructed from the legacy `name` on load, and a caller that indexed
    /// directly would see an empty map for any lock built in memory.
    #[must_use]
    pub fn stamp_for(&self, name: &str) -> Option<&StackStamp> {
        self.stacks.get(name)
    }
}

/// Why this apply and this state dir may not belong together.
///
/// Both variants mean the same thing operationally — one of `-f` /
/// `--state-dir` points at the wrong stack — so both carry the same tail, and
/// `undo` (phase 2) refuses on either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackConflict {
    /// This stack name was last applied from a different config file.
    OtherFile {
        /// The file the stamp recorded.
        old: String,
    },
    /// A machine this apply writes is recorded as another stack's.
    MachineOwnedBy {
        /// The machine both stacks name.
        machine: String,
        /// The stack that wrote it last.
        stack: String,
    },
}

impl fmt::Display for StackConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OtherFile { old } => write!(
                f,
                "was last applied from {old}; this apply re-stamps it from a different -f. \
                 If that is not a rename, one of -f/--state-dir points at the wrong stack \
                 (`forjar undo` refuses this combination)."
            ),
            Self::MachineOwnedBy { machine, stack } => write!(
                f,
                "would write machine '{machine}', which stack '{stack}' owns in this state dir; \
                 generations and machine locks are keyed by machine name alone, so the two \
                 stacks would overwrite each other. If that is not intended, one of \
                 -f/--state-dir points at the wrong stack (`forjar undo` refuses this \
                 combination)."
            ),
        }
    }
}

/// The stacks this dir has stamps for, in the order they were recorded.
///
/// The one place "how many stacks share this dir, and which" is answered, so
/// the apply-time warning, the `undo` owner check and the restore refusal
/// cannot disagree about it.
#[must_use]
pub fn stack_names(lock: &GlobalLock) -> Vec<String> {
    lock.stacks.keys().cloned().collect()
}

/// PMAT-161 (#469): the refusal a WHOLE-DIR restore owes a state dir that more
/// than one stack has applied to — `None` when the dir is one stack's, which is
/// the ordinary case and is unchanged.
///
/// Pure, and shared: the caller supplies the lock it loaded, the dir as it
/// wants it printed and how the target is named, so `undo`, `undo --resume` and
/// `rollback --generation` refuse in the same words. Restoring is whole-dir
/// while generations are numbered per state dir, so a restore in a shared dir
/// reverts every stack in it; scoping that is PMAT-162.
#[must_use]
pub fn multi_stack_restore_refusal(
    lock: &GlobalLock,
    state_dir: &str,
    target: &str,
) -> Option<String> {
    let names = stack_names(lock);
    if names.len() <= 1 {
        return None;
    }
    let quoted: Vec<String> = names.iter().map(|n| format!("'{n}'")).collect();
    Some(format!(
        "refusing to restore {target}: state dir {state_dir} holds {} stacks ({}); \
         a restore is whole-dir, so it would revert every one of them. Stack-scoped \
         restore is PMAT-162; until it lands, give this stack a --state-dir of its own.",
        names.len(),
        quoted.join(", "),
    ))
}

/// Reject a global lock whose schema this binary does not understand.
///
/// The fail-closed half of forjar#469: a 1.2 (or 2.0) state dir written by a
/// newer forjar must NOT be reinterpreted by an older one, because serde would
/// silently drop whatever it added. The message names the schema it found.
pub fn check_schema(schema: &str, path: &Path) -> Result<(), String> {
    if SCHEMA_KNOWN.contains(&schema) {
        return Ok(());
    }
    Err(format!(
        "global lock {} has schema '{}', which forjar {} does not understand \
         (it reads {}). Upgrade forjar; do not apply against this state dir.",
        path.display(),
        schema,
        env!("CARGO_PKG_VERSION"),
        SCHEMA_KNOWN.join(" / "),
    ))
}

/// Migrate a just-loaded lock to the current schema, in memory.
///
/// A 1.0 file carries one stamp in `name`; that stamp becomes the entry for
/// its own name, with `file: None` (1.0 recorded no config path) and the
/// machines the dir already has. Nothing is dropped — `name`, `machines` and
/// `outputs` are untouched — and the first `save_global_lock` persists it.
///
/// Idempotent: a lock that already has stamps is left alone, so loading twice
/// (or loading what this wrote) changes nothing.
pub fn migrate(lock: &mut GlobalLock) {
    if lock.stacks.is_empty() && !lock.name.is_empty() {
        let stamp = StackStamp {
            file: None,
            last_apply: lock.last_apply.clone(),
            generator: lock.generator.clone(),
            machines: lock.machines.keys().cloned().collect(),
            outputs: lock.outputs.keys().cloned().collect(),
        };
        lock.stacks.insert(lock.name.clone(), stamp);
    }
    lock.schema = SCHEMA_CURRENT.to_string();
}

/// Canonicalise a path, falling back to the path as given.
///
/// The config may have moved since the apply that recorded it; comparing two
/// uncanonicalised paths still beats claiming they differ.
fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The path relative to `base`, or `None` when the two share no prefix.
fn relative_to(base: &Path, target: &Path) -> Option<PathBuf> {
    let base: Vec<_> = base.components().collect();
    let target: Vec<_> = target.components().collect();
    let shared = base
        .iter()
        .zip(target.iter())
        .take_while(|(a, b)| a == b)
        .count();
    // One shared component is the filesystem root: no common tree.
    if shared <= 1 {
        return None;
    }
    let mut relative = PathBuf::new();
    for _ in shared..base.len() {
        relative.push("..");
    }
    for component in &target[shared..] {
        relative.push(component);
    }
    Some(relative)
}

/// How a config path is stored in a stamp: relative to the state dir when both
/// sit in one tree, else the absolute canonical path.
#[must_use]
pub fn stamped_config_file(state_dir: &Path, file: Option<&Path>) -> Option<String> {
    let config = canonical(file?);
    let base = canonical(state_dir);
    let stored = relative_to(&base, &config).unwrap_or(config);
    Some(stored.display().to_string())
}

/// Does a recorded stamp path name the config now being applied?
///
/// A recorded RELATIVE path is compared by the part it actually names (its
/// `..` prefix is the state dir's business, not the config's), which is what
/// makes the same layout in another checkout the same stack.
fn same_config_file(recorded: &str, config_file: &Path) -> bool {
    let recorded = Path::new(recorded);
    let current = canonical(config_file);
    if recorded == current {
        return true;
    }
    let named: PathBuf = recorded
        .components()
        .skip_while(|c| matches!(c, std::path::Component::ParentDir))
        .collect();
    !named.as_os_str().is_empty() && recorded.is_relative() && current.ends_with(&named)
}

/// The stack in this dir that owns `machine`, other than `name`.
///
/// A stamp that records the very config file now being applied is not another
/// stack: it is THIS stack under its old name. Renaming a stack (edit `name:`,
/// same `-f`, same machines) leaves the old name's entry behind still owning
/// every machine, so reading that ghost as a second stack made the documented
/// rename remedy self-defeating — the one apply that re-stamps the dir is what
/// creates the entry that would then own the machine for ever, and `undo`
/// would refuse a single stack against its own state dir.
///
/// Positive evidence only: the ghost must RECORD a file, and it must be this
/// file. A stamp with no file (migrated from 1.0) is not proof of a rename, so
/// the machine collision stands.
fn machine_owner(
    lock: &GlobalLock,
    name: &str,
    machine: &str,
    config_file: Option<&Path>,
) -> Option<String> {
    lock.stacks
        .iter()
        .find(|(stack, stamp)| {
            stack.as_str() != name
                && stamp.machines.iter().any(|m| m == machine)
                && !records_config_file(stamp, config_file)
        })
        .map(|(stack, _)| stack.clone())
}

/// Does this stamp record the config file now being applied?
fn records_config_file(stamp: &StackStamp, config_file: Option<&Path>) -> bool {
    let Some(current) = config_file else {
        return false;
    };
    stamp
        .file
        .as_deref()
        .is_some_and(|recorded| same_config_file(recorded, current))
}

/// GH-377 + forjar#469: is this apply about to write over another stack's work?
///
/// `None` is the supported many-stacks-one-state-dir layout: a different NAME
/// with its own machines. `Some` is a genuine hazard — the same name from a
/// different `-f`, or a machine another stack owns. A stamp with no recorded
/// file (migrated from 1.0) and a caller with no `-f` both mean "origin
/// unknown", which is not evidence of a mismatch.
#[must_use]
pub fn stack_conflict(
    lock: &GlobalLock,
    name: &str,
    config_file: Option<&Path>,
    machines: &[String],
) -> Option<StackConflict> {
    if let Some(old) = stack_written_from_other_file(lock, name, config_file) {
        return Some(StackConflict::OtherFile { old });
    }
    machines.iter().find_map(|machine| {
        machine_owner(lock, name, machine, config_file).map(|stack| StackConflict::MachineOwnedBy {
            machine: machine.clone(),
            stack,
        })
    })
}

/// The GH-377 half of [`stack_conflict`]: this name was last applied from a
/// different config file. Returns that file.
#[must_use]
pub fn stack_written_from_other_file(
    lock: &GlobalLock,
    name: &str,
    config_file: Option<&Path>,
) -> Option<String> {
    let recorded = lock.stamp_for(name)?.file.as_ref()?;
    let current = config_file?;
    (!same_config_file(recorded, current)).then(|| recorded.clone())
}

/// Write one stack's stamp plus the machine summaries from its apply.
///
/// Only `stacks[config_name]` is touched — plus the entry this apply RETIRES,
/// when the same file was last applied under another name (a rename). Every
/// other stack's record is left exactly as it was, and the top-level
/// `name`/`last_apply`/`generator` still track the stack that applied LAST.
pub fn apply_stamp(
    lock: &mut GlobalLock,
    state_dir: &Path,
    config: (&str, Option<&Path>), // (config name, -f path)
    machine_results: &[(String, usize, usize, usize)], // (name, total, converged, failed)
) {
    let (config_name, config_file) = config;
    let now = now_iso8601();
    let generator = format!("forjar {}", env!("CARGO_PKG_VERSION"));

    lock.schema = SCHEMA_CURRENT.to_string();
    lock.name = config_name.to_string();
    lock.last_apply.clone_from(&now);
    lock.generator.clone_from(&generator);

    let retired = retire_renamed(lock, state_dir, config);
    let stamp = next_stamp(
        lock.stamp_for(config_name),
        retired,
        stamped_config_file(state_dir, config_file),
        machine_results.iter().map(|(m, ..)| m.clone()).collect(),
        (&now, &generator),
    );
    lock.stacks.insert(config_name.to_string(), stamp);

    for (name, total, converged, failed) in machine_results {
        lock.machines.insert(
            name.clone(),
            MachineSummary {
                resources: *total,
                converged: *converged,
                failed: *failed,
                last_apply: now.clone(),
            },
        );
    }
}

/// Merge one stack's outputs into the shared `outputs` map.
///
/// FJ-1260 wrote `lock.outputs` wholesale, which in a shared state dir meant
/// the second stack's apply deleted the first stack's outputs — and with them
/// every `{{stack.*}}` reference pointing at that stack. Each stamp now
/// records the keys it owns: those are withdrawn, this apply's keys inserted,
/// and every other stack's keys left alone.
pub fn merge_outputs(lock: &mut GlobalLock, config_name: &str, outputs: &IndexMap<String, String>) {
    for key in owned_output_keys(lock, config_name) {
        lock.outputs.shift_remove(&key);
    }
    for (key, value) in outputs {
        lock.outputs.insert(key.clone(), value.clone());
    }
    let owned: Vec<String> = outputs.keys().cloned().collect();
    match lock.stacks.get_mut(config_name) {
        Some(stamp) => stamp.outputs = owned,
        None => {
            lock.stacks.insert(
                config_name.to_string(),
                StackStamp {
                    file: None,
                    last_apply: now_iso8601(),
                    generator: format!("forjar {}", env!("CARGO_PKG_VERSION")),
                    machines: Vec::new(),
                    outputs: owned,
                },
            );
        }
    }
}

/// Output keys this stack claimed on its last write.
fn owned_output_keys(lock: &GlobalLock, config_name: &str) -> Vec<String> {
    lock.stamp_for(config_name)
        .map(|stamp| stamp.outputs.clone())
        .unwrap_or_default()
}
