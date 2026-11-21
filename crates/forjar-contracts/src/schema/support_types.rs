use serde::{Deserialize, Serialize};

/// Kernel phase decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelStructure {
    pub phases: Vec<KernelPhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelPhase {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub invariant: Option<String>,
}

/// An enforcement rule from the contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementRule {
    pub description: String,
    #[serde(default)]
    pub check: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub reference: Option<String>,
}

/// A Popperian falsification test.
///
/// Each makes a falsifiable prediction about the implementation.
/// If the prediction is wrong, the test identifies root cause.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FalsificationTest {
    pub id: String,
    /// What the test asserts. Alias `description` accepted for legacy
    /// pre-APR-MONO contracts that used the `description:` field name.
    /// `name:` is NOT aliased because several legacy contracts (e.g.
    /// `publish-manifest-v1`) ship both `name:` (a slug) and
    /// `description:` (prose) side-by-side; aliasing both collapses to
    /// a `duplicate field` error.
    #[serde(default, alias = "description")]
    pub rule: String,
    /// The predicted outcome if the rule holds. Alias `expected` accepted
    /// for legacy contracts (e.g. `expected: exit 0`, `expected: "PASS"`).
    /// Defaulted because diagnostic contracts often encode prediction
    /// inside the rule text alone.
    #[serde(default, alias = "expected")]
    pub prediction: String,
    /// How to run the test. Alias `command` accepted for legacy contracts
    /// (e.g. shell snippets under `command: |`).
    #[serde(default, alias = "command")]
    pub test: Option<String>,
    /// What failure means. Alias `fails_if` accepted for legacy contracts.
    /// Defaulted because several legacy diagnostic contracts omit it.
    #[serde(default, alias = "fails_if")]
    pub if_fails: String,
}

/// A Kani bounded model checking harness definition.
///
/// Corresponds to Phase 6 (Verify) of the pipeline.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KaniHarness {
    pub id: String,
    pub obligation: String,
    #[serde(default)]
    pub property: Option<String>,
    #[serde(default)]
    pub bound: Option<u32>,
    #[serde(default)]
    pub strategy: Option<KaniStrategy>,
    #[serde(default)]
    pub solver: Option<String>,
    #[serde(default)]
    pub harness: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KaniStrategy {
    Exhaustive,
    StubFloat,
    Compositional,
    BoundedInt,
}

impl std::fmt::Display for KaniStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Exhaustive => "exhaustive",
            Self::StubFloat => "stub_float",
            Self::Compositional => "compositional",
            Self::BoundedInt => "bounded_int",
        };
        write!(f, "{s}")
    }
}

/// Phase 7: Lean 4 theorem proving metadata for a proof obligation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanProof {
    /// Lean 4 theorem name (e.g., `Softmax.partition_of_unity`).
    pub theorem: String,
    /// Lean 4 module path (e.g., `ProvableContracts.Softmax`).
    #[serde(default)]
    pub module: Option<String>,
    /// Current status of the Lean proof.
    #[serde(default)]
    pub status: LeanStatus,
    /// Lean-level theorem dependencies.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Mathlib import paths required.
    #[serde(default)]
    pub mathlib_imports: Vec<String>,
    /// Free-form notes (e.g., "Proof over reals; f32 gap addressed separately").
    #[serde(default)]
    pub notes: Option<String>,
}

/// Status of a Lean 4 proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LeanStatus {
    /// Proof is complete and type-checks.
    Proved,
    /// Proof uses `sorry` (axiomatized, not yet proved).
    #[default]
    Sorry,
    /// Work in progress.
    Wip,
    /// Obligation is not amenable to Lean proof (e.g., performance).
    NotApplicable,
}

impl std::fmt::Display for LeanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Proved => "proved",
            Self::Sorry => "sorry",
            Self::Wip => "wip",
            Self::NotApplicable => "not-applicable",
        };
        write!(f, "{s}")
    }
}

/// Phase 7: Verification summary across all obligations in a contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSummary {
    pub total_obligations: u32,
    #[serde(default)]
    pub l2_property_tested: u32,
    #[serde(default)]
    pub l3_kani_proved: u32,
    #[serde(default)]
    pub l4_lean_proved: u32,
    #[serde(default)]
    pub l4_sorry_count: u32,
    #[serde(default)]
    pub l4_not_applicable: u32,
}

/// QA gate definition for certeza integration.
///
/// Legacy diagnostic contracts (e.g.
/// `decode-hot-path-prefix-cache-diagnostic-v1`) ship a `qa_gate:` block
/// with only `must_pass:` / `integration:` / `regression_protection:` — no
/// `id:` or `name:`. All schema fields default so those parse cleanly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QaGate {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub pass_criteria: Option<String>,
    #[serde(default)]
    pub falsification: Option<String>,
}

/// A type-level invariant (Meyer's class invariant).
///
/// Asserts a predicate that must hold for every instance of `type_name`
/// at every stable state — after construction and after every public method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInvariant {
    pub name: String,
    /// Rust type name (e.g., `ValidatedTensor`).
    #[serde(rename = "type")]
    pub type_name: String,
    /// Rust boolean expression over `self` (e.g., `!self.dims.is_empty()`).
    pub predicate: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Coq verification specification for a contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoqSpec {
    /// Coq module name (e.g., `SoftmaxSpec`).
    pub module: String,
    /// Coq import statements.
    #[serde(default)]
    pub imports: Vec<String>,
    /// Coq definitions generated from equations.
    #[serde(default)]
    pub definitions: Vec<CoqDefinition>,
    /// Links from proof obligations to Coq lemmas.
    #[serde(default)]
    pub obligations: Vec<CoqObligation>,
}

/// A Coq definition derived from a contract equation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoqDefinition {
    pub name: String,
    pub statement: String,
}

/// A link between a proof obligation and a Coq lemma.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoqObligation {
    /// References a proof obligation property or ID.
    pub links_to: String,
    /// Coq lemma name.
    pub coq_lemma: String,
    /// Current status of the Coq proof.
    #[serde(default = "coq_status_default")]
    pub status: String,
}

fn coq_status_default() -> String {
    "stub".to_string()
}
