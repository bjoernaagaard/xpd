Use `xpd api` to read how to call a library symbol without importing the library. It prints one
symbol, method, rule or recipe in full from xpd's bundled indexes or project libraries registered
in `[tool.xpd.libraries]` via [`xpd library`](../library/), with each parameter as the source
writes it, the first paragraph of the docstring, the public methods and the rules that may apply
to it. The name is one that [`xpd find`](../find/) returns, or a method or property of a class.

## Examples

```console
$ xpd api sklego.meta.GroupedPredictor
sklego.meta.GroupedPredictor(estimator, groups, shrinkage=None, use_global_model=True, check_X=True, shrinkage_kwargs=None) [class]
`GroupedPredictor` is a meta-estimator that fits a separate estimator for each group in the input data.
  decision_function(X) Predict confidence scores for samples in `X`.
  fit(X, y) Fit one estimator for each group of training data `X` and `y`.
  predict(X) Predict target values on new data `X` by predicting on each group. If a group is not found during `.predict()` and `use_global_model=True` the fallback estimator will be used. If `use_global_model=False` a `ValueError` will be raised.
  predict_proba(X) Predict probabilities on new data `X`.
Rules: sklego/grouped-shrinkage-arguments
```

Add a member's name to its class to read one method or property. A property has no parameter
list.

```console
$ xpd api arch.unitroot.KPSS.pvalue
arch.unitroot.KPSS.pvalue [property]
The p-value for the test statistic.
```

`--format json` prints a list with the one entry. `parameters` holds the names, and
`parameters_written` holds each parameter as the source writes it, with its type annotation and
default. A stub file writes `...` for a default, which says only that the parameter is optional.

```console
$ xpd api sklearn.model_selection.TimeSeriesSplit.split --format json
[{"kind":"symbol","package":"sklearn","name":"sklearn.model_selection.TimeSeriesSplit.split","callable":"method","parameters":["X","y","groups"],"parameters_written":["X","y=None","groups=None"],"summary":"Generate indices to split data into training and test set.","members":[],"rules":[]}]
```

The output says how to call the symbol, and not what each parameter means, so read the
library's documentation for that. A rule link means the last part of the symbol's name appears
in the rule's patterns, so the rule may apply.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The entry was printed |
| 2 | No entry has that name, or xpd could not run |

```console
$ xpd api no.such.name
xpd: no entry named no.such.name
```
