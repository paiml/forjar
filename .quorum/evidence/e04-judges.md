# Judge scores — #406 ciphertext values and the auto-commit exclusion

## A value the redactor cannot name (`ENC[age,…]`)

| option | honest | cost | verdict |
|---|---|---|---|
| decrypt a second time on the reporting path to learn the plaintext | yes | the identity file and a second decrypt after the resource already converged; a failure here would fail a converged apply | rejected |
| **suppress the transcript for any resource carrying a ciphertext marker, as if `sensitive: true`** | yes | the operator loses that one transcript's content; the run directory and status survive | **chosen** |

Under default features nothing changes: an `ENC[` marker is refused outright
without `--features encryption`, so the resource never executes.

## Keeping transcripts out of `--auto-commit`

| option | honest | verdict |
|---|---|---|
| `:(exclude)state/*/runs/` (directory form, the first cut) | **no** — a no-op once one transcript is tracked, which is every repo that ran `--auto-commit` before the fix | rejected |
| **`:(exclude)state/*/runs/*` (per-path), plus `state/*/runs/` in the `init` gitignore, plus the documented `git rm -r --cached` migration** | yes | **chosen** |
