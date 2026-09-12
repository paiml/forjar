# PMAT-240 — pmat measurements

## The class, counted

| | |
|---|---|
| sites PMAT-239 fixed | 3 |
| files PMAT-239's rule covered | 3 |
| sites its census listed as remaining | 18 |
| sites a grep found today | 13, one of them a comment |
| sites the RULE found on `origin/main` | **15** |
| sites closed here | 15 |

The three the census never listed are three-stage pipelines whose fatal end is
not the stage after the first pipe:

```
scripts/dogfood/crux-reconcile.sh:74   printf | sed -n | head -1
scripts/publish-from-tag.sh:88         git show | manifest_version | head -n1
scripts/dogfood/release-check.sh:195   git tag | grep -vxF | head -1
```

## Red and green

In a scratch clone with its own `CARGO_TARGET_DIR`, over main's `scripts/`:
**15 pipelines**, each named with file, line and text. Green at HEAD.

The FIRST clone reported 10 — it had reused a stale test binary from the shared
target directory, and then a second one carried a rule that predated two
uncommitted fixes. A proof that reuses a cached binary is measuring the cache,
and a proof built from an uncommitted tree is measuring the last commit.

## The gates

Six pass at HEAD: B, D, G, H (pending, correctly — no cut is in flight), T, R.
They were **not** re-run against main's scripts, which the receipt now says
rather than implying "either way".

## bashrs

Every touched script at **0 errors** except `scripts/ledger-replay.sh`, whose
single SEC011 is pre-existing and is the one gate B's
`BASHRS_ERROR_CEILING=1` accounts for.

## Vacuity

`pmat analyze vacuous-tests`: 400 of 19650 `#[test]` fns cannot fail (2.0%).
None of the three cases here appears — and one of them WAS vacuous in a way the
analyzer does not model, planting its evidence at the end of a file where a
rule reading only the last line would pass. A review lane caught that; the
analyzer did not.
