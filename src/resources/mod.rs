//! Resource handlers — generate purified shell for each resource type.
//!
//! Each handler produces:
//! 1. A "check" script that reads current state
//! 2. An "apply" script that converges to desired state
//! 3. A "hash" function that computes the BLAKE3 of observable state

pub mod build;
pub mod cron;
pub mod docker;
pub mod file;
pub mod github_release;
pub mod gpu;
pub mod model;
pub mod mount;
pub mod network;
pub mod package;
pub mod pepita;
pub mod service;
pub mod task;
#[cfg(test)]
mod tests_service;
#[cfg(test)]
mod tests_task;
pub mod user;
pub mod wasm_bundle;

mod network_b;
#[cfg(test)]
mod tests_build;
#[cfg(test)]
mod tests_docker;
#[cfg(test)]
mod tests_docker_b;
#[cfg(test)]
mod tests_file;
#[cfg(test)]
mod tests_file_b;
#[cfg(test)]
mod tests_gpu;
#[cfg(test)]
mod tests_mount;
#[cfg(test)]
mod tests_mount_b;
#[cfg(test)]
mod tests_package;
#[cfg(test)]
mod tests_package_b;
#[cfg(test)]
mod tests_package_c;
#[cfg(test)]
mod tests_user;
