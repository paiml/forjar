# Quorum evidence — #405 (CRUX audit E03) — lane summaries

## probe lane
Forged the sidecar fields by hand on the unfixed binary and read what each
verb printed: `sign --verify` said valid over `deadbeef`; `sign --pq --verify`
said both valid over two forgeries; `lock-verify-hmac` said `verified: 1`
over the literal bytes `not-a-signature`. Then, on the first cut, forged the
algorithm name in the digest sidecar and watched `digest --verify` echo it
back beside `valid: true`. Every one of those payloads is quoted in the test
that pins it.

## crux lane
cosign trusts nothing self-reported in the artifact — key and algorithm come
from Fulcio/Rekor or the operator. TUF pins keys, algorithms and thresholds in
an out-of-band `root.json`. git looks a signature's key id up in a local
keyring or `allowed_signers` and trusts the keyring, not the id. apt trusts
the hashes in `Release` only after the detached signature verifies against
`/etc/apt/trusted.gpg.d/`. The one keyed verb that survives here takes its
key from the operator and trusts nothing in the file; the one unkeyed verb
says it is unkeyed. Below the default was a verb that read a signature and
checked a hash; at the default is a verb that reads nothing it does not check.

## design lane
Subtract, do not implement. There was no key material anywhere, so "sign"
could only ever have been a hash; a signature scheme needs a key-distribution
answer this codebase does not have and this ticket does not ask for. The
sidecar's unverified `signature`, `signer` and `algorithm: blake3-hmac` fields
are removed rather than left in place — a consumer reading them was reading a
lie. `digest --verify` requires the recorded algorithm to equal the one it
runs and reports the one it ran.

## judges
Two decisions scored: implement vs withdraw, and whether `lock-audit`'s dead
recompute should be made real or deleted. See the judges file.

## agy /teamwork
Independent stack review in plan mode. Confirmed key-binding and no-self-trust;
charged that the unkeyed verbs "fail verification" (refuted: they claim none);
found the dead recompute in `lock-audit` and the over-broad `withdrawn()`
helper — both taken. See the agy file.
