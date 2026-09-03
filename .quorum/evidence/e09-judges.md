# Judge scores — #412 one scheduler; proving parity

## Port the five features, or make one scheduler

| option | honest | consequence | verdict |
|---|---|---|---|
| port --retry, meta.yaml map, --trace, --progress, input cache to the wave path | yes | two implementations remain; #393/#394 showed the next drift is one feature away | rejected |
| **sequential = wave scheduler at width 1, plan order** | yes | one implementation; the parity tests stay meaningful | **chosen** |

## Proving parity

| option | honest | verdict |
|---|---|---|
| assert each feature "present" on both paths | no — presence is not sameness | rejected |
| **run the same fixture through both paths and compare the artefacts (hooks, lock, events, run log, retry counts, trace, progress, cache)** | yes | **chosen** |
