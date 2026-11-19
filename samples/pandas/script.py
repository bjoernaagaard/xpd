"""A script that builds one frame line by line at the top of the module.

A line that xpd reports ends with a comment that names the rule. samples/pandas/steps.py
computes the same frame with step functions, and `xpd equiv samples/pandas/script.py
samples/pandas/steps.py` compares the two.
"""

import numpy as np
import pandas as pd

RATE = 1.25

r = np.random.default_rng(0)
df = pd.DataFrame({  # xpd: script-steps
    "price": r.normal(100, 15, 200).round(2),
    "qty": r.integers(1, 9, 200),
    "city": r.choice(["Oslo", "Bergen", "Trondheim"], 200),
})
df = df[df["price"] > 90]  # xpd: filter-ladder
df = df[df["qty"] >= 2]
df = df[df["city"] != "Oslo"]
df["total"] = df["price"] * df["qty"]  # xpd: frame-column-write
df["with_tax"] = df["total"] * RATE
df["large"] = df["qty"] > 4
df["share"] = df["total"] / df["total"].sum()
regions = pd.DataFrame({"city": ["Bergen", "Trondheim"], "region": ["west", "mid"]})
df = df.merge(regions, on="city", validate="many_to_one")
result = df.reset_index(drop=True)
