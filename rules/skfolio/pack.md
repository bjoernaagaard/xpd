# skfolio

The skfolio pack checks portfolio code against skfolio 1.14.4.

## Version migrations

- removed-in-skfolio
- skfolio-factor-model-class
- deprecated-in-skfolio
- skfolio-hierarchical-seriation
- skfolio-base-hierarchical-check

## Financial validation

- skfolio-shuffled-returns-split
- skfolio-preselection-before-split
- skfolio-sklearn-cross-val-predict
- skfolio-cpcv-zero-purge

## Returns and portfolio accounting

- skfolio-log-returns-in-optimizer
- skfolio-prices-not-returns
- skfolio-weight-drift-uncompounded
- skfolio-cost-periodicity
- skfolio-unchecked-failed-portfolio

## Optimizer configuration

- skfolio-string-enum-option
- skfolio-invalid-convex-bounds
- skfolio-estimator-on-optimizer
- skfolio-missing-fallback-in-cv

## Online evaluation and metadata

- skfolio-fit-loop-vs-online
- skfolio-grid-search-vs-online
- skfolio-metadata-routing-setup

## Asset panels and reproducibility

- skfolio-panel-inactive-policy
- skfolio-unseeded-estimator


## Runtime evidence

The proofs ran with skfolio 1.14.4, scikit-learn 1.9.1, NumPy 2.5.5,
pandas 3.1.1 and cvxpy-base 1.9.3 on Python 3.13. Exceptions and warnings
were measured with `python -W error`. Unless a row names another input,
the optimization examples used 60 observations of 3 assets with NumPy
generator seed 42.

| Rule | Severity | Measured result | Fix |
| --- | --- | --- | --- |
| [removed-in-skfolio](removed-in-skfolio.md) | Error | Both EW constructors rejected `alpha=0.2` with TypeError. Both fit with a half-life of about 3.1063. WalkForward accepted `expand_train=True` and rejected `expend_train=True`. UncertaintySet raised TypeError for the 2 old keywords and AttributeError for the 2 old fields. | Unsafe for the WalkForward keyword |
| [skfolio-factor-model-class](skfolio-factor-model-class.md) | Error | The dataclass required 13 constructor arguments. Positional factors raised TypeError in direct and parent fits. The direct keyword fit returned 3 asset means. | None |
| [deprecated-in-skfolio](deprecated-in-skfolio.md) | Error, warning | Both old hierarchical export paths, both sort forms and the old annualization name raised FutureWarning. The 3 constructors tested raised ValueError when given both annualization names. Alternate spellings gave equal values of the same type in all 3. | Safe for the annualization keyword |
| [skfolio-hierarchical-seriation](skfolio-hierarchical-seriation.md) | Error, warning | Both HRP and Schur raised FutureWarning for the old option and alias. Both raised ValueError when given 2 ordering estimators and exposed a fitted HierarchicalClustering through the new path. HERC and NCO fit with their clustering keywords. | None |
| [skfolio-base-hierarchical-check](skfolio-base-hierarchical-check.md) | Warning, info | All 3 built-in hierarchical optimizers tested returned False for both checks against the old base. All 3 were subclasses of BaseOptimization. | None |
| [skfolio-shuffled-returns-split](skfolio-shuffled-returns-split.md) | Warning | For 20 rows, the default holdout and ShuffleSplit trained on row 19 while testing on row 0. The ordered holdout trained through row 14 and tested from row 15. All 4 shuffled KFold splits included training dates later than test dates. | None |
| [skfolio-preselection-before-split](skfolio-preselection-before-split.md) | Warning | Selecting the asset with the highest mean chose A from 3 training rows and B from all 6 rows. | None |
| [skfolio-sklearn-cross-val-predict](skfolio-sklearn-cross-val-predict.md) | Warning | scikit-learn converted an EqualWeighted prediction into a 60-element array with KFold and raised with 3 financial splitters. skfolio returned a MultiPeriodPortfolio for WalkForward and a Population for the other 2. | None |
| [skfolio-cpcv-zero-purge](skfolio-cpcv-zero-purge.md) | Info | In the first combinatorial split, training contained 36 rows with both gaps at 0 and 34 with both gaps at 1. | None |
| [skfolio-log-returns-in-optimizer](skfolio-log-returns-in-optimizer.md) | Warning | The means on the same 59 periods were 0.0002523 using log returns and 0.0003062 using linear returns. | None |
| [skfolio-prices-not-returns](skfolio-prices-not-returns.md) | Warning | The 3 loaders returned prices. The mean of an equal-weighted example was 101.7354 using levels and 0.0003062 after conversion. | None |
| [skfolio-weight-drift-uncompounded](skfolio-weight-drift-uncompounded.md) | Info | Period returns were 0.1 and 0.109091 in both modes. Both Portfolio and MultiPeriodPortfolio gave 0.209091 as the simple cumulative return or 1.22 as final compounded wealth. | None |
| [skfolio-cost-periodicity](skfolio-cost-periodicity.md) | Info | A 1% transaction cost charged 0.01 from omitted holdings and 0 from identical holdings. | None |
| [skfolio-unchecked-failed-portfolio](skfolio-unchecked-failed-portfolio.md) | Info | An infeasible fit with exceptions disabled returned FailedPortfolio with 3 NaN weights and a NaN mean. | None |
| [skfolio-string-enum-option](skfolio-string-enum-option.md) | Error | The string risk measure raised AttributeError on 2 optimizers. The string objective raised TypeError on MeanRisk. | None |
| [skfolio-invalid-convex-bounds](skfolio-invalid-convex-bounds.md) | Error, warning | Bounds of 0.7 and 0.2 were infeasible on both optimizers. Each penalty at -0.1 raised DCPError on MeanRisk. The selected tail measures at -0.2 or 1.2 failed with an unbounded objective or an invalid logarithm. | None |
| [skfolio-estimator-on-optimizer](skfolio-estimator-on-optimizer.md) | Error | Both moment keywords raised TypeError on MeanRisk and RiskBudgeting. Both optimizers fit with EmpiricalPrior. | None |
| [skfolio-missing-fallback-in-cv](skfolio-missing-fallback-in-cv.md) | Info | A deliberately infeasible constraint aborted validation. EqualWeighted as fallback returned 4 test periods. | None |
| [skfolio-fit-loop-vs-online](skfolio-fit-loop-vs-online.md) | Info | The default MeanRisk prior failed with missing partial_fit support. An empirical prior with EWMu and EWCovariance ran online with a half-life of 8 and warmup of 20. | None |
| [skfolio-grid-search-vs-online](skfolio-grid-search-vs-online.md) | Info | Batch GridSearchCV ran with the default prior. OnlineGridSearch required a prior with incremental moment estimators and enough warmup observations. | None |
| [skfolio-metadata-routing-setup](skfolio-metadata-routing-setup.md) | Info | A direct implied volatility fit produced a 3 by 3 covariance matrix with routing disabled. The nested fit required a request. With routing and the request enabled, the optimizer returned 3 weights. | None |
| [skfolio-panel-inactive-policy](skfolio-panel-inactive-policy.md) | Error, info | A string policy raised TypeError. Categorical ZERO raised ValueError because 0 is a valid code. A raw field with 1 finite inactive value raised ValueError. A DataFrame supplied as characteristics raised TypeError. | None |
| [skfolio-unseeded-estimator](skfolio-unseeded-estimator.md) | Info | The 5 seedable constructors tested kept seed 42. Random reproduced all 3 weights when NumPy's global seed was reset to 42. | None |

