use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use super::composition::{ShapeContract, ShapeExpr};
pub use super::kind::ContractKind;
pub use super::support_types::*;

/// A complete YAML kernel contract.
///
/// This is the root type for the contract schema defined in
/// `docs/specifications/pv-spec.md` Section 3.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Contract {
    pub metadata: Metadata,
    /// Equations are optional — kaizen, pipeline, and registry contracts
    /// may define only `proof_obligations` without mathematical equations.
    ///
    /// Accepts both map form (`equations: { silu: { formula: ... } }`, the
    /// canonical schema) and sequence form (`equations: [{ id: silu,
    /// formula: ... }]`, used by several diagnostic/methodology contracts
    /// predating APR-MONO). The sequence form promotes each item's `id`
    /// field to the map key.
    #[serde(default, deserialize_with = "deserialize_equations")]
    pub equations: BTreeMap<String, Equation>,
    #[serde(default)]
    pub proof_obligations: Vec<ProofObligation>,
    #[serde(default)]
    pub kernel_structure: Option<KernelStructure>,
    #[serde(default)]
    pub simd_dispatch: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pub enforcement: BTreeMap<String, EnforcementRule>,
    #[serde(default)]
    pub falsification_tests: Vec<FalsificationTest>,
    #[serde(default)]
    pub kani_harnesses: Vec<KaniHarness>,
    #[serde(default)]
    pub qa_gate: Option<QaGate>,
    /// Phase 7: Lean 4 verification summary across all obligations.
    #[serde(default)]
    pub verification_summary: Option<VerificationSummary>,
    /// Type-level invariants (Meyer's class invariants).
    #[serde(default)]
    pub type_invariants: Vec<TypeInvariant>,
    /// Coq verification specification.
    #[serde(default)]
    pub coq_spec: Option<CoqSpec>,
}

impl Contract {
    /// Back-compat: `metadata.registry: true` OR `metadata.kind: registry`.
    pub fn is_registry(&self) -> bool {
        self.metadata.registry || self.metadata.kind == ContractKind::Registry
    }

    /// The effective kind, honoring the legacy `registry: true` flag.
    pub fn kind(&self) -> ContractKind {
        if self.metadata.registry && self.metadata.kind == ContractKind::Kernel {
            ContractKind::Registry
        } else {
            self.metadata.kind
        }
    }

    /// True iff this contract must satisfy PROVABILITY-001 (kernel only).
    pub fn requires_proofs(&self) -> bool {
        self.kind() == ContractKind::Kernel
    }

    /// Enforce the provability invariant: kernel contracts MUST have
    /// `proof_obligations`, `falsification_tests`, and `kani_harnesses`.
    /// Returns a list of violations. Empty list = contract is valid.
    pub fn provability_violations(&self) -> Vec<String> {
        if !self.requires_proofs() {
            return vec![];
        }
        let mut violations = Vec::new();
        if self.proof_obligations.is_empty() {
            violations.push("Kernel contract has no proof_obligations".into());
        }
        if self.falsification_tests.is_empty() {
            violations.push("Kernel contract has no falsification_tests".into());
        }
        if self.kani_harnesses.is_empty() {
            violations.push("Kernel contract has no kani_harnesses".into());
        }
        if self.falsification_tests.len() < self.proof_obligations.len() {
            violations.push(format!(
                "falsification_tests ({}) < proof_obligations ({})",
                self.falsification_tests.len(),
                self.proof_obligations.len(),
            ));
        }
        violations
    }
}

/// Contract metadata block.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub version: String,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    pub description: String,
    #[serde(default)]
    pub references: Vec<String>,
    /// Contract dependencies — other contracts this one composes.
    /// Values are contract stems (e.g. "silu-kernel-v1").
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Legacy registry flag — prefer `metadata.kind: registry` for new contracts.
    #[serde(default)]
    pub registry: bool,
    /// Contract kind. Defaults to [`ContractKind::Kernel`].
    #[serde(default)]
    pub kind: ContractKind,
    /// Per-contract enforcement level (Section 17, Gap 1).
    /// `basic` → schema valid; `standard` → + falsification + kani;
    /// `strict` → + all bindings implemented; `proven` → + Lean 4 proved.
    #[serde(default)]
    pub enforcement_level: Option<EnforcementLevel>,
    /// Once set, the contract cannot drop below this verification level
    /// without an explicit `pv unlock` (Section 17, Gap 5).
    #[serde(default)]
    pub locked_level: Option<String>,
}

