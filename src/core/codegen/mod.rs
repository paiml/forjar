//! FJ-005: Script generation — dispatch to resource handlers.
//! FJ-036: bashrs purification pipeline integrated (Invariant I8).

mod dispatch;

pub use dispatch::*;

#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod tests_completeness;
#[cfg(test)]
mod tests_coverage;
#[cfg(test)]
mod tests_dispatch;
#[cfg(test)]
mod tests_sudo;
