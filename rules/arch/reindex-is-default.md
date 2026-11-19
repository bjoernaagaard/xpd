`reindex=False` has been the default of `forecast` since arch 6.2, so the argument does nothing. Drop it.

```diff lang="python"
 res = arch_model(returns).fit(disp="off")
-forecasts = res.forecast(horizon=5, reindex=False)
+forecasts = res.forecast(horizon=5)
```

## What the rule reports

The rule reports `reindex=False` given to `forecast` of an arch model or result.

## How to fix it

`xpd check --fix` applies the fix of this rule.

With the safe fix, the argument is taken out. When it is the first argument or follows a comment, the fix writes the arguments again without it and drops the comments between them.

Before arch 6.2, `forecast` returned a frame as long as the data and warned that the default
would change. Code from before 6.2 passes `reindex=False` to get the short frame without the
warning, and the document carries `since: "6.2"`.

| arch | `forecast()` on 1,500 rows | `forecast(reindex=False)` |
| --- | --- | --- |
| 6.1.0 | 1,500 rows and a FutureWarning | the short frame, without the warning |
| 6.2.0, 8.0.0, 9.0.0 | 1 row and no warning | the same frame as `forecast()`, for the last row and with `start` |

The fix is safe, because the two frames were equal on 6.2.0, 8.0.0 and 9.0.0. `reindex=True`
still returns the long frame and is left alone.

The import `from arch.__future__ import reindexing` comes from the same habit and is reported by
[`removed-in-arch`](removed-in-arch.md). The rule made 61 reports on the public code, and 20 on
arch's own tests, notebooks and documentation.
