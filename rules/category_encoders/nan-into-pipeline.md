With `"return_nan"`, the encoder hands NaN to the next step of the pipeline, and many models raise on NaN at predict. Use `"value"`, or put an imputer after the encoder.

```diff lang="python"
-model = make_pipeline(ce.OneHotEncoder(cols=["city"], handle_unknown="return_nan"), LogisticRegression())
+model = make_pipeline(ce.OneHotEncoder(cols=["city"], handle_unknown="value"), LogisticRegression())
```

## What the rule reports

The rule reports an encoder with `handle_unknown="return_nan"` or `handle_missing="return_nan"`
inside a `Pipeline` or `make_pipeline` call, and a `CountEncoder(handle_unknown=None)` there.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `handle_unknown="value"` for `CountEncoder(handle_unknown=None)`, which codes an unseen category as 0 in place of NaN. `"return_nan"` has no fix, because the message names two rewrites, `"value"` or an imputer after the encoder.

xpd gives the replacement for `"return_nan"` as advice and doesn't apply it.

`return_nan` codes a category that fit did not see as NaN, in every output column of the row, and
in a Pipeline the NaN goes to the next step. With a `OneHotEncoder` or a `TargetEncoder` in front
of a `LogisticRegression`, `predict` on a row with an unseen city raised
`ValueError: Input X contains NaN`. An `OrdinalEncoder` with `handle_missing="return_nan"` raised
the same on a row with a missing city. The fit passed, because the training rows had no unseen
category. The error first shows on new data. With `"value"`, the default, the same `predict`
ran.

| Encoder | Code of an unseen category with `"value"` | With `"return_nan"` |
| --- | --- | --- |
| `OrdinalEncoder` | -1.0 | NaN |
| `OneHotEncoder` | 0 in every column | NaN in every column |
| `BinaryEncoder` | 0 in every column | NaN in every column |
| `TargetEncoder` | the mean of the target, 0.5 here | NaN |
| `CountEncoder` | 0.0 | NaN |

The rule stays silent when the same pipeline call holds an imputer or a model that takes NaN,
such as `HistGradientBoostingClassifier`. Outside a pipeline call the option is left alone,
because the code may handle the NaN itself.
