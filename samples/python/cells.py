"""A file that an editor runs cell by cell, and what xpd reports in it.

A line that xpd reports ends with a comment that names the rule. Without its cell markers the
file would also get result-discarded on lines 20 and 27, filter-ladder on line 33, and
import-side-effect on lines 14, 20, 26, 27 and 48.
"""

# %%
import logging

import numpy as np
import pandas as pd

logging.basicConfig(level=logging.INFO)

# %%
total = np.asarray(pd.read_csv("orders.csv")["total"])

# %%
total.round(2)

# %% [markdown]
# The value of the last statement of a cell is shown, so only the first call here does nothing.

# %%
total.cumsum()  # xpd: result-discarded
total.clip(0, 100)

# %%
orders = pd.read_csv("orders.csv")

# %%
orders = orders[orders["price"] > 0]

# %%
orders = orders[orders["qty"] > 0]

# %%
orders = orders[orders["region"] != "test"]


# %%
def top(frame: pd.DataFrame, count: int) -> pd.DataFrame:
    return frame.nlargest(count, "price")


# %%
print(top(orders, 5))
