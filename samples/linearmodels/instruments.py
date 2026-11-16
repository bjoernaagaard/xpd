"""Instrument models and AbsorbingLS given their arguments in the wrong role or the wrong type.

A line that xpd reports ends with a comment that names the rule. A form that raises on
linearmodels 7.1 sits in a function that is not a case and that nothing calls. Each case returns
what the code computes.
"""

import numpy as np
import pandas as pd
from linearmodels import IV2SLS, IVGMM, IVLIML, OLS
from linearmodels.datasets import mroz
from linearmodels.iv import absorbing

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def women():
    return mroz.load().dropna().assign(const=1.0)


@case
def roles_by_name():
    data = women()
    dependent, instruments = data.lwage, data[["fatheduc", "motheduc"]]
    exog, endog = data[["const", "exper"]], data.educ
    crossed = IV2SLS(dependent, endog, exog, instruments).fit()  # xpd: iv-arguments-swapped
    right = IV2SLS(dependent, exog, endog, instruments).fit()
    by_keyword = IV2SLS(dependent, endog=endog, exog=exog, instruments=instruments).fit()
    limited = IVLIML(dependent, exog, endog, instruments).fit()
    return crossed.params, right.params, by_keyword.params, limited.params


@case
def through_the_origin():
    data = women()
    no_intercept = IV2SLS(data.lwage, None, data.educ, data[["fatheduc", "motheduc"]]).fit()  # xpd: iv-without-exog
    also_none = IVGMM(data.lwage, None, data[["educ"]], data[["fatheduc", "motheduc"]], weight_type="robust").fit()  # xpd: iv-without-exog
    with_constant = IV2SLS(data.lwage, data[["const"]], data.educ, data[["fatheduc", "motheduc"]]).fit()
    least_squares = IV2SLS(data.lwage, data[["const", "exper"]], None, None).fit()
    return no_intercept.params, also_none.params, with_constant.params, least_squares.params


@case
def second_stage_by_hand():
    data = women()
    first_stage = ["const", "exper", "fatheduc", "motheduc"]
    first = OLS(data.educ, data[first_stage]).fit()
    by_hand = OLS(data.lwage, pd.concat([data[["const", "exper"]], first.fitted_values], axis=1)).fit()  # xpd: manual-two-stage
    by_keyword = OLS(data.lwage, exog=first.fitted_values.assign(const=1.0)).fit()  # xpd: manual-two-stage
    # IV2SLS with None for endog and for instruments is least squares, so this is the same second stage.
    by_iv_class = IV2SLS(data.lwage, pd.concat([data[["const", "exper"]], first.fitted_values], axis=1), None, None).fit()  # xpd: manual-two-stage
    one_step = IV2SLS(data.lwage, data[["const", "exper"]], data.educ, data[["fatheduc", "motheduc"]]).fit()
    # The fitted values as the dependent variable are another regression, not a second stage.
    of_the_fit = OLS(first.fitted_values, data[["const", "exper"]]).fit()
    return by_hand.std_errors, by_keyword.std_errors, by_iv_class.std_errors, one_step.std_errors, of_the_fit.params


@case
def absorbed_as_a_number(n=2000, firms=100):
    r = np.random.default_rng(0)
    firm = r.integers(0, firms, n)
    effect = r.normal(size=firms)[firm] * 2
    x = r.normal(size=n) + effect
    data = pd.DataFrame({"y": x + 3 * effect + r.normal(size=n), "x": x, "firm": firm})
    continuous = absorbing.AbsorbingLS(data.y, data[["x"]], absorb=data[["firm"]]).fit()  # xpd: absorb-not-categorical
    no_absorb = absorbing.AbsorbingLS(data.y, data[["x"]]).fit()
    return continuous.params, no_absorb.params


def raises_on_7_1():
    data = women()
    IV2SLS(data.lwage, data[["const", "exper"]], data[["fatheduc"]])  # xpd: iv-missing-arguments
    IVLIML(data.lwage, data[["const", "exper"]])  # xpd: iv-missing-arguments
    IV2SLS(data.lwage, data[["const", "exper"]], data.educ, instrument=data[["fatheduc"]])  # xpd: iv-missing-arguments
    IV2SLS(data.lwage, data[["const", "exper"]], endog=data.educ, instruments=data[["fatheduc"]])
    IV2SLS(data.lwage, data[["const", "exper"]], None, None)
    parts = (data.lwage, data[["const", "exper"]], data.educ, data[["fatheduc"]])
    IV2SLS(*parts)
    OLS(data.lwage, data[["const", "exper"]])
