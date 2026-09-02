# Quorum evidence — #416 (CRUX audit E14) — lane summaries

## probe lane
Ran the built binary: `prove` over a fixture whose package obligations are
UNKNOWN exited 0 and printed `[PASS]`; `provenance` printed "SLSA Level 3"
over an unsigned JSON whose `predicateType` still said slsa.dev after the
banner was fixed; `lock-audit-trail` reported on a chain nothing wrote.
Each is a case in the falsifier, RED on main (0 of 3) and RED per hunk.

## crux lane
Terraform `plan -detailed-exitcode` fails the run on anything it could not
settle; Kani and cargo-verify treat UNKNOWN (solver timeout, unsupported
construct) as a hard failure, never as UNREACHABLE; SLSA's own conformance
levels start at a signed, non-falsifiable attestation, so an unsigned JSON
cannot be Level anything; in-toto is explicit that a statement without a DSSE
envelope is untrusted. forjar was below all four; it is now at them on
`prove`, and honest about what `provenance` is.

## design lane
Three defects, three smallest honest changes: UNKNOWN fails (with counts in
the error); the attestation says unsigned in text AND in its JSON predicate
type; the chain and the verb that reported on it are withdrawn rather than
propped up. The `-m` scope for structural invariants is the consequence of
the first change that the review found and the falsifier now pins.

## judges
Two decisions scored: wire the chain vs withdraw; fail UNKNOWN under --json
vs keep exit 0. See the judges file.

## agy /teamwork
Implementation lane (accept-edits) produced the three fixes and the first
falsifier; a second, plan-mode review lane then charged two things (the JSON
predicate type; the stray clap attribute) and found the `-m` isolation break —
all three taken. See the agy file.
