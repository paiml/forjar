//! One way to put a binary somewhere: stage a sibling, then `rename(2)`.
//!
//! forjar had three, and two of them were wrong.
//!
//! # The refusals
//!
//! `cp` opens the destination in place (`O_WRONLY|O_TRUNC`), which the kernel
//! and coreutils both refuse for the two states a provisioning tool most needs
//! to repair:
//!
//! - **`ETXTBSY` — "Text file busy".** Raised for a file that is currently
//!   being executed. It is a property of the INODE, not of the directory
//!   entry, so replacing the entry is legal while opening the inode for write
//!   is not. This is why a self-applying host could not manage its own tools:
//!   paiml/infra's lambda-labs box left forjar undeclared for exactly this
//!   reason and drifted to 1.20.1 while the fleet ran 1.21.x. It is not
//!   confined to forjar-updating-forjar — the same provider installs `rclone`,
//!   `age` and `sops` into /usr/local/bin, and a running `rclone` is enough.
//!
//! - **"cp: not writing through dangling symlink".** coreutils refuses to
//!   create the target of a symlink that points at nothing. That is precisely
//!   the wreckage forjar exists to repair: a CI cache-prune deletes the real
//!   files in a shared `~/.cargo/bin` and leaves the symlinks behind, so
//!   `apply --refresh` DETECTED the divergence and then died on `cp`.
//!
//! Both are the same root cause — an in-place open of the destination path —
//! which is why one fix retires both.
//!
//! # Why not `install(1)`
//!
//! The cargo provider already moved to `install`, which does clear both
//! refusals: GNU unlinks the destination first, BSD writes a sibling temp and
//! renames. But GNU's unlink-then-create leaves the path ABSENT in between,
//! and on a host where sixteen CI runners share one `$CARGO_HOME/bin` an
//! `exec` landing in that window fails ENOENT.
//!
//! Measured on paiml's lambda-labs, statting the destination in a tight loop
//! while it was replaced 4000 times:
//!
//! ```text
//! install(1):  10611 absent of 396132 stats   (2.7%)
//! temp + mv:       0 absent of 741725 stats
//! ```
//!
//! `rename(2)` is atomic by definition: a concurrent `exec` gets the old
//! binary or the new one, never nothing. It also does not follow a symlink at
//! the destination, so it replaces a dangling link rather than chasing it.
//! Staging as a SIBLING of the destination is what keeps the rename within one
//! filesystem, where atomicity is guaranteed and `mv` cannot silently
//! degrade into copy-then-unlink.

/// Shell function name emitted by [`atomic_install_fn`].
pub const ATOMIC_INSTALL_FN: &str = "_fj_install_bin";

/// POSIX shell definition of `_fj_install_bin <src> <dest> [runner]`.
///
/// `runner` is the command runner for destinations the current user cannot
/// write: `command` (the POSIX builtin, and the default) or `sudo`. It is
/// expanded QUOTED — `"$_fji_run" cp ...` — which is why the emitted shell
/// carries no SC2086/SC2183 findings. An earlier draft used an unquoted
/// prefix defaulting to the empty string; it worked, and cost the shipped
/// `install.sh` sixteen new bashrs warnings, in a repo whose whole premise is
/// that every script it ships passes that gate.
///
/// The destination DIRECTORY is the caller's responsibility. A `mkdir -p
/// "$dir"` here tripped bashrs SEC010 (path traversal) in `install.sh`, and
/// all three call sites create the directory already — so a missing one is a
/// loud failure rather than something this helper papers over.
///
/// Emitted rather than shelled out to because forjar's job is to produce a
/// script that runs on the target host, and the target may be reached over
/// ssh with no forjar on it.
pub fn atomic_install_fn() -> &'static str {
    r#"# _fj_install_bin <src> <dest> [runner]
#
# Replace <dest> with <src> ATOMICALLY: stage a sibling, then rename(2).
# rename() neither opens the destination (so a RUNNING binary is fine) nor
# follows it (so a DANGLING SYMLINK is replaced, not chased), and leaves no
# window in which the path does not exist.
#
# [runner] is the command runner: `command` (the default) or `sudo`.
# The destination directory must already exist -- every caller creates it.
_fj_install_bin() {
  _fji_src="$1"
  _fji_dst="$2"
  _fji_run="${3:-command}"
  case "$_fji_dst" in
    */*) _fji_dir="${_fji_dst%/*}" ;;
    *)   _fji_dir="." ;;
  esac
  # Staged as a SIBLING so the rename cannot cross a filesystem and degrade
  # into copy-then-unlink. The name is predictable, so unlink before writing:
  # cp must not follow a symlink someone left at that path.
  _fji_tmp="$_fji_dir/.forjar-install.$$"
  "$_fji_run" rm -f "$_fji_tmp" || return 1
  if ! "$_fji_run" cp -f "$_fji_src" "$_fji_tmp"; then
    "$_fji_run" rm -f "$_fji_tmp"
    return 1
  fi
  if ! "$_fji_run" chmod 755 "$_fji_tmp"; then
    "$_fji_run" rm -f "$_fji_tmp"
    return 1
  fi
  if ! "$_fji_run" mv -f "$_fji_tmp" "$_fji_dst"; then
    "$_fji_run" rm -f "$_fji_tmp"
    return 1
  fi
}"#
}

/// `_fj_install_bin` plus a loop that lands every file of a staging `bin/`
/// directory into `$2`, used by the cargo provider where one crate may ship
/// several binaries.
pub fn atomic_install_dir_fn() -> String {
    format!(
        "{}\n{}",
        atomic_install_fn(),
        r#"_fj_install_bins() {
  # $1 = source bin/ directory, $2 = destination directory.
  for _fjb in "$1"/*; do
    [ -f "$_fjb" ] || continue
    _fj_install_bin "$_fjb" "$2/$(basename "$_fjb")" || return 1
  done
}"#
    )
}

#[cfg(test)]
#[path = "tests_shell_install.rs"]
mod tests;
