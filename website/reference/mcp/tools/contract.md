Use `contract` to read the contract that a training script saved beside its model,
with the data file, the split, the inputs, the steps, the validation and the model file. It
reads the JSON file and runs no binary. Pass its result to [`gates`](../gates/).

## Example

With the server's root at `samples/pipeline`:

```json
{"file":"model/contract.json"}
```

The result has `discovery`, `data`, `split`, `inputs`, `steps`, `validation` and `model`. These
are its `validation` and `model`.

```json
{"validation":{"splitter":{"class":"StratifiedKFold","args":{"n_splits":5,"shuffle":true,"random_state":0}},"scoring":"accuracy","folds":5,"cv_scores":[0.9807692307692307,0.9807692307692307,1.0,1.0,0.9803921568627451],"test_score":0.9883720930232558},"model":{"file":"model/model.joblib","type":"sklearn.pipeline.Pipeline","versions":{"python":"3.14.10","scikit-learn":"1.9.1","pandas":"3.1.1","numpy":"2.5.5","joblib":"1.6.0"},"code":["features.py"]}}
```

## Errors

A file that is missing or does not match the contract's schema fails with the reason.