## Library audit

The source scan covered the 283 installed skfolio Python files. The first
pass reported an internal forwarding call for management fees and a library
default for an unseeded distribution estimator. The pack now checks literal
fee entries and leaves library fallback factory defaults alone. The final
scan reported 0 skfolio diagnostics. A separate scan of [skfolio portfolio
analytics at 03bcc1f](https://github.com/JDS-265/skfolio-portfolio-analytics/tree/03bcc1f411ff9a7b54d2f5f4aa5ed0cb6723680b)
covered two Python files that import skfolio and reported zero skfolio diagnostics.

[API reference](https://skfolio.org/api.html) | [Migration guide](https://skfolio.org/user_guide/migration.html)

## What the pack leaves out

Rules follow imports and local assignments to prove that a call uses skfolio.
Names imported under a different class name, dynamically unpacked options
and transformations hidden behind arbitrary user functions can require
manual review.

- Importing the deprecated cluster modules alone does not warn. The warning
  comes from accessing their deprecated exports. The import rule checks
  explicit imports of optimizer classes from the old paths.
- The confidence checks leave 0 and 1 alone because both endpoints worked
  for the tested MeanRisk tail measures. The weight bounds check covers
  literal contradictions across zero, one and the middle of the unit
  interval. It leaves computed values and pairwise array comparisons alone.
- The preselection rule follows data from a direct fit_transform into a split
  or validation call. The cross-sectional scalers operate within each
  observation, so the pack leaves those scalers out of the leakage warning.
  Plain KFold and generic sklearn preprocessing are outside this pack.
- Zero purging, uncompounded drift, cost periodicity, failed-portfolio
  handling and missing fallback are advice. Their effect depends on data,
  evaluation intent or error handling elsewhere. The online rules remain
  advice because estimator support and warmup vary and no general speed
  improvement was established.
- Metadata routing can be configured outside the file. A time-series prior
  may consume factors directly, and ImpliedCovariance can consume direct
  implied_vol. The rule advises on parent calls and checks an explicit
  nested implied-volatility request where the constructor shows it.
- ConvexOptimization is abstract. It cannot be instantiated by moving
  moment options to a prior. Random has no per-estimator seed, SyntheticData
  uses its distribution estimator, and the bootstrap estimators take seed.
