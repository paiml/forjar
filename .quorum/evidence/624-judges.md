# Judge scores — #624 how the nightly runs on an older-glibc host

| option | honest | consequence | verdict |
|---|---|---|---|
| gnu nightly built in a glibc-2.35 container | yes | two linkage rules, one per channel; breaks again on the next older host | rejected |
| **static musl legs, same targets as release.yml, parity test** | yes | one rule for both channels; a target added to releases alone is RED | **chosen** |
| install nothing nightly on lambda (catalogue skip) | no — hides the WONT-RUN instead of fixing it | the andon goes quiet while lambda never dogfoods a nightly | rejected |

Three judges (the three lanes) scored the chosen option PASS.
