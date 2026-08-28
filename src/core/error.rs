//! forjar's error taxonomy — the classification that decides the exit code.
//!
//! # The defect this module exists to remove
//!
//! `main` used to choose the process exit code by substring-matching the error
//! TEXT. Applying a resource to gx10 produced, verbatim:
//!
//! ```text
//! transport error: I8 violation — script failed bashrs validation: bashrs lint errors: ...
//! ```
//!
//! That message contains `"transport"`, so the old classifier returned 4 — the
//! connection code, which every CI script treats as RETRYABLE — for a
//! deterministic bashrs rejection that fails identically on every retry. The
//! text was never a taxonomy: any failure whose prose happens to name a
//! transport was classified as a transport failure, and the classifier could
//! not tell "the SSH connection dropped" from "the script we were about to send
//! over SSH is invalid".
//!
//! # What changed, and what did NOT
//!
//! The exit code VALUES are a public contract — CI scripts key on them:
//!
//! | code | class                | meaning                                  |
//! |------|----------------------|------------------------------------------|
//! | 0    | —                    | success (all resources converged)         |
//! | 1    | [`ErrorClass::Other`]      | general/unclassified failure       |
//! | 2    | [`ErrorClass::Partial`]    | partial failure — some resources failed |
//! | 3    | [`ErrorClass::Validation`] | bad input: YAML, validation, I8 gate |
//! | 4    | [`ErrorClass::Connection`] | could not reach the target — retryable |
//! | 10   | [`ErrorClass::Drift`]      | drift detected                      |
//!
//! What changed is HOW the code is chosen: from a VARIANT a producer declares,
//! not from prose a consumer guesses at. What the codes MEAN is unchanged.
//!
//! # The two paths, and which one is temporary
//!
//! * **Declared** — a producer builds a [`ForjarError`] with the class it knows
//!   it has ([`ForjarError::validation`] and friends). This is the real path.
//! * **[`classify_untyped`]** — the NAMED, deliberately-temporary fallback for
//!   the error sites that still return `Result<_, String>`. It is the old prose
//!   heuristic, kept verbatim so no code's meaning shifts under it, and it
//!   applies only where nothing was declared.
//!
//! Migrating a site means giving it a class. When every site has one,
//! [`legacy_prose_class`] and its callers delete outright.

use std::fmt;

/// What kind of failure this is. The exit code is a property of the VARIANT.
///
/// Adding a variant is a change to the public exit-code contract: give it a
/// code deliberately, and say so in the changelog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    /// Exit 1 — a failure we have not classified. The honest default.
    Other,
    /// Exit 2 — partial failure: some resources converged, some did not.
    Partial,
    /// Exit 3 — the INPUT is wrong: invalid YAML, a failed validation rule, a
    /// script the I8 bashrs gate rejected. Deterministic: a retry cannot help.
    Validation,
    /// Exit 4 — we could not REACH the target: SSH or container transport
    /// refused, timed out, or died. Retryable in principle.
    Connection,
    /// Exit 10 — drift detected (`forjar drift` found a non-zero diff).
    Drift,
}

impl ErrorClass {
    /// The process exit code for this class. These values are the contract.
    pub const fn exit_code(self) -> i32 {
        match self {
            ErrorClass::Other => 1,
            ErrorClass::Partial => 2,
            ErrorClass::Validation => 3,
            ErrorClass::Connection => 4,
            ErrorClass::Drift => 10,
        }
    }

    /// A short stable label, for diagnostics and structured output.
    pub const fn label(self) -> &'static str {
        match self {
            ErrorClass::Other => "other",
            ErrorClass::Partial => "partial",
            ErrorClass::Validation => "validation",
            ErrorClass::Connection => "connection",
            ErrorClass::Drift => "drift",
        }
    }
}

impl fmt::Display for ErrorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// The headline of an I8 rejection, written ONCE here.
///
/// `src/transport/mod.rs::validate_before_exec` formats its message with this
/// constant and [`ForjarError::validation`]; [`classify_untyped`] recognises the
/// same constant on the way back out. Producer and classifier cannot drift
/// apart, because there is only one string.
pub const I8_VALIDATION_MARKER: &str = "I8 violation — script failed bashrs validation";

