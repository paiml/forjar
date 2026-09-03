# Judge scores — #369 generated ids; policy-coverage re-publication

## Generated id for a rule with no `id:`

| option | honest | consequence | verdict |
|---|---|---|---|
| `RULE-<slug of message>` (status quo) | no — two rules, one id | `remediate --policy-id` applies the sibling; coverage cannot add up | rejected |
| suffix a counter only on collision | no — an id depends on which other rules exist | adding a rule renames its sibling | rejected |
| **`RULE-<index>-<slug>`; explicit `id:` verbatim** | yes | stable under everything but reordering | **chosen** |

## Re-publish `policy-coverage` on the verb surface?

| option | honest | verdict |
|---|---|---|
| re-publish in this branch since the withdrawal reason is gone | answers to the verb-surface suites on every transport; a separate decision | deferred, recorded |
| **keep withdrawn; make the ledger/book say the answer is repaired** | yes | **chosen** |
