An encoder that names its `cols` follows a step that returns a NumPy array, so the names are gone and fit raises ValueError. Call `set_output(transform="pandas")` on the step before, or put the encoder first.

```diff lang="python"
-model = make_pipeline(SimpleImputer(strategy="most_frequent"), ce.OneHotEncoder(cols=["city"]))
+imputer = SimpleImputer(strategy="most_frequent").set_output(transform="pandas")
+model = make_pipeline(imputer, ce.OneHotEncoder(cols=["city"]))
```

## What the rule reports

The rule reports an encoder with names in `cols` that comes after an imputer, a scaler or a
`ColumnTransformer` in a pipeline. Such steps hand the next step a NumPy array, and the names are
gone.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The old form raised `ValueError: X does not contain the columns listed in cols: ['city'] The
encoder received a non-DataFrame input at fit, where named columns cannot be recovered
positionally` at fit. With `set_output(transform="pandas")` on the imputer the same
pipeline ran, and so did the old form with the column given by position, as in `cols=[0]`.

The rule knows that the following steps give an array: `SimpleImputer`, `KNNImputer`, `IterativeImputer`,
`StandardScaler`, `MinMaxScaler`, `MaxAbsScaler`, `RobustScaler`, `Normalizer`,
`PowerTransformer`, `QuantileTransformer`, `KBinsDiscretizer`, `PolynomialFeatures`,
`ColumnTransformer` and `make_column_transformer`. It reads `make_pipeline(...)` and
`Pipeline([...])` calls whose steps are written in the call. It stays silent in a file that
calls `set_output` or sets `transform_output`, anywhere.
