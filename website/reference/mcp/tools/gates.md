Use `gates` to judge a run from a data file to a saved model. It runs the commands below on the
files you name and checks each gate against what they print, so you pass it no result of another
command. It runs the scripts with the project's Python. The run is done when it returns
`passed: true`.

| What `gates` runs | On |
| --- | --- |
| [`xpd profile`](../../../commands/profile/) | The data file that the contract names |
| [`xpd check`](../../../commands/check/) | The training script, the prediction script and the step file |
| [`xpd graph`](../../../commands/graph/) | The file with the feature steps |
| [`xpd equiv`](../../../commands/equiv/) | The training script with itself, and with the script that loads the model |
| [`xpd refit`](../../../commands/refit/) | The contract |

`discovery` is optional, and it is the result of [`discovery_gate`](../discovery_gate/) on the
same three files.

## Example

In `samples/pipeline`, after `train.py` wrote its model, the call below passed every gate.

```json
{"contract":"model/contract.json","scripts":{"train":"train.py","predict":"predict.py","features":"features.py"},"discovery":"(the result of discovery_gate)"}
```

```json
{"passed":true,"failed":[],"gates":[{"name":"data-is-profiled","stage":"raw data","evidence":["profile","contract"],"passed":true,"detail":"penguins.csv has 344 rows, and the target species has no nulls."},"..."]}
```

Each entry in `gates` corresponds to one requirement in the description above; the discovery gates
are judged only when the contract or the call holds a discovery record.

## Errors

A gate that fails is a result, with its name in `failed` and a `detail` that says why. Arguments
that do not match the schemas fail before any gate is judged, and so does a contract that cannot
be read or a script that leaves no `result`.
