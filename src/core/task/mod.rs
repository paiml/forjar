//! FJ-2700: Task framework runtime — quality gates, GPU targeting, I/O tracking, pipelines.

#[allow(unused)]
mod quality_gate;
mod io_tracking;
pub mod pipeline;

pub use quality_gate::{evaluate_gate, gpu_env_vars, GateResult};
pub use io_tracking::{hash_inputs, hash_outputs, should_skip_cached};

#[cfg(test)]
mod tests_quality_gate;
#[cfg(test)]
mod tests_io_tracking;
