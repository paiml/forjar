//! FJ-1300: Store path derivation — content-addressed paths from input hashes.
//!
//! Every store entry lives under `STORE_BASE/<hash>` where the hash is
//! deterministically computed from the recipe hash, input hashes, architecture,
//! and provider.  This mirrors the Nix store model (`/nix/store/<hash>-name`).

use crate::tripwire::hasher::composite_hash;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// System-wide store base, used when forjar has the privileges to own it.
///
/// This is the *default*, not the answer — read [`store_root`] instead. It
/// remains public because it names the canonical system location that packaging
/// and docs refer to.
pub const STORE_BASE: &str = "/var/lib/forjar/store";

/// Environment variable that overrides the store root.
pub const STORE_ENV: &str = "FORJAR_STORE";

/// Resolve the store root for this process.
///
/// GH-239: [`STORE_BASE`] was a compile-time constant with no override, and
/// `/var/lib` is root-owned on every mainstream distribution. An unprivileged
/// user could not create the store and could not point forjar anywhere else, so
/// `forjar store list` and `forjar store gc` failed with a bare
/// `No such file or directory` for every non-root caller — including CI and
/// library consumers.
///
/// Precedence:
/// 1. `$FORJAR_STORE` — an explicit choice always wins, even if unwritable, so
///    a misconfiguration surfaces as an error on the path the operator named
///    rather than being silently redirected somewhere else.
/// 2. [`STORE_BASE`], when this process could actually write it — the system
///    install, which must keep working exactly as before for root and for
///    packaged deployments.
/// 3. `$XDG_DATA_HOME/forjar/store`, else `~/.local/share/forjar/store` — the
///    per-user store.
/// 4. [`STORE_BASE`] as a last resort, so that with no HOME and no privileges
///    the error names the canonical system path.
///
/// Resolved once per process: this is called from path derivation, and probing
/// the filesystem on every call would make path construction non-deterministic
/// with respect to concurrent `mkdir`s.
pub fn store_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(resolve_store_root)
}

fn resolve_store_root() -> PathBuf {
    if let Some(explicit) = std::env::var_os(STORE_ENV) {
        if !explicit.is_empty() {
            return PathBuf::from(explicit);
        }
    }

    if is_writable_by_us(Path::new(STORE_BASE)) {
        return PathBuf::from(STORE_BASE);
    }

    if let Some(data) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(data).join("forjar").join("store");
    }
    if let Some(home) = std::env::var_os("HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("forjar")
            .join("store");
    }

    PathBuf::from(STORE_BASE)
}

/// Can this process write into `dir` (creating it if needed)?
///
/// Probes rather than inspecting mode bits: ownership, supplementary groups,
/// ACLs and read-only mounts all decide this, and a permission-bit reading gets
/// at most the first of them right. The probe file is removed immediately, and
/// nothing is created that would not have been created by the first store write.
fn is_writable_by_us(dir: &Path) -> bool {
    if !dir.is_dir() {
        // Not created yet. Writable iff we could create it — which is decided
        // by the nearest ancestor that does exist.
        return dir
            .ancestors()
            .skip(1)
            .find(|p| p.is_dir())
            .is_some_and(probe_write);
    }
    probe_write(dir)
}

fn probe_write(dir: &Path) -> bool {
    let probe = dir.join(format!(".forjar-write-probe-{}", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Compute a deterministic store hash from recipe inputs.
///
/// The hash is computed from sorted input components so that identical
/// inputs always produce the same store path regardless of argument order.
pub fn store_path(recipe_hash: &str, input_hashes: &[&str], arch: &str, provider: &str) -> String {
    let mut components: Vec<&str> = Vec::with_capacity(input_hashes.len() + 3);
    components.push(recipe_hash);
    let mut sorted_inputs: Vec<&str> = input_hashes.to_vec();
    sorted_inputs.sort();
    components.extend(sorted_inputs);
    components.push(arch);
    components.push(provider);
    composite_hash(&components)
}

/// Build the full store entry path from a store hash.
///
/// Returns `<store_root()>/<hash>` (stripping the `blake3:` prefix). See
/// [`store_root`] for how the base is chosen — it is no longer unconditionally
/// [`STORE_BASE`].
pub fn store_entry_path(store_hash: &str) -> String {
    let hash_hex = store_hash.strip_prefix("blake3:").unwrap_or(store_hash);
    format!("{}/{hash_hex}", store_root().display())
}
