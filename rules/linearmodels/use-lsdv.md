`use_lsdv=True` builds a dummy column for each entity, which was up to 1,508 times slower and gave the same coefficients. Drop the option.

```diff lang="python"
-result = PanelOLS(y, X, entity_effects=True).fit(use_lsdv=True)
+result = PanelOLS(y, X, entity_effects=True).fit()
```

## What the rule reports

The rule reports `use_lsdv=True` in `fit` of a `PanelOLS`. The option estimates each effect as a
dummy column, where the default removes the effects by subtracting group means and gives the same
estimates.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `use_lsdv=True` is left out, and the estimates are equal but not bit equal.

On one thread, as the median of 5 fits with entity effects, 3 regressors and 10 periods, the two
forms took the following times:

| Entities | LSDV | Default | LSDV was slower on 7.1 by | On 7.0 by |
| --- | --- | --- | --- | --- |
| 500 | 0.298 s | 0.017 s | 17 times | 18 times |
| 3,000 | 77.1 s | 0.051 s | 1,508 times | 1,382 times |

The coefficients differed by at most 2.2e-16 and the standard errors by at most 1.7e-18, so the
estimates are equal but not bit equal, and the fix is unsafe. `use_lsmr=True` and
`low_memory=True` ran within 10% of the default and are not reported.
