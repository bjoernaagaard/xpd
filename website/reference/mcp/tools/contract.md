Call `contract` to read and validate the JSON contract a training script wrote beside its saved model without running the `xpd` binary. [`gates`](../gates/) reads the same contract automatically during verification.

## Example

With the server root at `samples/pipeline`:

```json root=samples/pipeline
{"file":"model/contract.json"}
```

The returned object holds `discovery`, `data`, `split`, `inputs`, `steps`, `validation`, and `model`.

```json
{"validation":{"splitter":{"class":"StratifiedKFold","args":{"n_splits":5,"shuffle":true,"random_state":0}},"scoring":"accuracy","folds":5,"cv_scores":[0.9807692307692307,0.9807692307692307,1.0,1.0,0.9803921568627451],"test_score":0.9883720930232558},"model":{"file":"model/model.joblib","type":"sklearn.pipeline.Pipeline","versions":{"python":"...","scikit-learn":"...","pandas":"...","numpy":"...","joblib":"..."},"code":["features.py"]}}
```

## Errors

Returns a tool error when the file is missing or fails contract schema validation.
