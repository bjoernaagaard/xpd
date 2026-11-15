"""SciPy names that are gone or that warn.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
import scipy.integrate as si
from scipy import integrate

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def removed_integration_names(y):
    a = si.trapz(y)  # xpd: removed-in-scipy
    b = integrate.cumtrapz(y)  # xpd: removed-in-scipy
    c = np.trapezoid(y)
    return a, b, c


@case
def current_integration_names():
    y = np.array([1.0, 2.0, 4.0])
    return si.trapezoid(y), integrate.simpson(y)
