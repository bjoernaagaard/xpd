# A pandas pipeline of step functions

A pandas pipeline in the house style is a set of step functions joined with `pipe`. Each step takes the frame and returns it with `assign`, and one function names the steps in order. `xpd graph` reads which step needs which column, and it prints the `columns=` list for the read.

## Asks

- build a robust pandas pipeline
- build a pandas pipeline for data processing
- write a data processing pipeline with pipe and assign
- clean and prepare a dataframe in steps
- make my pandas code more robust

## Code

```python
"""Orders made ready for a report, with one function for each step.

    prepared(orders)
    ├── valid_rows()        rows with a customer and a positive quantity
    ├── amounts()           gross and net amount of each order
    └── customer_shares()   each order's share of what its customer spent
"""

from pathlib import Path

import pandas as pd

COLUMNS = ["customer", "quantity", "unit_price", "discount"]


def valid_rows(orders: pd.DataFrame) -> pd.DataFrame:
    known = orders["customer"].notna()
    return orders.loc[known & (orders["quantity"] > 0)]


def amounts(orders: pd.DataFrame) -> pd.DataFrame:
    gross = orders["quantity"] * orders["unit_price"]
    return orders.assign(gross_amount=gross, net_amount=gross - orders["discount"])


def customer_shares(orders: pd.DataFrame) -> pd.DataFrame:
    spent = orders.groupby("customer")["net_amount"].transform("sum")
    return orders.assign(customer_share=orders["net_amount"] / spent)


def prepared(orders: pd.DataFrame) -> pd.DataFrame:
    return orders.pipe(valid_rows).pipe(amounts).pipe(customer_shares)


def load(path: Path) -> pd.DataFrame:
    orders = pd.read_parquet(path, columns=COLUMNS)
    return prepared(orders)
```

## Commands

For model preparation, use `xpd data-prep start`, list with bare `xpd data-prep`, then use its `plan`, `approve`, `gate`, `drop`, `status` and `finish` stages with the `xpd-data-prep` skill.

1. `xpd profile <data file>` prints the dtype, the nulls and the unique values of each column, so each step is written for the real data.
2. `xpd graph <file> --format json` lists the steps and the columns each one reads. `pipelines[].source` is the `columns=` list for the read.
3. `xpd check <file> --format json` reports what is left of the old style, and the code is done when it reports 0 issues.
4. `xpd equiv old.py new.py` proves that a rewrite gives the same `result` as the code it replaces.

## Rules

- pandas/frame-column-write
- pandas/filter-ladder
- pandas/reassign-ladder
- pandas/nested-step-calls
- pandas/script-steps
- pandas/lambda-in-assign
- pandas/no-inplace
- pandas/read-parquet-all-columns
