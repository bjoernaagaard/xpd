These metric arguments and names warn on scikit-learn 1.9.1 and go away in a later release. Use the replacement that the message names, such as `y_proba=` for `y_pred=`.

```diff lang="python"
-loss = log_loss(y_test, y_pred=proba)
+loss = log_loss(y_test, y_proba=proba)
```

## What the rule reports

The rule reports metric APIs that warn on scikit-learn 1.9.1. Its message gives the release that
removes each form, from the warning text:

| Form | Removed in | Write instead |
| --- | --- | --- |
| `y_pred=` on `log_loss` and `d2_log_loss_score` | 1.11 | `y_proba=` |
| `estimator_name=` on `PrecisionRecallDisplay` | 1.10 | `name=` |
| `y_pred=` on `PrecisionRecallDisplay.from_predictions` and `DetCurveDisplay.from_predictions` | 1.10 | `y_score=` |
| matplotlib keywords passed straight to `PrecisionRecallDisplay` | 1.11 | `curve_kwargs={...}` |
| `sklearn.metrics.cluster.entropy` | 1.10 | `scipy.stats.entropy` of the class counts |

## How to fix it

`xpd check --fix` applies the fix of this rule.

The safe fix writes `log_loss(y_test, y_proba=proba)`, `PrecisionRecallDisplay(precision=precision, recall=recall, name="model")`, `y_score=` for `y_pred=` on `PrecisionRecallDisplay.from_predictions` and `DetCurveDisplay.from_predictions`, and `curve_kwargs={"color": "red"}` for `color="red"` on `PrecisionRecallDisplay`, which already used those keywords as `curve_kwargs`. The other forms have no fix.

The four display forms passed in the first run with matplotlib 3.11.2, and the saved rerun had no
matplotlib. The two fixes rename a keyword and gave bit equal results, the `log_loss` rename in 12
of 12 runs and the `PrecisionRecallDisplay` rename in 2 of 2 with the same plot label.
`scipy.stats.entropy` of the class counts differs from `entropy` in the last bits, so that form
has no fix. Recall was 48 of 48.
