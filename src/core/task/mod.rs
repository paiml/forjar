//! FJ-2700: Task framework runtime — quality gates, GPU targeting, pipeline state.

#[allow(unused)]
mod quality_gate;

pub use quality_gate::{evaluate_gate, gpu_env_vars, GateResult};

#[cfg(test)]
mod tests_quality_gate;
