The default kernel bandwidth changed in linearmodels 7.1, so results change between versions and 7.1 warns. Pass `bandwidth=` or `optimal_bw=True`.

```diff lang="python"
-result = IVGMM(y, exog, endog, instruments, weight_type="kernel").fit()
+result = IVGMM(y, exog, endog, instruments, weight_type="kernel", optimal_bw=True).fit()
```

## What the rule reports

The rule reports `weight_type="kernel"` on `IVGMM`, `IVGMMCUE` and `IVSystemGMM`, and
`cov_type="kernel"` in `fit` of `IVGMM` and `IVGMMCUE`, with neither `bandwidth` nor `optimal_bw`.
7.1 changed the default bandwidth from `nobs - 2` to one selected from the data, so the same code
gives other numbers on 7.0 and on 7.1.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix adds `optimal_bw=True`. On 7.1 it gave estimates and standard errors bit equal to the default, without the warning, and on 7.0 it changes the bandwidth from `nobs - 2` to the one selected from the data.

On the mroz data, the two releases gave the following numbers:

| Form | On 7.0 | On 7.1 |
| --- | --- | --- |
| `IVGMM(..., weight_type="kernel")`, J statistic and first standard error | 0.8022 and 0.43964 | 0.3794 and 0.43137, with `DefaultBandwidthWarning` |
| the same with `optimal_bw=True` | 0.3794 and 0.43137 | 0.3794 and 0.43137 |
| the same with `bandwidth=4` | 0.43117 | 0.43117 |
| `IVGMM(...).fit(cov_type="kernel")`, first standard error | 0.35929 | 0.4658 |
| the same with `bandwidth=4` | 0.46795 | 0.46795 |

`IVSystemGMM.fit(cov_type="kernel")` and `IV2SLS.fit(cov_type="kernel")` gave the same numbers on
both releases with no warning and are not reported.
