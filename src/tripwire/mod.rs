//! Tripwire — provenance tracing, BLAKE3 hashing, drift detection, syscall tracing.

pub mod anomaly;
pub mod drift;
pub mod eventlog;
pub mod hasher;
pub mod tracer;

#[cfg(test)]
mod tests_eventlog;
#[cfg(test)]
mod tests_hasher;
#[cfg(test)]
mod tests_anomaly;
