`ColumnSelector`, `ColumnDropper` and `TypeSelector` have no `set_output` and no `get_feature_names_out`, so calling either on a pipeline that holds them raises. Select columns with a `ColumnTransformer` and `"passthrough"`.

```diff lang="python"
-pipeline = make_pipeline(ColumnSelector(["age", "fare"]), StandardScaler())
+pipeline = make_pipeline(ColumnTransformer([("keep", "passthrough", ["age", "fare"])]), StandardScaler())
 pipeline.set_output(transform="pandas")
```

## What the rule reports

The rule reports `set_output(...)` and `get_feature_names_out()` on a pipeline that holds
`ColumnSelector`, `ColumnDropper` or `TypeSelector`, called on the pipeline itself or on a name the
file assigns it to. The three classes have `get_feature_names` and neither of the two scikit-learn
methods.

## How to fix it

xpd has no automatic fix, so make the change by hand.

For each of the three classes, `set_output` raised ValueError "Unable to configure output for
ColumnSelector(...) because `set_output` is not available.", and `get_feature_names_out` raised
AttributeError "Estimator columnselector does not provide get_feature_names_out". The
`ColumnTransformer` form did both, and it names the columns `keep__age` and `keep__fare`.
