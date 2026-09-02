# Judge scores — #405 implement vs withdraw; lock-audit's dead check

## The three verbs that verified nothing

| option | honest | cost | verdict |
|---|---|---|---|
| implement Ed25519 / SLH-DSA behind the existing flags | **no** — a real signature with no answer to "whose key, obtained how"; TUF and cosign exist because that question is the hard part | a signing design, key distribution, rotation | rejected for this ticket |
| keep the verbs, make them error "not implemented" | no — a CI gate on them stays a gate on nothing | low | rejected |
| **withdraw `sign --pq` and `lock-verify-hmac`; rename `sign` to what it is (`digest`); strip the unverified fields** | yes | breaking CLI change, documented in CHANGELOG and the book | **chosen** |

The breaking change was weighed against the alternative of a green gate that
verified nothing, and lost by a wide margin: nobody was protected by the old
verbs, and anyone who thought they were is better told now.

## `lock-audit`'s recompute-and-ignore block

| option | honest | verdict |
|---|---|---|
| make it real: recompute `hash_desired_state` and fail on mismatch | would need the resolved config, which `lock-audit` does not load; and a hash "mismatch" after a config edit is not tampering | rejected |
| **delete it; document `lock-audit` as a format audit and name the verbs that ARE tamper evidence** | yes | **chosen** |

A dead check that looks like tamper detection is the #405 shape in miniature.
