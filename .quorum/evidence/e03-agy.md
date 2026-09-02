# Independent review — agy /teamwork (plan mode) — #405

Verdict as delivered: the branch "successfully removes the fraudulent
`verify` commands and correctly re-labels `sign` to `digest`", but "fails to
achieve robust cryptographic verification for all remaining paths" and its
"falsification tests introduced are vacuous".

TAKEN (changed the branch):

> "The `audit_lock_integrity` function in `lock-audit` detects mismatched
> hashes for individual resources but explicitly chooses to 'flag it but
> don't fail' … allowing the audit to exit 0 despite detecting inner-lock
> tampering."

Sharper than stated: the recompute compared against a value that could never
match (`blake3("{name}:{status}")` is not how lock hashes are made), so it
detected nothing and looked like it did. Deleted, with the function
documented as a format audit and the real tamper-evidence verbs named.

> "the tests pass vacuously via the `withdrawn()` helper which blindly
> succeeds on `clap` parsing errors like 'unexpected argument', meaning
> regressions could hide behind minor CLI argument changes."

Correct. The helper now takes the withdrawn name and requires clap's message
to quote it exactly.

CONFIRMED BY THE REVIEWER (attacked and survived):

> "`lock-verify-sig` computes `blake3(content ++ key)`, successfully binding
> the lock's integrity to an out-of-band secret."

> "`lock-verify-sig` strictly uses the provided `--key`, and `digest --verify`
> strictly enforces that the self-reported algorithm is exactly `blake3`."

REFUTED (did not survive the code):

> "`digest --verify` and `lock-audit` still pass on a hash that the attacker
> controls."

True of the mechanism, and exactly what `digest` says about itself in its
module header and its help text: tamper evidence, not a signature, no key.
The charge assumes a verification claim the verb no longer makes. Unkeyed
integrity is what `sha256sum -c` is; the keyed check is `lock-verify-sig`.

> "The PQ path was never real … the entire module has now been deleted."

Agreed, and that is the branch, not a refutation of it.

Its CRUX survey placed the surviving design AT the industry default on all
four systems (cosign, TUF, git, apt): keys and algorithms from the operator
or an out-of-band root, never from the artifact.
