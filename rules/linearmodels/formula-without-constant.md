linearmodels adds no intercept to a formula, where statsmodels and R add one, so the estimates change. Write `1 +` at the start of the right side.

```diff lang="python"
-model = PooledOLS.from_formula("lwage ~ exper", data)
-iv = IV2SLS.from_formula("lwage ~ exper + [educ ~ fatheduc + motheduc]", data)
+model = PooledOLS.from_formula("lwage ~ 1 + exper", data)
+iv = IV2SLS.from_formula("lwage ~ 1 + exper + [educ ~ fatheduc + motheduc]", data)
```

## What the rule reports

The rule reports a formula with no `1` term in `from_formula` of `PooledOLS`, `RandomEffects`,
`BetweenOLS`, `FamaMacBeth`, `PanelOLS` and the system models. It also reports such a formula in
`from_formula` of `IV2SLS`, `IVLIML`, `IVGMM`, `IVGMMCUE`, `IVJIVE` and `OLS` when the formula has
an instrument block in brackets. linearmodels adds no intercept to such formulas, where statsmodels
and R add one. `"y ~ x"` is therefore a regression through the origin in linearmodels and a
regression with an intercept in statsmodels.

## How to fix it

xpd has no automatic fix, so make the change by hand.

On 7.1 and on 7.0, the models gave the following coefficient of `exper` on the wage panel, and of
`educ` for `IV2SLS` on the mroz data:

| Model | Without `1` | With `1` |
| --- | --- | --- |
| `PooledOLS` | 0.2184 | 0.0338, with an intercept of 1.4289 |
| `RandomEffects` | 0.1119 | 0.0589 |
| `BetweenOLS` | 0.2364 | -0.0229 |
| `IV2SLS` with an instrument block | 0.0769 | 0.0664 |

The instrument models for one equation add the intercept themselves when the formula has no block
in brackets. On 7.1, 7.0 and 5.0, `IV2SLS`, `IVLIML`, `IVGMM` and `IVGMMCUE` returned `Intercept`
and `exper` for `"lwage ~ exper"`, so the rule leaves the form alone on the four classes.

The rule stays silent when:

- the formula says `- 1` or `0 +`, which states that no intercept is meant
- the formula names a column that looks like a constant, such as `const`
- the formula has a `C()` term, whose dummies span the intercept
- a `PanelOLS` formula has effects, or the call has `other_effects`, which absorb the intercept

A model without an intercept can be meant, so the rule is a warning.
