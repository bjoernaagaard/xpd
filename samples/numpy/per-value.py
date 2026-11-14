"""Python work per value on arrays.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def bigger(a, b):
    return a if a > b else b


@case
def vectorize():
    a, b = np.arange(10.0), np.full(10, 4.5)
    return np.vectorize(bigger)(a, b)  # xpd: np-vectorize
