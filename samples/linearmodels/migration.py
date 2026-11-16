"""Forms that a release of linearmodels removed or deprecated, next to the forms that replace them.

A line that xpd reports ends with a comment that names the rule. A removed form raises on
linearmodels 7.1, so it sits in a function that is not a case and that nothing calls. Each case
returns what the code computes, so a copy fixed with `xpd check --fix` can be compared with this
file.
"""

import warnings

import linearmodels
import numpy as np
import pandas as pd
from linearmodels import WARN_ON_MISSING, AbsorbingLS, OLS
from linearmodels.__future__ import ordering  # xpd: deprecated-in-linearmodels
from linearmodels.shared.exceptions import MissingValueWarning
from linearmodels.shared.hypotheses import WaldTestStatistic

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def table(n=400, seed=0):
    r = np.random.default_rng(seed)
    data = pd.DataFrame({"y": r.normal(size=n), "x": r.normal(size=n), "z": r.normal(size=n), "w": r.normal(size=n)})
    return data.assign(firm=pd.Categorical(r.integers(0, 20, n)))


@case
def formula_order():
    data = table()
    return list(OLS.from_formula("y ~ 1 + z + x + w", data).fit().params.index)


@case
def absorb_options():
    data = table()
    model = AbsorbingLS(data.y, data[["x"]], absorb=data[["firm"]])
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", FutureWarning)
        old = model.fit(method="lsmr", lsmr_options={"atol": 1e-10, "btol": 1e-10})  # xpd: deprecated-in-linearmodels
    new = model.fit(method="lsmr", absorb_options={"atol": 1e-10, "btol": 1e-10})
    return old.params, new.params


@case
def missing_values():
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", FutureWarning)
        old = linearmodels.DROP_MISSING  # xpd: deprecated-in-linearmodels
    data = table()
    data.loc[0, "x"] = np.nan
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", MissingValueWarning)
        result = OLS(data.y, data[["x"]]).fit()
    return old, WARN_ON_MISSING, result.nobs


@case
def wald_test():
    data = table()
    result = OLS(data.y, data[["x", "z"]]).fit()
    test = result.wald_test(np.eye(2), np.zeros(2))
    return isinstance(test, WaldTestStatistic), test.stat


def removed():
    from linearmodels import DROP_MISSING  # xpd: deprecated-in-linearmodels
    from linearmodels.__future__ import ordering  # xpd: deprecated-in-linearmodels
    from linearmodels.utility import AttrDict, MissingValueWarning as OldWarning  # xpd: removed-in-linearmodels
    import linearmodels.utility  # xpd: removed-in-linearmodels

    data = table()
    result = OLS(data.y, data[["x", "z"]]).fit()
    test = result.test_linear_constraint(np.eye(2), np.zeros(2))  # xpd: removed-in-linearmodels
    chained = OLS(data.y, data[["x"]]).fit().test_linear_constraint(np.eye(1), np.zeros(1))  # xpd: removed-in-linearmodels
    try:
        from linearmodels.utility import WaldTestStatistic as Old
    except ImportError:
        Old = WaldTestStatistic
    return DROP_MISSING, AttrDict, OldWarning, test, chained, Old
