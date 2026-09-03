# Judge scores — #374 gate position; the read-only line

## Where the operator gate lives

| option | honest | consequence | verdict |
|---|---|---|---|
| add the check to each early exit (#370's shape, ×4) | yes | a gate each exit must remember; the next exit forgets | rejected |
| **one positional check in `dispatch_apply_cmd`, before any exit/hook/backup** | yes | a new mode is gated unless it is named as a read | **chosen** |

## Gate the reads?

| option | honest | verdict |
|---|---|---|
| gate everything | measured cost: a listed operator loses `apply -m theirs --check` (the check iterates every machine); no confidentiality gain | rejected |
| **reads stay open; an invocation carrying a hook is not a read** | yes | **chosen** |
