# Judge scores — #364 where the yank denial lives

| option | honest | consequence | verdict |
|---|---|---|---|
| `yanked = "deny"` in deny.toml | no — measured blind (`all-features = false`) | exits 0 without seeing the crate | rejected |
| `all-features = true` in deny.toml | changes what every other deny check sees; optional GPU/WASM deps enter the license and ban graphs | rejected for this ticket |
| **cargo-audit `--deny yanked` + a lockfile-reading test** | yes | a yanked pin fails locally and in the lane | **chosen** |
