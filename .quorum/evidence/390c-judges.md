# Judge scores — #390-C exit_code

| option | honest | cost | verdict |
|---|---|---|---|
| widen `record_failure` to take an exit code | yes | 6 call sites, several with no exit code (transport error, pre_apply gate) | deferred |
| parse the code out of the error string | **no** — reconstructs a value from prose | low | rejected outright |
| **leave `exit_code: None`, name it** | **yes** | none | **chosen** |

A faked exit code is worse than an absent one: a consumer can branch on null, but
cannot detect a wrong number. Recorded in the receipt's known_limits.