/// Markers a MIGRATED producer stamps into its message so that the class it
/// DECLARED survives a not-yet-typed `Result<_, String>` boundary.
///
/// Each entry is a contract with one named call site — not a guess about prose.
/// An entry disappears when that call site can return a [`ForjarError`]
/// directly. [`ForjarError::into_untyped`] debug-asserts the round trip, so a
/// typed error cannot cross an untyped boundary and silently lose its class.
const DECLARED_MARKERS: &[(&str, ErrorClass)] = &[
    // src/transport/mod.rs::validate_before_exec — the I8 bashrs gate. Reaches
    // `dispatch` through `exec_script` (78 call sites, still `String`).
    (I8_VALIDATION_MARKER, ErrorClass::Validation),
];

/// An error that knows what kind of failure it is.
///
/// The message is what the user reads; the class is what the process exits
/// with. Nothing infers one from the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForjarError {
    class: ErrorClass,
    message: String,
}

impl ForjarError {
    /// Build an error with an explicitly declared class.
    pub fn new(class: ErrorClass, message: impl Into<String>) -> Self {
        Self {
            class,
            message: message.into(),
        }
    }

    /// Exit 3 — bad input: YAML, a validation rule, the I8 bashrs gate.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::Validation, message)
    }

    /// Exit 4 — the target could not be reached.
    pub fn connection(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::Connection, message)
    }

    /// Exit 2 — some resources converged and some failed.
    pub fn partial(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::Partial, message)
    }

    /// Exit 10 — drift detected.
    pub fn drift(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::Drift, message)
    }

    /// Exit 1 — an unclassified failure, said out loud.
    pub fn other(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::Other, message)
    }

    /// The declared class.
    pub fn class(&self) -> ErrorClass {
        self.class
    }

    /// The human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The process exit code — read straight off the variant.
    pub fn exit_code(&self) -> i32 {
        self.class.exit_code()
    }

    /// Consume the error, returning its message.
    pub fn into_message(self) -> String {
        self.message
    }

    /// TEMPORARY. Classify a `String` from a site that has not been migrated.
    ///
    /// This is the named fallback: it is the only way an unclassified error
    /// acquires a class, and it is the only remaining place prose is read. See
    /// [`classify_untyped`] for what it can and cannot tell apart.
    pub fn from_untyped(message: impl Into<String>) -> Self {
        let message = message.into();
        let class = classify_untyped(&message);
        Self { class, message }
    }

    /// TEMPORARY. Lower a typed error onto a `Result<_, String>` boundary that
    /// has not been migrated yet.
    ///
    /// The class survives only because the message carries a marker registered
    /// in `DECLARED_MARKERS`; the debug assertion below is what stops a future
    /// producer from crossing here and quietly losing its class. Delete this
    /// method when the boundary it serves takes a [`ForjarError`].
    pub fn into_untyped(self) -> String {
        debug_assert_eq!(
            classify_untyped(&self.message),
            self.class,
            "a declared {} error crossed an untyped boundary with a message no \
             marker in DECLARED_MARKERS recognises — it would be reclassified as \
             {} at the exit-code boundary. Register a marker, or migrate the \
             boundary to ForjarError.",
            self.class,
            classify_untyped(&self.message),
        );
        self.message
    }
}

impl fmt::Display for ForjarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ForjarError {}

/// TEMPORARY. Classify an error that arrived as a bare `String`.
///
/// Order matters: a class a producer DECLARED (via a marker) always wins over
/// the prose heuristic. That single rule is what fixes the measured gx10
/// failure — an I8 rejection is validation (3) even though its message is
/// wrapped in the words "transport error".
///
/// Everything below the marker table is guesswork retained for compatibility.
/// It cannot tell "the SSH connection dropped" from "the script we were going
/// to send over SSH is invalid"; that is exactly why it is being retired.
pub fn classify_untyped(message: &str) -> ErrorClass {
    declared_class(message).unwrap_or_else(|| legacy_prose_class(message))
}

/// The class a migrated producer declared, if this message carries its marker.
fn declared_class(message: &str) -> Option<ErrorClass> {
    DECLARED_MARKERS
        .iter()
        .find(|(marker, _)| message.contains(marker))
        .map(|(_, class)| *class)
}

