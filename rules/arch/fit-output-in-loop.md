`fit` prints every iteration and a summary on each call, so a loop of fits floods the output. Pass `disp="off"`.

```diff lang="python"
 for end in range(1000, len(returns)):
-    res = arch_model(returns.iloc[:end]).fit()
+    res = arch_model(returns.iloc[:end]).fit(disp="off")
```

## What the rule reports

The rule reports `fit` without `disp` on an arch model inside a loop or a comprehension.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix adds `disp="off"` at the end of the arguments and drops the comments between them. The fit returns the same result and prints nothing, so a script or a log that reads the printed lines gets none.

`fit` has `disp="final"` and `update_freq=1`, so it prints a line for every iteration and a
summary at the end. The rule is info, because the output costs no time.

| Twenty fits of a GARCH(1,1) in a loop | Lines printed | Time |
| --- | --- | --- |
| `fit()` | 240 | 0.102 seconds |
| `fit(disp="off")` | 0 | 0.101 seconds |

`update_freq=0` alone still prints the summary. All 15 reports on the public code were fits in a
loop over windows, assets or model orders.
