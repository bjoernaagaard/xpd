A metric that ranks rows by score, such as `roc_auc_score` or `log_loss`, gets class labels from `predict`, which leaves only two values to rank. Pass `predict_proba(X)[:, 1]` or `decision_function(X)`.

```diff lang="python"
-loss = log_loss(y_test, model.predict(X_test))
-auc = roc_auc_score(y_test, model.predict_proba(X_test))
+loss = log_loss(y_test, model.predict_proba(X_test))
+auc = roc_auc_score(y_test, model.predict_proba(X_test)[:, 1])
```

## What the rule reports

The rule reports a metric that needs one probability or score per row and gets class labels or a
whole matrix, such as `log_loss(y_test, model.predict(X_test))`,
`roc_auc_score(y_test, model.predict_proba(X_test))` and
`roc_auc_score(y_test, model.predict(X_test))`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The severity is error for `log_loss` and `brier_score_loss` given labels, for the whole `predict_proba` matrix where one score per row is needed, for `decision_function` given to a loss, and for `roc_auc_score` with `multi_class` given labels, and info for ranking metrics given labels, column 0 of `predict_proba`, and `make_scorer(roc_auc_score)` without `response_method`. The unsafe fix writes `log_loss(y_test, model.predict_proba(X_test))` for labels or `decision_function` given to a loss, `model.predict_proba(X_test)[:, 1]` for labels given to `brier_score_loss` and for the whole matrix given to `roc_curve`, `precision_recall_curve` or `det_curve`, `model.predict_proba(X_test)` for labels given to `roc_auc_score` with `multi_class`, and column 1 for column 0 when no `pos_label` is passed. Each fix changes the score on purpose, and the `multi_class` fix raises on a target with two classes. The other forms have no fix.

The rule was measured with `LogisticRegression` over 30 seeds:

| Form | Data | Old form | Right form |
| --- | --- | --- | --- |
| `log_loss` given the labels | no signal | 17.998 | 0.699, where the honest loss is ln 2, 0.693 |
| `brier_score_loss` given the labels | no signal | 0.499 | 0.253, where the honest loss is 0.25 |
| a ranking metric given the labels | no signal | 0.495 | 0.498 |
| AUC from the labels | signal | 0.735 | 0.822 |
| average precision from the labels | signal | 0.677 | 0.827 |
| AUC from column 0 of `predict_proba` | signal | 0.178 | not given |
| `make_scorer(roc_auc_score)` without `response_method`, in `cross_val_score` | signal | 0.739 | 0.824 |

Ranking metrics given labels are info, because on noise the labels and the probabilities scored
about the same. The following forms raise ValueError:

- the whole `predict_proba` matrix given to `roc_auc_score` without `multi_class`, or to
  `roc_curve`, `precision_recall_curve` or `det_curve`
- `roc_auc_score(multi_class="ovr")` given labels on three classes
- `decision_function` given to `log_loss` or `brier_score_loss`

The rule also reports `y_pred = model.predict(X_test)` when a later line passes `y_pred` to the
metric. The rule leaves alone a `predict` on a Booster that `xgb.train` or `lgb.train` returned,
and a `predict` whose argument is named like predictions or probabilities, as on a calibrator. It
also leaves alone files that import keras, tensorflow or torch, because `predict` returns
probabilities there.

The fix changes the loss on purpose, from 12.0 to 0.63 on the case built for it. On three classes
the old form raised and the fixed form runs. On an `SVC` without probabilities, the fixed form
raises AttributeError.

The readers called 2 and 2 of the 20 held-back reports wrong advice. Recall was 55 of 72. The rule
found none of 13 plants that keep column 0 or the whole `predict_proba` matrix in a variable
before the metric. It also misses a `predict` variable whose argument is wrapped, as in
`model.predict(np.array(X_test.values))`.
