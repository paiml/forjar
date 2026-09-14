# PMAT-537 — pmat measurements

## The defect, from GitHub

On `7c100454`, the commit v1.29.0's ledger row named:

| file | says |
|---|---|
| `Cargo.toml` | `forjar = { version = "1.2", default-features = false }` |
| `Cargo.lock` | `name = "forjar"` / `version = "1.2.1"` |

On `0be3e1ec`, the commit it names now:

| file | says |
|---|---|
| `Cargo.toml` | `forjar = { version = "1.29", default-features = false }` |
| `Cargo.lock` | `name = "forjar"` / `version = "1.29.0"` |

## The gate, before and after

```
GATE T FAIL v1.29.0: the cookbook at 7c100454… LOCKS forjar 1.2.1 and the
release is 1.29.0 — the requirement 1.2 admits it, but a requirement is a range
and the lock is what cargo builds, so that cookbook was never compiled against
this release.
```

```
GATE T v1.29.0 cookbook 0be3e1ec… requires forjar 1.29 and locks 1.29.0 ok
GATE T PASS 7 tagged release(s) since v1.25.0 reconcile with git and GitHub…
```

Both in `docs/audits/logs/PMAT-537-cookbook-lock.log`, 459 lines. Its first
version was 9 lines and contained neither, because the script that wrote it
piped `tee` into `head`; three review lanes opened the file and refused the
receipt for it.

## The cookbook bump

paiml/forjar-cookbook#20 → `0be3e1ec`:

| check | result |
|---|---|
| `cargo build --workspace --locked` | clean |
| `cargo clippy --workspace --locked --all-targets -- -D warnings` | silent |
| `cargo test --workspace --locked --no-fail-fast` | 6 + 12 + 72 + 55 passed |
| its own CI | 15 checks green |
| source changes | **none** across twenty-seven minor versions |

## Vacuity

`pmat analyze vacuous-tests`: 400 of 19650 `#[test]` fns cannot fail (2.0%)
across 2164 parsed files. None of the five new cases appears — each drives the
real gate over a fixture repository with a stubbed `gh` and asserts on the
verdict's own text.

## bashrs

`bashrs lint scripts/dogfood/tagged.sh`: **0 errors**. One `local` declared
mid-body produced SC2168 on the first attempt and was hoisted rather than
suppressed, the same correction as PMAT-241's.
