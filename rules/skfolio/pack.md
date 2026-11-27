# skfolio

The skfolio pack catches skfolio 1.14 migrations, financial validation leaks, return and weight accounting errors, and optimizer misconfigurations on files that import `skfolio`.

## Version migrations

- removed-in-skfolio
- skfolio-factor-model-class
- deprecated-in-skfolio
- skfolio-hierarchical-seriation
- skfolio-base-hierarchical-check

## Financial validation

- skfolio-shuffled-returns-split
- skfolio-preselection-before-split
- skfolio-fit-on-holdout
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
