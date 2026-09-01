# Judge scores — #390-E delimiter strategy

Three candidate strategies for collision-free heredoc delimiters, scored on
determinism (recipe-determinism-v1 requires the script be a pure function of the
declaration), readability of the common case, and provable absence.

| strategy | deterministic | common case | absence proven | verdict |
|---|---|---|---|---|
| blake3 suffix of the body | yes | opaque `FORJAR_TIMEOUT_a91f...` for every task | yes | rejected — every script pays for a rare case |
| always append a fixed nonce | yes | opaque, same objection | no — nonce could still appear | rejected |
| **extend only on collision** | **yes** | **plain `FORJAR_TIMEOUT`** | **yes, by loop termination** | **chosen** |

The chosen loop terminates because each iteration lengthens the delimiter and the
body is finite. Substring containment is checked rather than whole-line equality:
only a bare line can actually terminate a heredoc, so containment is strictly
safer and costs only a longer delimiter in a case that is already rare.
