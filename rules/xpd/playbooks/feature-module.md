# A feature engineering module

A feature module in the house style has one function for each feature. The function is named for the column it returns, each parameter names the feature or the input column it reads, and each has a type. `xpd graph` reads the module as a dataflow without running it, and Apache Hamilton can run the same module as it is.

## Asks

- write a feature engineering module
- add features for a model
- engineer features from columns
- build model features as functions
- create lag and rolling features

## Code

```python
"""Features of hourly power prices for a model, with one function for each feature.

Each function is named for the column it returns, and each parameter names the feature or the
input column it reads. `xpd graph` reads the module as a dataflow without running it.

    inputs: price, load, wind
    ├── residual_load()          load that wind does not cover
    ├── price_day_mean()         the mean price of the last 24 hours
    ├── price_above_day_mean()   how far the price is above that mean
    └── scarcity()               the price for each unit of residual load
"""

import pandas as pd

HOURS_IN_DAY = 24


def residual_load(load: pd.Series, wind: pd.Series) -> pd.Series:
    return load - wind


def price_day_mean(price: pd.Series) -> pd.Series:
    return price.rolling(HOURS_IN_DAY).mean()


def price_above_day_mean(price: pd.Series, price_day_mean: pd.Series) -> pd.Series:
    return price - price_day_mean


def scarcity(price: pd.Series, residual_load: pd.Series) -> pd.Series:
    return price / residual_load.where(residual_load > 0)
```

## Commands

1. `xpd profile <data file>` prints the dtype, the nulls and the unique values of each column that a feature reads.
2. `xpd graph <file> --format json` lists each feature with the names it reads. `inputs` are the columns that the caller passes, and a warning names an input that is one edit from a feature name.
3. `xpd check <file> --format json` reports the issues of the module, and the code is done when it reports 0 issues.
4. `xpd equiv old.py new.py` proves that the features equal the columns of the code they replace.

## Rules

- hamilton/node-mutates-input
- pandas/frame-column-write
- pandas/row-loop
- python/pass-through-function
