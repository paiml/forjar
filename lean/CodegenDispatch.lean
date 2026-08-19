/-
  L4 Lean 4 proof for forjar `codegen-dispatch-v1` — the decidable pure core of
  the dispatch completeness + symmetry obligations, mirroring the Kani harnesses
  KANI-CD-001 (all Phase 1 types dispatched) and KANI-CD-002 (dispatch symmetric
  across check/apply/state_query).

  We model ResType = Package | File | Service | Mount and three total functions
  checkOk / applyOk / stateOk : ResType → Bool, exactly as the codegen pipeline
  dispatches (§6.3 Shell Generation Pipeline). The I/O part (non-empty shell
  string on disk) is not modelled — we prove the DECISION core: every Phase 1
  type dispatches Ok, and the three functions agree pointwise.

  Verify: `lean lean/CodegenDispatch.lean` (0 errors, no sorry).
-/
namespace ProvableContracts.Forjar.CodegenDispatch

/-- The four Phase 1 resource types. -/
inductive ResType
  | Package
  | File
  | Service
  | Mount

/-- check_script dispatch: Ok (true) for every Phase 1 type. -/
def checkOk : ResType → Bool
  | .Package => true
  | .File    => true
  | .Service => true
  | .Mount   => true

/-- apply_script dispatch: same total coverage. -/
def applyOk : ResType → Bool
  | .Package => true
  | .File    => true
  | .Service => true
  | .Mount   => true

/-- state_query_script dispatch: same total coverage. -/
def stateOk : ResType → Bool
  | .Package => true
  | .File    => true
  | .Service => true
  | .Mount   => true

/-- Theorems.Completeness (KANI-CD-001) — check_script returns Ok for every
    Phase 1 resource type; dispatch is total (no missing match arm). -/
theorem Completeness : ∀ t : ResType, checkOk t = true := by
  intro t; cases t <;> rfl

/-- Theorems.SymmetryCheckApply (KANI-CD-002, part 1) — check and apply agree
    pointwise: one never handles a type the other misses. -/
theorem SymmetryCheckApply : ∀ t : ResType, checkOk t = applyOk t := by
  intro t; cases t <;> rfl

/-- Theorems.SymmetryApplyState (KANI-CD-002, part 2) — apply and state_query
    agree pointwise, completing the symmetry chain. -/
theorem SymmetryApplyState : ∀ t : ResType, applyOk t = stateOk t := by
  intro t; cases t <;> rfl

/-- Theorems.DispatchTotalSymmetric — the full contract obligation: for every
    resource type, check succeeds AND all three codegen functions agree. This is
    the exact conjunction the Kani harness asserts over bound 4. -/
theorem DispatchTotalSymmetric :
    ∀ t : ResType, checkOk t = true ∧ checkOk t = applyOk t ∧ applyOk t = stateOk t := by
  intro t
  cases t <;> exact ⟨rfl, rfl, rfl⟩

/-- Theorems.SymmetryTransitive — the derived three-way agreement check = state,
    proving the ⟺ chain is closed (check ⟺ apply ⟺ state on Ok). -/
theorem SymmetryTransitive : ∀ t : ResType, checkOk t = stateOk t := by
  intro t
  have h1 := SymmetryCheckApply t
  have h2 := SymmetryApplyState t
  rw [h1, h2]

end ProvableContracts.Forjar.CodegenDispatch
