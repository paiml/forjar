//! FJ-2500–2504: Validation output types — structured error reporting,
//! field suggestions, and validation pipeline results.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2500: Validation severity level.
///
/// # Examples
///
/// ```
/// use forjar::core::types::ValidationSeverity;
///
/// let sev = ValidationSeverity::Error;
/// assert!(sev > ValidationSeverity::Warning);
/// assert_eq!(sev.to_string(), "error");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    /// Informational hint (does not block).
    Hint,
    /// Warning (printed to stderr, does not block by default).
    Warning,
    /// Error (blocks validation/apply).
    Error,
}

impl fmt::Display for ValidationSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hint => write!(f, "hint"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// FJ-2500: A structured validation finding.
///
/// # Examples
///
/// ```
/// use forjar::core::types::{ValidationFinding, ValidationSeverity};
///
/// let finding = ValidationFinding {
///     severity: ValidationSeverity::Warning,
///     resource: Some("nginx".into()),
///     field: Some("packges".into()),
///     message: "unknown field".into(),
///     suggestion: Some("did you mean 'packages'?".into()),
/// };
/// assert!(finding.is_warning());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationFinding {
    /// Severity level.
    pub severity: ValidationSeverity,
    /// Resource ID (if applicable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    /// Field name (if applicable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Human-readable error/warning message.
    pub message: String,
    /// "Did you mean?" suggestion (if applicable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl ValidationFinding {
    /// Create an error finding.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            resource: None,
            field: None,
            message: message.into(),
            suggestion: None,
        }
    }

    /// Create a warning finding.
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            resource: None,
            field: None,
            message: message.into(),
            suggestion: None,
        }
    }

    /// Attach resource context.
    pub fn for_resource(mut self, resource_id: impl Into<String>) -> Self {
        self.resource = Some(resource_id.into());
        self
    }

    /// Attach field context.
    pub fn for_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    /// Attach a "did you mean?" suggestion.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Whether this finding is an error.
    pub fn is_error(&self) -> bool {
        self.severity == ValidationSeverity::Error
    }

    /// Whether this finding is a warning.
    pub fn is_warning(&self) -> bool {
        self.severity == ValidationSeverity::Warning
    }
}

impl fmt::Display for ValidationFinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.severity)?;
        if let Some(ref resource) = self.resource {
            write!(f, ": resource \"{resource}\"")?;
            if let Some(ref field) = self.field {
                write!(f, " field \"{field}\"")?;
            }
        }
        write!(f, " — {}", self.message)?;
        if let Some(ref suggestion) = self.suggestion {
            write!(f, " ({suggestion})")?;
        }
        Ok(())
    }
}

/// FJ-2500: Aggregate validation output (human-readable and JSON).
///
/// # Examples
///
/// ```
/// use forjar::core::types::{ValidateOutput, ValidationFinding, ValidationSeverity};
///
/// let output = ValidateOutput {
///     valid: false,
///     resource_count: 12,
///     machine_count: 3,
///     findings: vec![
///         ValidationFinding::error("missing required field 'packages'")
///             .for_resource("nginx")
///             .for_field("packages"),
///     ],
/// };
/// assert_eq!(output.error_count(), 1);
/// assert_eq!(output.warning_count(), 0);
/// assert!(!output.valid);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidateOutput {
    /// Whether the config is valid (no errors).
    pub valid: bool,
    /// Total resource count.
    pub resource_count: usize,
    /// Total machine count.
    pub machine_count: usize,
    /// All findings (errors + warnings + hints).
    pub findings: Vec<ValidationFinding>,
}

impl ValidateOutput {
    /// Build output from findings.
    pub fn from_findings(
        findings: Vec<ValidationFinding>,
        resource_count: usize,
        machine_count: usize,
    ) -> Self {
        let valid = !findings.iter().any(|f| f.is_error());
        Self {
            valid,
            resource_count,
            machine_count,
            findings,
        }
    }

    /// Count of error-severity findings.
    pub fn error_count(&self) -> usize {
        self.findings.iter().filter(|f| f.is_error()).count()
    }

