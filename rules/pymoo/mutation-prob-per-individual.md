Since pymoo 0.6.0, `prob` is the share of individuals that mutate, and `prob_var` is the chance for each variable, so an old `prob=1/n` mutates far less. Write `prob_var=` when you mean the chance per variable.

```diff lang="python"
-mutation = PM(prob=1 / problem.n_var, eta=20)
+mutation = PM(prob_var=1 / problem.n_var, eta=20)
```

## What the rule reports

The rule reports `prob=` on `PM`, `PolynomialMutation`, `BitflipMutation`, `BFM`, `GaussianMutation` and `GM` when the value is 1 divided by something, or a number of at most 0.2, and the call has no `prob_var`. Up to pymoo 0.5.0 `prob` was the chance that one variable changes. Since 0.6.0 it is the share of individuals that mutate, and `prob_var` is the chance per variable.

| 20,000 individuals of 30 variables | pymoo 0.5.0 | pymoo 0.6.2 |
| --- | --- | --- |
| individuals changed by `prob=1/30` | 63 percent | 2 percent |
| individuals changed by `prob_var=1/30` | not run | 57 percent |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `prob_var=`.
