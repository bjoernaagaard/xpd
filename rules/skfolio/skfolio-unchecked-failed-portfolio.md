With `raise_on_failure=False`, skfolio can produce a `FailedPortfolio` with NaN values and a stored error.

```diff
-book = MeanRisk(raise_on_failure=False).fit(returns).predict(returns)
-weights = book.weights
+book = MeanRisk(raise_on_failure=False).fit(returns).predict(returns)
+if isinstance(book, FailedPortfolio):
+    raise RuntimeError(book.optimization_error)
+weights = book.weights
```

## What the rule reports

The rule follows a direct prediction using an explicitly non-raising constructor. During sequential evaluation, inspect `failed_portfolios` on the result to find failed periods.