/// TEMPORARY. The pre-taxonomy heuristic, preserved exactly.
///
/// Kept verbatim (same tests, same order) so that migrating the mechanism does
/// not silently re-code any error that was already being classified. It applies
/// only to sites that declare nothing; every site migrated to [`ForjarError`]
/// removes a caller of this function, and the last one deletes it.
fn legacy_prose_class(message: &str) -> ErrorClass {
    if message.contains("validation error")
        || message.contains("YAML parse error")
        || message.contains("not a forjar config")
    {
        ErrorClass::Validation
    } else if message.contains("SSH")
        || message.contains("connection")
        || message.contains("transport")
    {
        ErrorClass::Connection
    } else if message.contains("partial") || message.contains("some resources failed") {
        ErrorClass::Partial
    } else if message.contains("drift detected") {
        ErrorClass::Drift
    } else {
        ErrorClass::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact text `forjar apply` produced against gx10. It is the reason
    /// this module exists: a deterministic bashrs rejection, wrapped by
    /// `resource_ops` in the words "transport error", which the prose
    /// classifier read as a retryable connection failure.
    const MEASURED_GX10_I8_FAILURE: &str =
        "transport error: I8 violation — script failed bashrs validation: bashrs lint errors: ...";

    #[test]
    fn the_measured_gx10_failure_is_validation_not_connection() {
        assert_eq!(
            classify_untyped(MEASURED_GX10_I8_FAILURE),
            ErrorClass::Validation
        );
        assert_eq!(
            ForjarError::from_untyped(MEASURED_GX10_I8_FAILURE).exit_code(),
            3,
            "an I8 bashrs rejection is deterministic — exiting 4 tells CI to retry \
             a failure that cannot succeed"
        );
    }

    /// Falsification: without the declared marker the SAME string classifies as
    /// connection/4. This is what makes the fix a fix, and not luck.
    #[test]
    fn prose_alone_gets_the_measured_failure_wrong() {
        assert_eq!(
            legacy_prose_class(MEASURED_GX10_I8_FAILURE),
            ErrorClass::Connection,
            "if the prose heuristic ever got this right on its own, this test \
             would be measuring nothing"
        );
        assert_eq!(legacy_prose_class(MEASURED_GX10_I8_FAILURE).exit_code(), 4);
    }

    /// The EXECUTION half of KANI-FVS-002.
    ///
    /// Kani proves `ErrorClass::exit_code` is variant-determined and injective,
    /// allocation-free. It deliberately never constructs a `ForjarError`, because
    /// that carries a `String` and modelling the allocator to prove a one-line
    /// delegation costs 117 minutes for nothing (see kani_proofs_backup_sync).
    ///
    /// So the delegation is proved HERE, by building real errors whose messages
    /// differ in length, content, and — critically — in whether they contain the
    /// very words the OLD prose classifier keyed on. If `exit_code` ever stopped
    /// reading the variant, this is what catches it.
    #[test]
    fn the_exit_code_ignores_the_message_entirely() {
        let messages = [
            "",
            "x",
            "SSH connection refused",        // old heuristic: 4
            "YAML parse error at line 3",    // old heuristic: 3
            "drift detected on 2 resources", // old heuristic: 10
            "some resources failed",         // old heuristic: 2
            "transport error: I8 violation — script failed bashrs validation",
            &"very long message ".repeat(200),
        ];
        for class in [
            ErrorClass::Other,
            ErrorClass::Partial,
            ErrorClass::Validation,
            ErrorClass::Connection,
            ErrorClass::Drift,
        ] {
            let expected = class.exit_code();
            for m in &messages {
                let e = ForjarError::new(class, *m);
                assert_eq!(
                    e.exit_code(),
                    expected,
                    "class {class:?} with message {m:?} produced exit {} — the code \
                     must come from the VARIANT, and this message is one the old \
                     prose classifier would have keyed on",
                    e.exit_code()
                );
            }
        }
    }

    #[test]
    fn exit_codes_are_the_published_contract() {
        assert_eq!(ErrorClass::Other.exit_code(), 1);
        assert_eq!(ErrorClass::Partial.exit_code(), 2);
        assert_eq!(ErrorClass::Validation.exit_code(), 3);
        assert_eq!(ErrorClass::Connection.exit_code(), 4);
        assert_eq!(ErrorClass::Drift.exit_code(), 10);
    }

    #[test]
    fn a_declared_class_beats_the_prose_of_its_own_message() {
        // The message says every connection-ish word there is; the variant says
        // validation. The variant wins, because the producer knew.
        let e = ForjarError::validation("SSH connection transport all at once");
        assert_eq!(e.class(), ErrorClass::Validation);
        assert_eq!(e.exit_code(), 3);
    }

    #[test]
    fn every_declared_marker_round_trips() {
        for (marker, class) in DECLARED_MARKERS {
            assert_eq!(
                classify_untyped(marker),
                *class,
                "marker {marker:?} does not classify as {class}"
            );
            // And it survives being wrapped by a caller, which is the whole point.
            let wrapped = format!("transport error: {marker}: details here");
            assert_eq!(classify_untyped(&wrapped), *class);
        }
    }

    #[test]
    fn into_untyped_preserves_the_declared_class() {
        let lowered = ForjarError::validation(format!(
            "{I8_VALIDATION_MARKER}: bashrs lint errors:\n[Error] SC2135: x"
        ))
        .into_untyped();
        assert_eq!(ForjarError::from_untyped(lowered).exit_code(), 3);
    }

    #[test]
    fn legacy_meanings_are_unchanged() {
        // Same inputs, same codes as the pre-taxonomy classifier in `main`.
        assert_eq!(
            classify_untyped("validation error: bad field").exit_code(),
            3
        );
        assert_eq!(
            classify_untyped("YAML parse error at line 3").exit_code(),
            3
        );
        assert_eq!(classify_untyped("SSH handshake failed").exit_code(), 4);
        assert_eq!(classify_untyped("connection refused").exit_code(), 4);
        assert_eq!(classify_untyped("transport timeout").exit_code(), 4);
        assert_eq!(classify_untyped("partial apply").exit_code(), 2);
        assert_eq!(classify_untyped("some resources failed").exit_code(), 2);
        assert_eq!(
            classify_untyped("drift detected in 2 resources").exit_code(),
            10
        );
        assert_eq!(classify_untyped("something else entirely").exit_code(), 1);
    }

    #[test]
    fn constructors_carry_their_class() {
        assert_eq!(ForjarError::other("x").class(), ErrorClass::Other);
        assert_eq!(ForjarError::partial("x").class(), ErrorClass::Partial);
        assert_eq!(ForjarError::validation("x").class(), ErrorClass::Validation);
        assert_eq!(ForjarError::connection("x").class(), ErrorClass::Connection);
        assert_eq!(ForjarError::drift("x").class(), ErrorClass::Drift);
        assert_eq!(
            ForjarError::new(ErrorClass::Drift, "x").class(),
            ErrorClass::Drift
        );
    }

    #[test]
    fn display_is_the_message_and_nothing_else() {
        // `main` prints "error: {e}" — the taxonomy must not change what a user
        // reads, only what the process exits with.
        let e = ForjarError::connection("ssh: connect to host gx10 port 22: No route to host");
        assert_eq!(
            e.to_string(),
            "ssh: connect to host gx10 port 22: No route to host"
        );
        assert_eq!(e.message(), e.to_string());
        assert_eq!(
            ForjarError::other("boom").into_message(),
            "boom".to_string()
        );
    }

    #[test]
    fn class_labels_are_stable() {
        assert_eq!(ErrorClass::Validation.label(), "validation");
        assert_eq!(ErrorClass::Connection.to_string(), "connection");
        assert_eq!(ErrorClass::Partial.to_string(), "partial");
        assert_eq!(ErrorClass::Drift.to_string(), "drift");
        assert_eq!(ErrorClass::Other.to_string(), "other");
    }

    #[test]
    fn errors_are_std_errors() {
        fn as_std(e: &dyn std::error::Error) -> String {
            e.to_string()
        }
        assert_eq!(as_std(&ForjarError::other("boom")), "boom");
    }
}
