Call `gates` to run all 15 verification checks on a scikit-learn training run using the project's Python environment. Pass the contract path, the three script paths, and optional `discovery_gate` output. The run succeeds when `passed` is `true`.

| Command run by `gates` | Target |
| --- | --- |
| [`xpd profile`](../../../commands/profile/) | Data file named in the contract |
| [`xpd check`](../../../commands/check/) | Training, prediction, and feature scripts |
| [`xpd graph`](../../../commands/graph/) | Feature step script |
| [`xpd equiv`](../../../commands/equiv/) | Training reproducibility and saved-model prediction agreement |
| [`xpd refit`](../../../commands/refit/) | Raw-row and fold refit agreement against contract scores |

## Example

In `samples/pipeline` after running `train.py`:

```json
{"contract":"model/contract.json","scripts":{"train":"train.py","predict":"predict.py","features":"features.py"},"discovery":"(the result of discovery_gate)"}
```

```json
{"passed":true,"failed":[],"gates":[{"name":"data-is-profiled","stage":"raw data","evidence":["profile","contract"],"passed":true,"detail":"penguins.csv has 344 rows, and the target species has no nulls."},"..."]}
```

Each entry in `gates` reports one check verdict, and discovery gates run whenever a discovery record is supplied in the call or the contract.

## Errors

A failed gate returns as a normal result with its name in `failed` and explanation in `detail`. Schema mismatches, unreadable contracts, or scripts missing `result` return a tool error before gates finish.
