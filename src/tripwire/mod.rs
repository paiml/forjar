//! Tripwire — provenance tracing, BLAKE3 hashing, drift detection, syscall tracing.

pub mod anomaly;
pub mod chain;
pub mod drift;
pub mod eventlog;
pub mod hasher;
pub mod otlp_export;
pub mod tracer;

#[cfg(test)]
mod tests_anomaly;
#[cfg(test)]
mod tests_chain;
#[cfg(test)]
mod tests_eventlog;
#[cfg(test)]
mod tests_eventlog_b;
#[cfg(test)]
mod tests_hasher;
#[cfg(test)]
mod tests_hasher_b;
