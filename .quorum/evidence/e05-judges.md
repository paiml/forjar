# Judge scores — #407 falsifier placement; task checks under a read-only verb

## Where the falsifier lives

| option | honest | verdict |
|---|---|---|
| the `mcp::tests_drift_e05` unit suite only (the first cut) | partial — proves the handler, not the process; the gate cannot bind to it | kept as the inner loop |
| **a binary-level suite through `forjar verb call drift --json`** | yes — an agent's answer is stdout + exit code | **chosen** |

## A config-declared `completion_check` under `readOnlyHint: true`

| option | honest | risk | verdict |
|---|---|---|---|
| run it, as the CLI does | no — `readOnlyHint` promises the opposite | an untrusted checkout with its own `state/` executes shell on the controller | rejected |
| run it, but only for non-local machines | no — the promise is about WHAT executes, not where | same, one hop away | rejected |
| **decline, and disclose on the census AND by name** | yes | a task's convergence is reported as "not checked", never as clean | **chosen** |
