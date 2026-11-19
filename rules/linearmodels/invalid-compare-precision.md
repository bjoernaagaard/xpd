`compare` takes only `"tstats"`, `"std_errors"` or `"pvalues"` for `precision`, so another value raises ValueError. Use the name that `compare` takes for the same number.

```diff lang="python"
-table = compare({"pooled": pooled, "fixed": fixed}, precision="se")
+table = compare({"pooled": pooled, "fixed": fixed}, precision="std_errors")
```

## What the rule reports

The rule reports `compare(..., precision=...)` of `linearmodels.panel` or `linearmodels.iv` with a
string other than `"tstats"`, `"std_errors"`, `"std-errors"` and `"pvalues"`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `"std_errors"` for `"se"`, `"stderr"`, `"std_err"` and `"bse"`, `"tstats"` for `"t"`, `"tstat"` and `"t-stats"`, and `"pvalues"` for `"p"` and `"pvalue"`. The call raised before the fix. Any other string has no fix.

On 7.1 and 7.0 both functions took the four strings in any case, and raised `ValueError: Unknown precision value. Must be one of 'tstats', 'std_errors' or 'pvalues'.` for `"se"`, `"stderr"`, `"t"`, `"p"`, `"tstat"`, `"std_err"`, `"pvalue"`, `"t-stats"` and `"bse"`. The four strings were also read from the source. The rule reads `compare` only when the file imports it from linearmodels or calls it through the module.
