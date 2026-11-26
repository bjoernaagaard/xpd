# arch

The arch pack catches removed arch 9.0 APIs, ignored or invalid options, unscaled returns that break volatility fits, and unseeded bootstraps on files that import [arch](https://bashtage.github.io/arch/). Rules fire only on calls bound to `arch` imports and literal arguments.

## Removed API

- removed-in-arch

## Options and arguments

- arch-unknown-option
- arch-ignored-argument
- arch-rejected-number

## The units of the data

- unscaled-returns
- rescaled-units

## Forecasts and fits

- analytic-forecast-horizon
- forecast-per-origin
- reindex-is-default
- fit-output-in-loop

## Bootstraps, tests and covariance estimators

- arch-unseeded-draws
- results-before-compute
- opt-bandwidth-as-bandwidth
