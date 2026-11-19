Step names with two underscores in a row, and Pipelines built from the wrong kind of argument, raise ValueError at fit. Use one underscore, and pass the steps the way the message shows.

```diff lang="python"
-from sklearn.pipeline import Pipeline
+from imblearn.pipeline import Pipeline
 
 pipe = Pipeline([("smote", SMOTE()), ("model", LogisticRegression())])
```

## What the rule reports

The rule reports step names with `__`, `make_pipeline` given one list, `Pipeline` given bare
estimators, and an imbalanced-learn sampler before another step in the Pipeline of scikit-learn.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `make_pipeline(a, b)` for `make_pipeline([a, b])`, because the old form raises at fit. The other forms have no fix.

xpd applies the replacement with `--fix --unsafe-fixes`.

| Form | Error |
| --- | --- |
| a step name with two underscores in a row, in the list given to `Pipeline`, `ColumnTransformer` or `FeatureUnion` | ValueError at fit |
| `make_pipeline([a, b])` | TypeError at fit |
| `Pipeline([a, b])` | "TypeError: cannot unpack non-iterable" |
| a sampler such as `SMOTE` before another step | "TypeError: All intermediate steps should be transformers" |

The first three forms are reported only when the name comes from scikit-learn, imported from
`sklearn.pipeline` or `sklearn.compose` or called through the two modules. Other libraries, such as
xfeat, have a Pipeline that takes a plain list. The rule as the readers saw it made 4 reports on
the held-back files, on such Pipelines, and both readers called all 4 wrong. Files that import
from `imblearn.pipeline` are left alone, and so is a sampler as the last step, which fits.

Recall was 29 of 35, measured before the rule was narrowed to the Pipeline of scikit-learn. The
rule found 4 of 10 plants of `Pipeline` given bare estimators. It misses them given by `steps=`,
by names that hold the estimators, or as a single step.
