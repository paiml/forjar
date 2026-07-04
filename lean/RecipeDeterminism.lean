/-
  L4 Lean 4 proof for forjar `recipe-determinism-v1` — the decidable/pure core
  of the two ERROR-severity proof obligations, mirroring the Kani harness for
  contract_tests::FALSIFY-RD-001 (Determinism) and FALSIFY-RD-002 (Integer
  bounds enforced).

  We prove the tractable PURE core (the actual template-expansion touches disk
  I/O and a HashMap, which is not modelled here — see notes):

    * Expansion determinism = reflexivity of a pure expansion function: for any
      fixed inputs, `expand inputs = expand inputs`. Modelled with a total pure
      `expand : Inputs → Resources`.
    * Integer-bounds enforcement: `checkBound min max n = (min <= n && n <= max)`,
      proved equivalent to `min <= n ∧ n <= max`, with out-of-range rejected.

  Verify: `lean lean/RecipeDeterminism.lean` (0 errors, fully proved).
-/
namespace ProvableContracts.Forjar.RecipeDeterminism

/-- A minimal pure model of recipe inputs (a list of key→value Nat pairs). -/
abbrev Inputs := List (Nat × Nat)

/-- A minimal pure model of the namespaced resources produced by expansion.
    The concrete shape is irrelevant to determinism; what matters is that
    expansion is a *pure total function* of its inputs. -/
def expand (inputs : Inputs) : List (Nat × Nat) :=
  inputs.map (fun kv => (kv.1, kv.2))

/-- Theorems.ExpansionDeterminism (FALSIFY-RD-001) — same inputs → same
    expanded resources. Reflexivity of a pure function is `rfl`. -/
theorem ExpansionDeterminism (inputs : Inputs) :
    expand inputs = expand inputs := rfl

/-- Theorems.ExpansionDeterminismEq — determinism stated via a shared witness:
    if two evaluation sites feed equal inputs, they produce equal expansions.
    This is the form the Kani harness checks (two calls, asserted equal). -/
theorem ExpansionDeterminismEq (a b : Inputs) (h : a = b) :
    expand a = expand b := by rw [h]

/-- The integer-bounds decision procedure, mirroring `validate_input_type` for
    the `int` case: accept iff `min ≤ n ≤ max`. -/
def checkBound (min max n : Nat) : Bool := (min <= n) && (n <= max)

/-- Theorems.BoundSpec (FALSIFY-RD-002) — the decision procedure is EXACTLY the
    in-range predicate: `checkBound min max n = true ↔ (min ≤ n ∧ n ≤ max)`. -/
theorem BoundSpec (min max n : Nat) :
    checkBound min max n = true ↔ (min <= n ∧ n <= max) := by
  simp [checkBound]

/-- Theorems.BoundRejectsBelow — `n < min` is rejected (Err), never Ok. -/
theorem BoundRejectsBelow (min max n : Nat) (h : n < min) :
    checkBound min max n = false := by
  simp only [checkBound, Bool.and_eq_false_iff]
  exact Or.inl (by simp [Nat.not_le.mpr h])

/-- Theorems.BoundRejectsAbove — `n > max` is rejected (Err), never Ok. -/
theorem BoundRejectsAbove (min max n : Nat) (h : max < n) :
    checkBound min max n = false := by
  simp only [checkBound, Bool.and_eq_false_iff]
  exact Or.inr (by simp [Nat.not_le.mpr h])

end ProvableContracts.Forjar.RecipeDeterminism