/// Per-contract enforcement level (gradual enforcement, Section 17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnforcementLevel {
    /// Schema valid, has equations.
    Basic,
    /// + falsification tests + Kani harnesses.
    Standard,
    /// + all bindings implemented + `#[contract]` annotations.
    Strict,
    /// + Lean 4 proved (no sorry).
    Proven,
}

/// A mathematical equation extracted from a paper (Phase 1 output).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Equation {
    /// Default-empty so diagnostic/methodology contracts that use prose
    /// requirements instead of a formula (e.g.
    /// `decode-hot-path-prefix-cache-diagnostic-v1`) still parse.
    #[serde(default)]
    pub formula: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub codomain: Option<String>,
    #[serde(default)]
    pub invariants: Vec<String>,
    /// Rust preconditions — compiled to `debug_assert!()` by `build.rs`.
    #[serde(default)]
    pub preconditions: Vec<String>,
    /// Rust postconditions — compiled to `debug_assert!()` by `build.rs`.
    #[serde(default)]
    pub postconditions: Vec<String>,
    /// Lean 4 theorem name that proves this equation correct.
    /// Example: "ProvableContracts.Theorems.Softmax.PartitionOfUnity"
    #[serde(default)]
    pub lean_theorem: Option<String>,
    /// IEEE 754 tolerance: codegen emits `>=` instead of `>` for boundaries (GH-67).
    #[serde(default)]
    pub float_tolerance: Option<f64>,
    /// Compositional verification: what this equation requires from upstream.
    /// References a guarantees block from another contract/equation.
    #[serde(default)]
    pub assumes: Option<ShapeContract>,
    /// Compositional verification: what this equation provides to downstream.
    /// Must be satisfiable by any downstream equation that assumes it.
    #[serde(default)]
    pub guarantees: Option<ShapeContract>,
}

/// A proof obligation derived from an equation.
///
/// 26 obligation types: 19 property types plus 7 Design by Contract
/// types (`precondition`, `postcondition`, `frame`, `loop_invariant`,
/// `loop_variant`, `old_state`, `subcontract`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProofObligation {
    /// Obligation category. Defaults to `Invariant` for legacy contracts
    /// that predate the DbC split (e.g. `eval-harness-humaneval-v1`,
    /// `publish-manifest-v1`) which ship with just `property:`/`formal:`.
    #[serde(rename = "type", default)]
    pub obligation_type: ObligationType,
    /// Human-readable statement of what must hold. Alias `statement`
    /// accepted for legacy diagnostic contracts (e.g.
    /// `decode-hot-path-prefix-cache-diagnostic-v1`) whose POs predate
    /// the canonical `property:` naming.
    #[serde(default, alias = "statement")]
    pub property: String,
    /// Formal predicate (Rust/Lean syntax). Alias `verification` accepted
    /// for legacy contracts that ship a shell/pmat-query check instead of
    /// a formal predicate.
    #[serde(default, alias = "verification")]
    pub formal: Option<String>,
    #[serde(default)]
    pub tolerance: Option<f64>,
    #[serde(default)]
    pub applies_to: Option<AppliesTo>,
    /// Phase 7: Lean 4 theorem proving metadata.
    #[serde(default)]
    pub lean: Option<LeanProof>,
    /// Postcondition only: links to a precondition obligation ID.
    #[serde(default)]
    pub requires: Option<String>,
    /// Loop invariant/variant only: references a `kernel_structure.phases[]` name.
    #[serde(default)]
    pub applies_to_phase: Option<String>,
    /// Subcontract only: contract stem being refined (must be in `metadata.depends_on`).
    #[serde(default)]
    pub parent_contract: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObligationType {
    #[default]
    Invariant,
    Equivalence,
    Bound,
    Monotonicity,
    Idempotency,
    Linearity,
    Symmetry,
    Associativity,
    Conservation,
    Ordering,
    Completeness,
    Soundness,
    Involution,
    Determinism,
    Roundtrip,
    #[serde(rename = "state_machine")]
    StateMachine,
    Classification,
    Independence,
    Termination,
    /// Memory/IO safety obligation (bounds checks, non-null, etc.). Legacy
    /// pre-APR-MONO contracts (e.g. `apr-cli-publish-extra-v1`) used this
    /// spelling; kept for back-compat alongside the 26 other types.
    Safety,
    /// Liveness property (eventually-happens). Same legacy contract
    /// (`apr-cli-publish-extra-v1`) uses this for progress obligations;
    /// kept for back-compat.
    Liveness,
    // Eiffel DbC types (Meyer 1997)
    Precondition,
    Postcondition,
    Frame,
    #[serde(rename = "loop_invariant")]
    LoopInvariant,
    #[serde(rename = "loop_variant")]
    LoopVariant,
    #[serde(rename = "old_state")]
    OldState,
    Subcontract,
}

