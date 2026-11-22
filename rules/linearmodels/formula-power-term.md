A linearmodels formula reads `**` as a power of interactions, so `exper**2` is just `exper`, with no error. Write `I(exper**2)` for the squared values.

```diff lang="python"
-model = IV2SLS.from_formula("lwage ~ 1 + exper + exper**2", data)
+model = IV2SLS.from_formula("lwage ~ 1 + exper + I(exper**2)", data)
```

## What the rule reports

The rule reports a term that is a name, `**` and a number, such as `exper**2`, in a formula of any
model. A formula reads `**` as a power of interactions, so `exper**2` is `exper`. The squared values
need `I(exper**2)`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `I(exper**2)` for `exper**2`, which changes the model on purpose.

On 7.1 and 7.0 the first formula gave a model with the columns `Intercept` and `exper` on `IV2SLS` and on `PooledOLS`, with no error, and the second gave three columns. The README of linearmodels itself has `exper + exper ** 2` in its first example, which fits a model with the columns `Intercept`, `exper` and `educ`. `(a + b)**2`, which is the use the operator is for, is left alone.
