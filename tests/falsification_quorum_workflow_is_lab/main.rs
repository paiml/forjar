//! C58-3 / rule C324: the `Quorum` workflow cannot block a merge, so it is LAB (see `common.rs`).
//! Split into submodules to keep every file under 500 lines; behaviour unchanged.

mod common;
mod hardening;
mod pin;
mod structural;
