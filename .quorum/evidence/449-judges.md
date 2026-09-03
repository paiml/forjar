# Judge scores — #449 when destroy records its generation

| option | honest | consequence | verdict |
|---|---|---|---|
| before the mutation (snapshot the pre-destroy state directly) | yes | inverts apply's convention; undo after destroy rewinds to a copy of current | rejected |
| **after, through apply's `maybe_record_generation`** | yes | one rule for both verbs; the control proves the rewind | **chosen** |
| a destroy-specific snapshot type | no — a second mechanism for the same question | rejected |
