# Judge scores — #408 the literal form; prefix vs separate flag

## Keep accepting an inline literal?

| option | honest | cost | verdict |
|---|---|---|---|
| remove it now | no — every existing signature becomes unverifiable with no migration path | breaks every signed fleet | rejected |
| **keep it, warn on every use naming the flag and the removal version (2.0.0)** | yes | a warning line per use during the transition | **chosen** |

## `file:`/`env:` prefixes vs `--key-file` / `--key-env` flags

| option | honest | cost | verdict |
|---|---|---|---|
| separate flags on four commands (eight new flags) | yes | eight flags, four mutually-exclusive groups, and rotate-keys needs both for two keys | rejected |
| **a prefix on the existing flag, resolved in one place** | yes | a literal that starts with `file:`/`env:` must move to the indirect form (it should anyway) | **chosen** |
