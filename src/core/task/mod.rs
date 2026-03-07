//! FJ-2700: Task framework runtime — quality gates, GPU targeting, I/O tracking,
//! pipelines, service mode, dispatch mode.

pub mod dispatch;
mod io_tracking;
pub mod pipeline;
#[allow(unused)]
mod quality_gate;
pub mod service;

pub use io_tracking::{hash_inputs, hash_outputs, should_skip_cached};
pub use quality_gate::{evaluate_gate, gpu_env_vars, GateResult};

#[cfg(test)]
mod tests_io_tracking;
#[cfg(test)]
mod tests_quality_gate;
#[cfg(test)]
mod tests_service;
