A `scoring` text that is not a scorer name raises. Pick a name from `get_scorer_names()`, and note that losses start with `neg_`.

```diff lang="python"
-scores = cross_val_score(model, X, y, scoring="mean_squared_error")
+scores = cross_val_score(model, X, y, scoring="neg_mean_squared_error")
```

## What the rule reports

The rule reports `scoring="mean_squared_error"` and any other string that `get_scorer` refuses. It
reads `scoring` in a search, `cross_val_score`, `cross_validate` and the other scikit-learn
functions that take it, and the argument of `get_scorer` itself.

## How to fix it

`xpd check --fix --unsafe-fixes` adds the missing `neg_` prefix to known scikit-learn loss names (such as `"mean_squared_error"` -> `"neg_mean_squared_error"`) and rewrites metric function names (such as `"roc_auc_score"` -> `"roc_auc"` and `"brier_score_loss"` -> `"neg_brier_score"`). Other unknown strings remain manual.

The rule checks each string against `sklearn.metrics.get_scorer_names()` on scikit-learn 1.9.1.
`scoring="mean_squared_error"`, `scoring="log_loss"` and `scoring="auc"` raise
InvalidParameterError, and `"roc_auc_score"` in a list given to `cross_validate` raises
ValueError. A string with `{` or `%` in it, or an f-string, is left alone, because code fills it in
later. Recall was 16 of 32. The rule found none of 16 scoring strings or dicts of scorer names kept
in a variable.