    /// Count of warning-severity findings.
    pub fn warning_count(&self) -> usize {
        self.findings.iter().filter(|f| f.is_warning()).count()
    }

    /// Format human-readable validation output.
    pub fn format_summary(&self) -> String {
        let mut out = String::new();
        for finding in &self.findings {
            out.push_str(&format!("{finding}\n"));
        }
        out.push_str(&format!(
            "\n{} errors, {} warnings ({} resources, {} machines)\n",
            self.error_count(),
            self.warning_count(),
            self.resource_count,
            self.machine_count,
        ));
        out
    }
}

/// FJ-2500: "Did you mean?" suggestion with Levenshtein distance.
///
/// # Examples
///
/// ```
/// use forjar::core::types::FieldSuggestion;
///
/// let s = FieldSuggestion::new("packges", "packages", 1);
/// assert!(s.should_suggest());
/// assert_eq!(s.to_string(), "'packges' -> 'packages' (distance: 1)");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSuggestion {
    /// The unknown field name (typo).
    pub unknown: String,
    /// The suggested known field name.
    pub known: String,
    /// Levenshtein distance between the two.
    pub distance: usize,
}

impl FieldSuggestion {
    /// Create a new suggestion.
    pub fn new(unknown: impl Into<String>, known: impl Into<String>, distance: usize) -> Self {
        Self {
            unknown: unknown.into(),
            known: known.into(),
            distance,
        }
    }

    /// Whether this suggestion should be shown (distance <= 2).
    pub fn should_suggest(&self) -> bool {
        self.distance <= 2
    }
}

impl fmt::Display for FieldSuggestion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "'{}' -> '{}' (distance: {})",
            self.unknown, self.known, self.distance
        )
    }
}

/// FJ-2503: Deep validation check flags.
///
/// # Examples
///
/// ```
/// use forjar::core::types::DeepCheckFlags;
///
/// let flags = DeepCheckFlags::exhaustive();
/// assert!(flags.templates);
/// assert!(flags.circular_deps);
/// assert!(flags.secrets);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeepCheckFlags {
    /// `--check-templates`: validate `{{...}}` references resolve.
    #[serde(default)]
    pub templates: bool,
    /// `--check-circular-deps`: detect DAG cycles.
    #[serde(default)]
    pub circular_deps: bool,
    /// `--check-connectivity`: test SSH connections.
    #[serde(default)]
    pub connectivity: bool,
    /// `--check-secrets`: scan for hardcoded secrets.
    #[serde(default)]
    pub secrets: bool,
    /// `--check-overlaps`: detect conflicting resource targets.
    #[serde(default)]
    pub overlaps: bool,
    /// `--check-naming`: resource naming conventions.
    #[serde(default)]
    pub naming: bool,
    /// `--check-machine-refs`: validate machine references in resources.
    #[serde(default)]
    pub machine_refs: bool,
    /// `--check-state-values`: valid state field values (present/absent/running/stopped).
    #[serde(default)]
    pub state_values: bool,
    /// `--check-drift-coverage`: verify all resources have state_query scripts.
    #[serde(default)]
    pub drift_coverage: bool,
    /// `--check-idempotency`: script idempotency simulation.
    #[serde(default)]
    pub idempotency: bool,
}

impl DeepCheckFlags {
    /// All deep checks enabled (`--exhaustive`).
    pub fn exhaustive() -> Self {
        Self {
            templates: true,
            circular_deps: true,
            connectivity: true,
            secrets: true,
            overlaps: true,
            naming: true,
            machine_refs: true,
            state_values: true,
            drift_coverage: true,
            idempotency: true,
        }
    }

    /// Whether any deep check is enabled.
    pub fn any_enabled(&self) -> bool {
        self.templates
            || self.circular_deps
            || self.connectivity
            || self.secrets
            || self.overlaps
            || self.naming
            || self.machine_refs
            || self.state_values
            || self.drift_coverage
            || self.idempotency
    }
}
