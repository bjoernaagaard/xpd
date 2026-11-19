scikit-lego warns on these classes and asks you to use a replacement, such as scikit-learn's `QuantileRegressor`. Use the replacement that the message names.

```diff lang="python"
-model = QuantileRegression(quantile=0.8)
-season = RepeatingBasisFunction(n_periods=12, column="day_of_year")
-keep = IdentityTransformer()
+model = QuantileRegressor(quantile=0.8, alpha=0)
+season = ColumnTransformer([("season", SplineTransformer(n_knots=13, extrapolation="periodic"), ["day_of_year"])])
+keep = FunctionTransformer()
```

## What the rule reports

The rule reports the classes that scikit-lego tells users to replace since 0.9.8, where building
one raises a UserWarning that names the replacement. It also reports `IdentityTransformer()`
without arguments.

| Class | Replacement |
| --- | --- |
| `QuantileRegression`, `LADRegression` | scikit-learn's `QuantileRegressor` |
| `RepeatingBasisFunction` | scikit-learn's `SplineTransformer(extrapolation="periodic")` |
| `DemographicParityClassifier`, `EqualOpportunityClassifier` | fairlearn's `ThresholdOptimizer` around a `LogisticRegression` |
| `IdentityTransformer()` | `FunctionTransformer()` |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The severity is warning for the classes that warn, and info for `IdentityTransformer()`. The unsafe fix writes `FunctionTransformer()` for `IdentityTransformer()`, in a file that imports `FunctionTransformer` from `sklearn.preprocessing`. The step is then another class, whose parameters and repr differ. The other classes have no fix.

xpd gives the other replacements as advice and doesn't apply them, because each one is another
model with other parameter names. On 500 rows `QuantileRegression(quantile=0.8)` and
`QuantileRegressor(quantile=0.8, alpha=0)` gave the same coefficients to three decimals. The spline
is another basis than the radial one, so its columns are other numbers. `IdentityTransformer()` and
`FunctionTransformer()` both returned their input itself.
