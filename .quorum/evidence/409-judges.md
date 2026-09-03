# Judge scores — #409 what to do with `store: true`; #410 the fictional steps

## E06: make the store real, or stop scoring it

| option | honest | consequence | verdict |
|---|---|---|---|
| (a) put the store on the apply path | yes | is the E07 delegation (namespace, hash-dir, atomic move) — not small; blocks on binaries that do not exist | rejected for this pass |
| **(b) stop scoring it; say "declared, not enforced" on every surface** | yes | every printed number is true today; schema unchanged | **chosen** |

## E07: the steps whose binaries do not exist

| option | honest | verdict |
|---|---|---|
| delete them from the plan (first cut) | hides the lifecycle; broke eight suites that describe it | rejected |
| **keep them, `command: None`, description says NOT EXECUTABLE and why; execution refuses by name** | yes | **chosen** |
