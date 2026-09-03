# Judge scores — #367 join placement; #375 wire annotations; outputSchema

## Where the workspace join lives

| option | honest | consequence | verdict |
|---|---|---|---|
| join in every resolver, including the enumerator | no | `state/prod/prod` for the documented workaround; the workspace listing becomes machine dirs | rejected |
| **join on the default branch only; `resolve_state_base` for the enumerator** | yes | explicit `state_dir` verbatim; one listing, one selection | **chosen** |

## Getting annotations onto the wire

| option | honest | verdict |
|---|---|---|
| bump pforge | measured byte-identical adapter | rejected |
| **build the pmcp server in-tree (`src/mcp/adapter.rs`), fill from `effects.read_only()`** | yes | **chosen** |

## Publish `outputSchema`?

| option | honest | verdict |
|---|---|---|
| publish it (first cut) | breaks every call on the official SDK client | rejected on adversarial review |
| **document it in `--schema`, do not promise it on the wire** | yes | **chosen** |
