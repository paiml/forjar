//! FJ-LOCK-001 C3/C5: the state-lock flags of `forjar apply`.

/// State-lock flags, flattened into [`super::ApplyArgs`].
#[derive(clap::Args, Debug, Default)]
pub struct ApplyLockArgs {
    /// FJ-266 / FJ-LOCK-001 C3: clear a dead holder's state lock before apply.
    /// `--force-unlock <ID>` names the lock (the id in the refusal message).
    /// A holder that is alive on this host is refused either way.
    #[arg(long, value_name = "ID", num_args = 0..=1, default_missing_value = "")]
    pub force_unlock: Option<String>,

    /// FJ-LOCK-001 C5: wait up to this long (e.g. 30s, 2m) for a held state
    /// lock, first come first served; 0 (default) fails fast naming the holder
    #[arg(long, value_name = "DUR")]
    pub lock_timeout: Option<String>,
}
