# Judge scores — #423 own it vs patch it; plain vs feature-gated

## Move the crate in, or overlay an external one

| option | honest | consequence | verdict |
|---|---|---|---|
| `[patch.crates-io]` / `cargo vendor` pointing at a copy that keeps the upstream name | yes | still an external crate by name; forjar's manifest keeps saying `aprender-contracts`; nothing to publish | rejected — the instruction was to MOVE it in |
| **workspace members `forjar-contracts*`, `[lib]` names unchanged, published in order at release** | yes | two more crates to publish; VENDORED.md per crate | **chosen** |

## Plain dependencies or a `contracts` feature

| option | honest | verdict |
|---|---|---|
| default-on `contracts` feature, deps optional, build.rs gated (first cut) | no — `--no-default-features` cannot compile forjar; the CI job that checks it would go red | rejected |
| **plain required path deps, pinned `=0.31.2`, as the registry deps were** | yes | **chosen** |
