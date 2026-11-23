In a statsmodels OLS formula, Patsy and Formulaic use `**` to expand interactions.
For a single predictor, `x**2` gives the same term as `x`. In both engines,
`y ~ x + x**2` produced two columns and `y ~ x + I(x**2)` produced three.

```diff
-smf.ols("y ~ x + x**2", data)
+smf.ols("y ~ x + I(x**2)", data)
```

## What the rule reports

The rule checks additive formula literals in `ols` and `OLS.from_formula`,
including raw strings and formulas with simple parenthesized terms. It
reports a standalone predictor name raised to a literal integer of at least
two. The edit preserves existing transforms such as `I(z**2)` and
interaction expansions such as `(a + b)**2`.

## How to fix it

Wrap a numeric power in `I(...)`. Use `xpd check --fix --unsafe-fixes`
to apply the edit. It changes the design matrix.

[Patsy formula operators](https://patsy.readthedocs.io/en/latest/formulas.html#operators) ·
[Formulaic grammar](https://matthew.wardrop.casa/formulaic/latest/guides/grammar/#operators)
