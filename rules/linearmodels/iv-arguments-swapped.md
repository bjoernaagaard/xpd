The linearmodels instrument models take `exog` before `endog`, and these arguments are named the other way round, so the model instruments the wrong variables. Swap them.

```diff lang="python"
-model = IV2SLS(dependent, endog, exog, instruments)
+model = IV2SLS(dependent, exog, endog, instruments)
```

## What the rule reports

The rule reports an instrument model whose second argument has `endog` in its name and whose third
has `exog`. linearmodels takes exog and then endog, and it doesn't check the roles.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

With the unsafe fix, the second and third arguments change places. The fix trusts the names, and it changes the estimates on purpose.

On the mroz data, `IV2SLS(lwage, educ, [const, exper], [fatheduc, motheduc])` ran without an error and gave `educ` 0.2021, where the right order gave 0.0664. The rule knows the roles only from the names, and reports no other case.
