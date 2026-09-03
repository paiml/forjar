# Judge scores — #360 how to ignore one field; #362 how to replace a cron job

## Ignoring one field of an observation

| option | honest | consequence | verdict |
|---|---|---|---|
| per-field `observed` map in the lock (the issue's proposal) | yes | schema bump, migration, every generator changes; `lock_core`'s schema checks trip | rejected |
| **mask named `key=value` tokens out of the stdout before hashing, at all three writers; record the mask** | yes | no schema change; a stale baseline is censused, not reported | **chosen** |

## Replacing a managed cron entry

| option | honest | verdict |
|---|---|---|
| keep the two `grep -v`s and add a third for the entry | substring match; orphans siblings | rejected |
| **one awk that deletes the intact block, exact-line, markers via ENVIRON** | deletes only what forjar wrote; a hand-edited block is left (and #445 asks to refuse by name) | **chosen** |
