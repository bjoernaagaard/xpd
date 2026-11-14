"""Whole-array math that picks a slow or numerically weak function.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


rng = np.random.default_rng(0)
x = rng.standard_normal(200)
y = rng.standard_normal(200)
grid = rng.standard_normal((20, 10))
n = 5


@case
def sort_then_select():
    return (
        np.sort(x)[0],  # xpd: np-sort-then-select
        np.sort(x)[-1],  # xpd: np-sort-then-select
        np.sort(x)[:5],  # xpd: np-sort-then-select
        np.sort(x)[-5:],  # xpd: np-sort-then-select
        np.sort(x)[::-1][:5],  # xpd: np-sort-then-select
        x[np.argsort(x)],  # xpd: np-sort-then-select
        np.argsort(x)[0],  # xpd: np-sort-then-select
        np.argsort(x)[-1],  # xpd: np-sort-then-select
        np.argsort(x)[:5],  # xpd: np-sort-then-select
        np.argsort(x)[-5:],  # xpd: np-sort-then-select
        np.argsort(x)[::-1][:5],  # xpd: np-sort-then-select
        np.argsort(x)[-5:][::-1],  # xpd: np-sort-then-select
        np.argsort(np.argsort(x)),  # xpd: np-sort-then-select
        np.sort(x)[1],
        np.sort(grid, axis=0)[0],
        np.sort(x)[:n],
        np.sort(x)[:150],
        np.sort(x)[::-1],
        y[np.argsort(x)],
        np.argsort(x)[1],
        np.argsort(x)[:n],
        np.argsort(x, kind="stable")[:5],
        np.partition(x, 5)[:5],
    )
