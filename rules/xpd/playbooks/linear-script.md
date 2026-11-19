# A mutating script turned into a linear chain

A script that filters a frame line by line and writes its columns one statement at a time becomes one chain that reads from top to bottom. Each stage is a step function, the script assigns `result` once, and `xpd equiv` proves that the new script gives the frame of the old one.

## Asks

- turn this non-linear code into a linear pipeline
- make this linear
- make this script linear
- refactor a script that mutates a dataframe line by line
- rewrite df = df[...] and df["x"] = ... as one method chain
- untangle this notebook code

## Code

```python
"""The paid Nordic orders with their amounts, written as one chain that reads top to bottom.

The script it replaces filtered `df` three times and wrote three columns to it, one statement at
a time. Here each stage is a function, and `result` is the only name that the script assigns.

    nordic_orders(orders)
    ├── paid_nordic()   paid orders of Denmark, Sweden and Norway with a positive quantity
    └── amounts()       amount, net amount and the flag for a large order
"""

import pandas as pd

NORDIC = ["DK", "SE", "NO"]
LARGE_ORDER = 1000


def paid_nordic(orders: pd.DataFrame) -> pd.DataFrame:
    paid = orders["status"] == "paid"
    ordered = orders["quantity"] > 0
    return orders.loc[paid & ordered & orders["country"].isin(NORDIC)]


def amounts(orders: pd.DataFrame) -> pd.DataFrame:
    amount = orders["quantity"] * orders["unit_price"]
    net_amount = amount - orders["discount"]
    return orders.assign(
        amount=amount,
        net_amount=net_amount,
        is_large=net_amount > LARGE_ORDER,
    )


def nordic_orders(orders: pd.DataFrame) -> pd.DataFrame:
    return orders.pipe(paid_nordic).pipe(amounts)


result = nordic_orders(pd.read_csv("orders.csv"))
```

## Commands

1. `xpd check old.py --format json` lists the shape of the old script, such as `pandas/filter-ladder` and `pandas/frame-column-write`.
2. Keep the old script unchanged and end both scripts with `result = ...`, because the old script is the judge.
3. `xpd equiv old.py new.py` runs both scripts on the real rows, and the rewrite is done when it prints `{"equivalent": true}`.
4. `xpd graph new.py --format json` lists the steps of the chain and the columns each one reads.
5. `xpd check new.py --format json` reports 0 issues for a script in the style.

## Rules

- pandas/filter-ladder
- pandas/frame-column-write
- pandas/script-steps
- pandas/reassign-ladder
- pandas/nested-step-calls
- pandas/chained-assignment