const OBLIGATION_TYPE_NAMES: &[(ObligationType, &str)] = &[
    (ObligationType::Invariant, "invariant"),
    (ObligationType::Equivalence, "equivalence"),
    (ObligationType::Bound, "bound"),
    (ObligationType::Monotonicity, "monotonicity"),
    (ObligationType::Idempotency, "idempotency"),
    (ObligationType::Linearity, "linearity"),
    (ObligationType::Symmetry, "symmetry"),
    (ObligationType::Associativity, "associativity"),
    (ObligationType::Conservation, "conservation"),
    (ObligationType::Ordering, "ordering"),
    (ObligationType::Completeness, "completeness"),
    (ObligationType::Soundness, "soundness"),
    (ObligationType::Involution, "involution"),
    (ObligationType::Determinism, "determinism"),
    (ObligationType::Roundtrip, "roundtrip"),
    (ObligationType::StateMachine, "state_machine"),
    (ObligationType::Classification, "classification"),
    (ObligationType::Independence, "independence"),
    (ObligationType::Termination, "termination"),
    (ObligationType::Safety, "safety"),
    (ObligationType::Liveness, "liveness"),
    (ObligationType::Precondition, "precondition"),
    (ObligationType::Postcondition, "postcondition"),
    (ObligationType::Frame, "frame"),
    (ObligationType::LoopInvariant, "loop_invariant"),
    (ObligationType::LoopVariant, "loop_variant"),
    (ObligationType::OldState, "old_state"),
    (ObligationType::Subcontract, "subcontract"),
];

impl std::fmt::Display for ObligationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = OBLIGATION_TYPE_NAMES
            .iter()
            .find(|(kind, _)| kind == self)
            .map_or("", |(_, name)| *name);
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppliesTo {
    All,
    Scalar,
    Simd,
    Converter,
    /// Algorithm-specific target (e.g., "degree", "bce", "huber").
    #[serde(other)]
    Other,
}

/// Accepts `equations:` as either a map (canonical) or a list-of-dicts
/// with an `id` field (legacy pre-APR-MONO diagnostic contracts like
/// `decode-hot-path-prefix-cache-diagnostic-v1`). The list form promotes
/// each entry's `id` to the map key; entries without `id` fall back to
/// `equation_{N}` so parsing never silently drops data.
fn deserialize_equations<'de, D>(d: D) -> Result<BTreeMap<String, Equation>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    use serde_yaml::Value;

    let value = Value::deserialize(d)?;
    match value {
        Value::Null => Ok(BTreeMap::new()),
        Value::Mapping(_) => serde_yaml::from_value(value).map_err(D::Error::custom),
        Value::Sequence(items) => {
            let mut out = BTreeMap::new();
            for (i, mut item) in items.into_iter().enumerate() {
                let key = match &mut item {
                    Value::Mapping(m) => m
                        .remove(Value::String("id".into()))
                        .and_then(|v| v.as_str().map(ToString::to_string))
                        .unwrap_or_else(|| format!("equation_{i}")),
                    _ => format!("equation_{i}"),
                };
                let eq: Equation = serde_yaml::from_value(item).map_err(D::Error::custom)?;
                out.insert(key, eq);
            }
            Ok(out)
        }
        other => Err(D::Error::custom(format!(
            "`equations:` must be a map or a list; got {other:?}"
        ))),
    }
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
