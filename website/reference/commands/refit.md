Use `xpd refit` to check that a saved scikit-learn Pipeline can be trained again from its raw
data. It fits a clone of the model on the training rows that its contract records, and compares
the outputs and scores with the ones that the training script saved. Run it in the project root,
with pandas and scikit-learn in the project's Python.

The contract is a JSON file that the training script saves beside the model, with the data file,
the split, the scores and the model file. It must record `validation.splitter`, with `class`
naming a splitter in `sklearn.model_selection` and `args` holding its keyword arguments. Every
path in the contract must be inside the current folder.

joblib runs code when it loads a model, so load only a model you trust.

## Examples

`samples/pipeline/train.py` writes a model and a contract that you can try `refit` on.

```console
$ cd samples/pipeline && xpd refit model/contract.json
{"rows": 344, "train_rows": 258, "test_rows": 86, "output": "predict_proba", "largest_gap": 0.0, "model_matches": true, "contract_test_score": 0.9883720930232558, "refit_test_score": 0.9883720930232558, "contract_cv_scores": [0.9807692307692307, 0.9807692307692307, 1.0, 1.0, 0.9803921568627451], "refit_cv_scores": [0.9807692307692307, 0.9807692307692307, 1.0, 1.0, 0.9803921568627451], "test_score_matches": true, "cv_scores_match": true, "counts_match": true, "passed": true}
```

`refit` takes these steps in order.

1. It reads the data file and rebuilds the split that the contract records.
2. It loads the model with joblib and fits a clone on the training rows.
3. It compares the outputs of the model and the clone on the held out rows. It uses
   `predict_proba` when the model has it, then `decision_function`, then `predict`.
4. It scores the held out rows with `get_scorer`, and it computes fold scores with
   `cross_val_score` on the training rows only.

DataFrames and Series are compared with the pandas default tolerances, as in
[`xpd equiv`](../equiv/). Other outputs and the scores must be exactly equal, so save the
contract scores without rounding. `refit` changes no saved file.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | Every comparison matches |
| 1 | A comparison differs |
| 2 | `refit` could not run, and the JSON has an `error` |
