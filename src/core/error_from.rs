//! `?` bridges from not-yet-migrated `String` errors into [`ForjarError`].
//!
//! A migrated producer keeps its inner `.map_err(|e| format!(..))?` lines; the
//! class is then read by the same named fallback as everywhere else
//! ([`ForjarError::from_untyped`]). Kept out of `error.rs` for the file-size cap.

use super::error::ForjarError;

impl From<String> for ForjarError {
    fn from(message: String) -> Self {
        Self::from_untyped(message)
    }
}

impl From<&str> for ForjarError {
    fn from(message: &str) -> Self {
        Self::from_untyped(message)
    }
}
