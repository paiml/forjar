//! Resource handlers — generate purified shell for each resource type.
//!
//! Each handler produces:
//! 1. A "check" script that reads current state
//! 2. An "apply" script that converges to desired state
//! 3. A "hash" function that computes the BLAKE3 of observable state

pub mod cron;
pub mod docker;
pub mod file;
pub mod mount;
pub mod network;
pub mod package;
pub mod service;
pub mod user;
