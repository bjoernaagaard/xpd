Each linearmodels estimator accepts its own covariance names. For example, a `PanelOLS` fit accepts `cov_type="robust"` and raises an error for the statsmodels name `"HC1"`.

```diff lang="python"
-result = PanelOLS(y, X, entity_effects=True).fit(cov_type="HC1")
-iv = IV2SLS(y, exog, endog, instruments).fit(cov_type="nonrobust")
+result = PanelOLS(y, X, entity_effects=True).fit(cov_type="robust")
+iv = IV2SLS(y, exog, endog, instruments).fit(cov_type="unadjusted", debiased=True)
```

## What the rule reports

The rule reports a `cov_type` string that the model doesn't take, such as `"HC1"`, `"HAC"`,
`"cluster"` or `"nonrobust"` of statsmodels, `"clustered"` on `FamaMacBeth` or `IVSystemGMM`,
`"kernel"` on `BetweenOLS`, and `"unadjusted"` on a factor model.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

On OLS, IV2SLS, IVLIML and AbsorbingLS, the unsafe fix writes `"robust"` for `"HC0"`, `"robust", debiased=True` for `"HC1"` and `"unadjusted", debiased=True` for `"nonrobust"`, in a call that doesn't pass `debiased`. The call raised before the fix. Any other name has no fix, because the code doesn't show which name of the model is meant.

7.1 raises `UnknownOptionError`, which is both a `ValueError` and a `KeyError`, with the list of
names the model takes, and 7.0 raised `ValueError` or `KeyError`. The lists were read from the
error for each of the 19 model classes, and each name on a list was then run on its class:

| Models | Names |
| --- | --- |
| `PanelOLS`, `PooledOLS`, `RandomEffects`, `FirstDifferenceOLS` | `unadjusted`, `homoskedastic`, `conventional`, `robust`, `heteroskedastic`, `clustered`, `kernel`, `dk`, `driscoll-kraay`, `ac`, `autocorrelated` |
| `BetweenOLS` | `unadjusted`, `homoskedastic`, `conventional`, `robust`, `heteroskedastic`, `clustered` |
| `FamaMacBeth` | `unadjusted`, `homoskedastic`, `robust`, `heteroskedastic`, `kernel` |
| `OLS`, `IV2SLS`, `IVLIML`, `AbsorbingLS` | `unadjusted`, `homoskedastic`, `homo`, `robust`, `heteroskedastic`, `hccm`, `kernel`, `clustered`, `one-way`, and the four class names such as `KernelCovariance` |
| `IVGMM`, `IVGMMCUE`, `IVJIVE` | `unadjusted`, `homoskedastic`, `robust`, `heteroskedastic`, `kernel`, `clustered` |
| `SUR`, `IV3SLS` | `unadjusted`, `homoskedastic`, `robust`, `heteroskedastic`, `kernel`, `hac`, `clustered` |
| `IVSystemGMM` | `unadjusted`, `homoskedastic`, `robust`, `heteroskedastic`, `kernel`, `hac` |
| `TradedFactorModel`, `LinearFactorModel`, `LinearFactorModelGMM` | `robust`, `heteroskedastic`, `kernel` |

Since 7.1 the names are not case sensitive, so the rule takes a known name in any case.

On `OLS` with the mroz data, the standard errors of linearmodels were equal to the ones of statsmodels
to 1e-13 for each pair below, and the message for the instrument models gives the same pairs:

| linearmodels | statsmodels |
| --- | --- |
| `"robust"` | `"HC0"` |
| `"robust"` with `debiased=True` | `"HC1"` |
| `"unadjusted"` with `debiased=True` | `"nonrobust"` |
| `"clustered"` with `clusters=` and `debiased=True` | `"cluster"` with the same groups |

The panel models default to `debiased=True`, and their default standard errors were equal to the ones
of statsmodels.
