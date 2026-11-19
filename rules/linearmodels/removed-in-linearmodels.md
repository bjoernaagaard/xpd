These modules and methods were removed from linearmodels, so the import or the call raises. Use the replacement that the message names.

```diff lang="python"
-from linearmodels.utility import MissingValueWarning, WaldTestStatistic
+from linearmodels.shared.exceptions import MissingValueWarning
+from linearmodels.shared.hypotheses import WaldTestStatistic
 
-test = result.test_linear_constraint(restriction, value)
+test = result.wald_test(restriction, value)
```

## What the rule reports

The rule reports an import of `linearmodels.utility`, which 4.18 split into the modules of
`linearmodels.shared`. It also reports `test_linear_constraint` on a result of `OLS`, `IV2SLS`,
`IVLIML` or `IVGMM`, which 4.12 renamed `wald_test` and 4.15 removed.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `wald_test` for `test_linear_constraint`, because on 4.14 `test_linear_constraint` is a method of its own that warns and then calls `wald_test`. The import has no fix.

| Form | Ran on | Raised on |
| --- | --- | --- |
| `from linearmodels.utility import ...` | 4.17 | 4.18 and 7.1, with `ModuleNotFoundError: No module named 'linearmodels.utility'` |
| `test_linear_constraint` | 4.14, where it returned a `WaldTestStatistic` | 4.15, with `AttributeError` |

`wald_test` ran on 4.14 and on 4.15. The 22 public names of the old module were looked up in the
7.1 source:

| Names of `linearmodels.utility` | Module on 7.1 |
| --- | --- |
| `MissingValueWarning`, `MemoryWarning`, `InferenceUnavailableWarning`, `SingletonWarning`, `missing_warning` | `linearmodels.shared.exceptions` |
| `AttrDict`, `ensure_unique_column`, `panel_to_frame` | `linearmodels.shared.utility` |
| `WaldTestStatistic`, `InvalidTestStatistic`, `InapplicableTestStatistic`, `InvalidTestWarning`, `quadratic_form_test` | `linearmodels.shared.hypotheses` |
| `has_constant`, `inv_sqrth` | `linearmodels.shared.linalg` |
| `pval_format`, `param_table`, `format_wide` | `linearmodels.shared.io` |
| `get_string`, `get_float`, `get_bool`, `get_array_like` | `linearmodels.shared.typed_getters` |

Code inside a try block with an except clause is a compatibility path and is left alone.
