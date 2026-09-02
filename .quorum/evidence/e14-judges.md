# Judge scores — #416 the chain; UNKNOWN under --json

## `tripwire::chain` — wire in or withdraw

| option | honest | cost | verdict |
|---|---|---|---|
| wire `append_event` into a hash chain and keep `lock-audit-trail` | only with a trust root, a verifier and a gap policy the ticket does not specify | a feature | rejected for this ticket |
| **withdraw the module and the verb; point the book at `lock-history`, `lock-verify`, `lock-verify-sig`** | yes | a verb nobody could rely on is gone | **chosen [A]** |

## UNKNOWN under `--json`

| option | honest | verdict |
|---|---|---|
| exit 0, state in the payload | no — a CI consumer branches on the exit code first | rejected |
| **non-zero on every path; counts of UNKNOWN/FALSIFIED in the error; state in the payload** | yes | **chosen** |
