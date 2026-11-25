Use `contract` to read the contract that a training script saved beside its model,
with the data file, the split, the inputs, the steps, the validation and the model file. It
reads the JSON file and runs no binary. [`gates`](../gates/) reads the same file itself, so call
`contract` when you want the recorded scores or the model's versions.

## Example

With the server's root at `samples/pipeline`:

```json root=samples/pipeline
{"file":"model/contract.json"}
```

The result has `discovery`, `data`, `split`, `inputs`, `steps`, `validation` and `model`. These
are its `validation` and `model`, where each `"..."` stands for a version.

```json
{"validation":{"splitter":{"class":"StratifiedKFold","args":{"n_splits":5,"shuffle":true,"random_state":0}},"scoring":"accuracy","folds":5,"cv_scores":[0.9807692307692307,0.9807692307692307,1.0,1.0,0.9803921568627451],"test_score":0.9883720930232558},"model":{"file":"model/model.joblib","type":"sklearn.pipeline.Pipeline","versions":{"python":"...","scikit-learn":"...","pandas":"...","numpy":"...","joblib":"..."},"code":["features.py"]}}
```

## Errors

A file that is missing or does not match the contract's schema fails with the reason.
