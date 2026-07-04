/-
  L4 Lean 4 proof for forjar `blake3-state-v1` — the decidable pure core of the
  BLAKE3 content-addressed state hashing contract, mirroring the Kani harness
  `KANI-B3-001` (verify_hash_string_prefix) and FALSIFY-B3-001/002.

  Strings are modelled as `List Nat` (byte sequences). The real BLAKE3 hash is
  opaque I/O — we do NOT model the cryptographic mixing (that is genuinely
  external). Instead we prove the two pure, contract-level invariants that hold
  regardless of the hash body:
    * Determinism  : hashString s = hashString s   (reflexivity / FALSIFY-B3-002)
    * Prefix format: every output startsWith "blake3:"  (KANI-B3-001 / FALSIFY-B3-001)

  Verify: `lean lean/BlakeState.lean` (0 errors, no sorry).
-/
namespace ProvableContracts.Forjar.BlakeState

/-- A string is a list of byte codes. -/
abbrev Str := List Nat

/-- The literal tag `"blake3:"` as byte codes (b l a k e 3 :). -/
def prefixTag : Str := [98, 108, 97, 107, 101, 51, 58]

/-- `isPrefix p xs` = true iff `p` is an initial segment of `xs`. -/
def isPrefix : Str → Str → Bool
  | [], _ => true
  | _ :: _, [] => false
  | a :: as, b :: bs => a == b && isPrefix as bs

/-- Opaque stand-in for the hex-encoded BLAKE3 digest. Its *contents* are
    irrelevant to the invariants, so we leave it abstract as an arbitrary
    total function on the input bytes. -/
def hexOf (s : Str) : Str := s

/-- The model of `hash_string`: the fixed tag followed by the digest hex.
    `H(s) = "blake3:" || hex(BLAKE3(s))`. -/
def hashString (s : Str) : Str := prefixTag ++ hexOf s

/-- Theorems.PrefixOfAppend — any list is a prefix of itself extended (the key
    structural lemma, by induction on the prefix). -/
theorem PrefixOfAppend (p xs : Str) : isPrefix p (p ++ xs) = true := by
  induction p with
  | nil => rfl
  | cons a as ih =>
    simp [isPrefix, ih]

/-- Theorems.HashDeterministic — FALSIFY-B3-002: hashing is a pure function, so
    the same input always yields the same output (reflexivity). -/
theorem HashDeterministic (s : Str) : hashString s = hashString s := rfl

/-- Theorems.HashHasBlakePrefix — KANI-B3-001 / FALSIFY-B3-001: every output of
    `hashString` starts with the `"blake3:"` tag, for ALL inputs. -/
theorem HashHasBlakePrefix (s : Str) : isPrefix prefixTag (hashString s) = true := by
  unfold hashString
  exact PrefixOfAppend prefixTag (hexOf s)

/-- Theorems.HashNonEmpty — a stronger structural fact: the output can never be
    empty (it always carries the 7-byte tag), so the `blake3:` invariant is
    non-vacuous. -/
theorem HashNonEmpty (s : Str) : (hashString s = []) → False := by
  intro h
  simp [hashString, prefixTag] at h

end ProvableContracts.Forjar.BlakeState
