/-
  L4 Lean 4 proof for forjar `provable-iac-v1` — the decidable core of I3
  conflict-freedom, mirroring the Kani harness `provable-iac-kani-001`
  (core::prove::checkers::has_conflict). Keys modelled as a List Nat.
  Verify: `lean lean/ProvableIac.lean` (0 errors, no sorry).
-/
namespace ProvableContracts.Forjar.ProvableIac

/-- The conflict detector: a conflict exists iff the head key repeats later, or
    the tail already conflicts (the exact recursion of has_conflict). -/
def hasConflict : List Nat → Bool
  | [] => false
  | k :: rest => rest.contains k || hasConflict rest

/-- Theorems.ConflictDetectorSpec — the recursive specification (definitional). -/
theorem ConflictDetectorSpec (k : Nat) (rest : List Nat) :
    hasConflict (k :: rest) = (rest.contains k || hasConflict rest) := rfl

/-- Theorems.EmptyNoConflict — the empty config has no target collision. -/
theorem EmptyNoConflict : hasConflict [] = false := rfl

/-- Theorems.SingletonNoConflict — one writer never collides with itself. -/
theorem SingletonNoConflict (k : Nat) : hasConflict [k] = false := by
  simp [hasConflict]

/-- Theorems.NoConflictHeadFresh (soundness) — if `prove` reports NO conflict,
    then the first writer's key does not reappear among the rest: the green
    result genuinely implies target-disjointness of the head. -/
theorem NoConflictHeadFresh (k : Nat) (rest : List Nat) :
    hasConflict (k :: rest) = false → rest.contains k = false := by
  intro h
  simp only [hasConflict, Bool.or_eq_false_iff] at h
  exact h.1

/-- Theorems.NoConflictTailFree (soundness) — a no-conflict verdict also implies
    the tail is itself conflict-free (the property holds hereditarily). -/
theorem NoConflictTailFree (k : Nat) (rest : List Nat) :
    hasConflict (k :: rest) = false → hasConflict rest = false := by
  intro h
  simp only [hasConflict, Bool.or_eq_false_iff] at h
  exact h.2

/-- Theorems.ConflictInheritsFromTail (completeness) — a collision in the tail is
    never masked: the detector still fires for the whole list. -/
theorem ConflictInheritsFromTail (k : Nat) (rest : List Nat) :
    hasConflict rest = true → hasConflict (k :: rest) = true := by
  intro h
  simp [hasConflict, h]

/-- Theorems.DuplicateHeadConflicts (completeness) — a head key that reappears
    later is always detected. -/
theorem DuplicateHeadConflicts (k : Nat) (rest : List Nat) :
    rest.contains k = true → hasConflict (k :: rest) = true := by
  intro h
  simp only [hasConflict, h, Bool.true_or]

end ProvableContracts.Forjar.ProvableIac
