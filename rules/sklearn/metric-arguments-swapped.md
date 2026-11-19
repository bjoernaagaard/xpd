Metrics take the true values first and the predictions second, so swapping them gives another number, such as the recall in place of the precision. Pass `y_true` first.

```diff lang="python"
-precision = precision_score(y_pred, y_test)
+precision = precision_score(y_test, y_pred)
```

## What the rule reports

The rule reports a metric whose predictions come first, such as `precision_score(y_pred, y_test)`
and `roc_auc_score(y_score, y_test)`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The severity is error for `precision_score`, `classification_report` and `precision_recall_fscore_support`, and for scores passed first to a ranking metric or a loss, and info for the other metrics that change with the order. The unsafe fix writes `precision_score(y_test, y_pred)`, and the labels first for scores passed first, as in `roc_auc_score(y_test, scores)`. A name can mislead.

The rule reports when the first name holds `pred` or `hat`, and the second holds `true`, `test`,
`val`, `actual`, `target` or `label`, or is `y`. With the arguments swapped, the precision a metric
reports is the recall. On noise where 30 percent of the rows are positive,
`LogisticRegression(class_weight="balanced")` gave the following numbers:

| Metric | Swapped | Right order | Severity |
| --- | --- | --- | --- |
| precision, and `classification_report` for class 1 | 0.471 | 0.294 | error |
| recall | 0.294 | 0.471 | info |
| balanced accuracy | 0.496 | 0.495 | info |
| R2 | -186.06 | -0.022 | info |
| R2, on data with signal | -0.069 | 0.486 | info |

The metrics other than precision are info, because their swapped numbers on noise are not above
the right value. Scores passed first to `roc_auc_score`, `average_precision_score`, `roc_curve`,
`precision_recall_curve`, `log_loss` or `brier_score_loss` raise ValueError, because the true
values then look continuous. `accuracy_score`, `f1_score`, `jaccard_score`, `matthews_corrcoef`,
`cohen_kappa_score` and the error metrics such as `mean_squared_error` return the same number in
either order, so they are left alone.

The fix changes the number on purpose. Precision becomes the old recall, and a confusion matrix is
transposed. Of 43 held-back reports, 30 were labelled, and neither reader called one wrong.
Recall was 66 of 100. The rule found none of 9 swapped keyword values, such as
`confusion_matrix(y_true=y_pred, y_pred=y_true)`, and 2 of 7 `log_loss` calls with the scores
first. It also misses a column slice first, such as `roc_auc_score(y_prob[:, 1], y_test)`, and
true values named with `train` or `Y_`.
