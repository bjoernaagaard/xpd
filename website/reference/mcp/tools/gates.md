Use `gates` to judge a run from a data file to a saved model. It takes the results
that [`profile`](../profile/), [`contract`](../contract/), [`check`](../check/),
[`graph`](../graph/), [`equiv`](../equiv/), [`refit`](../refit/) and
[`discovery_gate`](../discovery_gate/) returned, and checks each gate against them. It reads no
file and runs nothing, so it does not prove that the files are unchanged since those calls. The
run is done when it returns `passed: true`.

| Field of `proof` | The result of |
| --- | --- |
| `discovery` | `discovery_gate` on the training, prediction and step files. Optional |
| `lint` | `check` on the same files |
| `dataflow` | `graph` on the file with the feature steps |
| `reproducible` | `equiv` of the training script with itself |
| `refit` | `refit` on the saved contract |
| `served` | `equiv` of the training script with the script that loads the model |

## Example

In `samples/pipeline`, after `train.py` wrote its model, the results of the other tools passed
all 15 gates.

```json
{"profile":"(profile of penguins.csv)","contract":"(contract of model/contract.json)","proof":{"discovery":"...","lint":"...","dataflow":"...","reproducible":"...","refit":"...","served":"..."}}
```

```json
{"passed":true,"failed":[],"gates":[{"name":"data-is-profiled","stage":"raw data","evidence":["profile","contract"],"passed":true,"detail":"penguins.csv has 344 rows, and the target species has no nulls."},"..."]}
```

The gates are `data-is-profiled`, `split-covers-every-row`, `split-is-seeded`, `inputs-are-file-columns`, `dataflow-connects`, `one-pipeline`, `check-reports-no-error`, `cross-validated`, `reproducible`, `refit-matches-model`, `refit-matches-scores`, `saved-model-predicts`, `versions-recorded`, `indexed-api-calls`, `discovery-is-recorded`. The last two are judged
only when the contract or the proof holds a discovery record.

## Errors

A gate that fails is a result, with its name in `failed` and a `detail` that says why. Arguments
that do not match the schemas fail before any gate is judged.
