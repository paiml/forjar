# Judge scores — #403 hash identity

| option | honest | complete | fleet cost | verdict |
|---|---|---|---|---|
| keep the allowlist, add the 74 missing fields by hand | yes | **no** — the sixth piecemeal patch; nothing stops the seventh | one re-converge | rejected |
| hash the generated apply/check/destroy scripts (the FJ-036 route, generalised) | **no** — a script carries resolved secrets and per-host paths, so the hash leaks and forks per host | yes | one re-converge | rejected |
| **canonical serialisation minus an explicit denylist + reflection guard** | yes | yes — `tests_hash_completeness` fails on any field that is neither hashed nor denylisted | one re-converge, plus one more every time a field is ADDED to `Resource` | **chosen** |

The cost column is the honest part: because `Resource` has no
`skip_serializing_if`, adding a field moves every recorded hash. The judges
weighed skipping default-valued keys and rejected it — a field whose DEFAULT
later changes would then not re-converge, which is this ticket's defect one
level up. The loud side is kept and the release note says so.

Refutation attempted by the panel and RECORDED, not smoothed over: the first
injectivity counterexample offered (`a` / `a!b`) did not collide, because
`Tag`'s Display already carries a leading `!`; the corrected pair (`a` /
`a!!b`) did, and is now pinned.
