Use `refit` to check that a saved scikit-learn Pipeline can be trained again from
its raw data, as [`xpd refit`](../../../commands/refit/) does. It fits a clone on the training rows that the
contract records and compares outputs and scores. joblib runs code when it loads a model, so
load only a model you trust.

## Example

With the server's root at `samples/pipeline`, after `train.py` wrote its model:

```json
{"file":"model/contract.json"}
```

```json
{"rows":344,"train_rows":258,"test_rows":86,"output":"predict_proba","largest_gap":0.0,"model_matches":true,"contract_test_score":0.9883720930232558,"refit_test_score":0.9883720930232558,"contract_cv_scores":[0.9807692307692307,0.9807692307692307,1.0,1.0,0.9803921568627451],"refit_cv_scores":[0.9807692307692307,0.9807692307692307,1.0,1.0,0.9803921568627451],"test_score_matches":true,"cv_scores_match":true,"counts_match":true,"passed":true}
```

## Errors

A contract that cannot be read, a missing pandas or scikit-learn, or a model that fails to load
makes the call fail with the error that `refit` reports. A comparison that differs is a result,
with `passed: false`.
