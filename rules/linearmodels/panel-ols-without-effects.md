`PanelOLS` without effects fits pooled OLS, despite its name. Pass `entity_effects=True` or `time_effects=True` for fixed effects.

```diff lang="python"
-fixed = PanelOLS(y, X).fit()
+fixed = PanelOLS(y, X, entity_effects=True).fit()
+pooled = PooledOLS(y, X).fit()
```

## What the rule reports

The rule reports `PanelOLS(...)` with none of `entity_effects`, `time_effects` and
`other_effects`, and `PanelOLS.from_formula` with a formula that has no effects term. The name of
the class reads like a fixed effects estimator, but without one of the three it fits pooled OLS.

## How to fix it

xpd has no automatic fix, so make the change by hand.

On the wage panel, on 7.1 and 7.0, the three models gave the following coefficients:

| Model | Coefficient of `exper` |
| --- | --- |
| `PanelOLS(y, X)` | 0.0349 |
| `PooledOLS(y, X)` | 0.0349 |
| `PanelOLS(y, X, entity_effects=True)` | 0.0698 |

The rule reports 37 calls in 25 of the 950 public files. Among them are results named `fe_model` and
`mod_fe`, and a function whose docstring starts "One- and two-way fixed effects estimator for panel
data" and whose body is `PanelOLS(y, exogenous).fit()`. A call that passes any of the three
keywords is left alone, also with the value `False`, and so is a call that unpacks its keywords. A
formula with a `C()` term may hold the effects as dummies and is left alone too.
