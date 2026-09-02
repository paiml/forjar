# Quorum evidence — #408 (CRUX audit E13) — lane summaries

## probe lane
Ran every key-taking verb through the binary, because argv and exit codes
are properties of a process: `lock-sign --key file:<p>` on unfixed main
signed with the literal string `file:<p>` (a signature of the wrong bytes,
exit 0, "Signed 1 lock file(s)"); `--key file:/nope` did the same over a
file that does not exist. Then mutated the fixed tree: with the verifier's
resolve line deleted all ten first-cut tests stayed green, which is how the
eleventh was found.

## crux lane
`ssh` refuses a private key from argv and reads it from a file whose mode it
checks; `gpg` takes `--passphrase-file`/`--passphrase-fd` and warns on
`--passphrase`; `age` takes `-i <identity file>` only; `cosign` reads
`COSIGN_PASSWORD` from the environment and the key from a file or KMS ref;
`docker login --password` prints "WARNING! Using --password via the CLI is
insecure. Use --password-stdin." — a decade-old warning with the same
shape as this one. Every surveyed tool has an indirect form and most have
retired the direct one. forjar was below that bar and is now at it, with
the literal kept only for the migration and marked for removal.

## design lane
One resolver, three forms, no fallback. The verify side had to resolve too
— a signer and a verifier that both hash the wrong bytes agree with each
other, which is why the cross-form test exists. The unattended surfaces
(MCP/HTTP) are untouched: they never took key material.

## judges
Two decisions scored: keep the literal or not, and prefix syntax vs a
separate `--key-file` flag. See the judges file.

## agy /teamwork
Independent stack review; see the agy file.
