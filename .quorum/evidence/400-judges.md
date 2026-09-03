# Judge scores — #401 what stays tracked; #400 the cargo step

## `.pmat/baseline.json` and friends

| option | honest | consequence | verdict |
|---|---|---|---|
| keep tracked, re-include with `!` under `**/.pmat/**` | no — measured to lose to pmat's own `.pmat/.gitignore` (`*`) | a rule that differs between CI and any machine pmat has run on | rejected |
| **nothing under `.pmat/` is tracked; the CB-200 ceiling moves to `scripts/ratchets/`** | yes | one owner per directory; the ratchet reads the path from the script | **chosen** |

## The `cargo test` step on a cross-branch push

| option | honest | verdict |
|---|---|---|
| `git worktree add` at the pushed sha and build there | yes, but a second target dir on a fleet that races on one already | rejected for cost |
| **skip loudly, name the trade in the code, rely on quorum.yml on the PR head** | yes | **chosen** |
