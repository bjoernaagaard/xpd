These imports, keywords and names warn on linearmodels 7.1, and 8.0 removes them. Use the replacement that the message names.

```diff lang="python"
-from linearmodels.__future__ import ordering
-
-result = model.fit(method="lsmr", lsmr_options={"atol": 1e-10})
+result = model.fit(method="lsmr", absorb_options={"atol": 1e-10})
```

## What the rule reports

The rule reports `from linearmodels.__future__ import ordering`, the `lsmr_options` keyword of
`AbsorbingLS.fit`, and `linearmodels.DROP_MISSING`. 7.1 gives `FutureWarning` for each, and 8.0
removes them.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix deletes the import of `linearmodels.__future__.ordering` when it is a statement of the module, and writes `absorb_options=` with `method="lsmr"` for `lsmr_options=` in a call that names neither `method` nor `absorb_options`, when the value is a dictionary written with at least one key. The two calls gave equal results on 7.1. The unsafe fix writes the same fix for any other value. With `lsmr_options=None` or an empty dictionary the old call kept the default method, which may pick HDFE. The other forms have no fix.

The import turned off the sorting of formula variables in 4.31, and 5.0 made the unsorted order the
default. With `"y ~ 1 + z + x + w"`, the formula gave the following order of the variables:

| Release | Order without the import | With the import |
| --- | --- | --- |
| 4.31 | `Intercept, w, x, z` | `Intercept, z, x, w` |
| 5.0, 7.0 and 7.1 | `Intercept, z, x, w` | `Intercept, z, x, w` |

The fix deletes a line that does nothing from 5.0 on, and the cases of
`samples/linearmodels/migration.py` returned equal values before and after it. Because of `since`,
xpd skips the document on a project that uses 4.31. Inside a function or a block the import has no fix, because
it may be the only statement there.

| Form | Result |
| --- | --- |
| `absorb_options` | ran on 4.18, and raised `TypeError` on 4.17 |
| `lsmr_options` | gave `FutureWarning` on 4.31, 5.0, 7.0 and 7.1 |
| reading `DROP_MISSING` | ran clean on 7.0, and gave `FutureWarning: linearmodels.DROP_MISSING has no effect` on 7.1 |

The new form needs `method="lsmr"` next to the options, because the default method may pick an
algorithm that doesn't take them. On 7.1 `fit` turns the default method into `"lsmr"` when
`lsmr_options` is a dictionary that is not empty, and the fixed call gave bit equal estimates.
