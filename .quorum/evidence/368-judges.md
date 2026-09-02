# Judge scores — #363 hash shape; #378 forced count

## Which config does `config_hash` describe

| option | honest | consequence | verdict |
|---|---|---|---|
| hash the stripped config on both sides | yes | apply must reproduce the plan's narrowing exactly; a plan made with `--target` is applied under the same narrowing | rejected — more state to carry, and the plan body already carries the narrowed change list |
| **hash one sealed snapshot taken before any narrowing** | yes | the hash names the file the operator reviewed; both sides compute it from the same snapshot | **chosen** |

## Where the forced no-op count comes from

| option | honest | verdict |
|---|---|---|
| widen the assert so `forced_noop > converged` is allowed | no — hides the distinguishability failure the contract exists to catch | rejected |
| **count candidates before the run, keep only those that actually ran and converged** | yes | **chosen** |
