"""Names of statsmodels results read on linearmodels results, and options of compare.

A line that xpd reports ends with a comment that names the rule. A form that raises on
linearmodels 7.1 sits in a function that is not a case and that nothing calls. Each case returns
what the code computes.
"""

import linearmodels.panel as lmp
import numpy as np
import pandas as pd
import statsmodels.api as sm
from linearmodels import IV2SLS, PanelOLS, PooledOLS, RandomEffects
from linearmodels.datasets import mroz, wage_panel
from linearmodels.iv import compare as compare_iv
from linearmodels.panel import compare

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def panel():
    data = wage_panel.load().set_index(["nr", "year"])
    return data.lwage, sm.add_constant(data[["hours", "married"]])


def women():
    data = mroz.load().dropna().reset_index(drop=True)  # xpd: sort-then-reset-index
    return data.assign(const=1.0)


@case
def right_names():
    y, X = panel()
    fixed = PanelOLS(y, X, entity_effects=True).fit(cov_type="clustered", cluster_entity=True)
    table = fixed.summary
    # A statsmodels result has these names, and the rule leaves it alone.
    ols = sm.OLS(y, X).fit()
    text = ols.summary()
    return str(table), text, fixed.std_errors, fixed.tstats, fixed.resids, fixed.fitted_values, ols.bse, ols.tvalues, ols.resid, ols.fittedvalues, ols.aic


@case
def side_by_side():
    y, X = panel()
    pooled = PooledOLS(y, X).fit(cov_type="robust")
    fixed = PanelOLS(y, X, entity_effects=True).fit(cov_type="robust")
    with_errors = compare({"pooled": pooled, "fixed": fixed}, precision="std_errors")
    with_stars = lmp.compare([pooled, fixed], precision="pvalues", stars=True)
    default = compare({"pooled": pooled, "fixed": fixed})
    return str(with_errors.summary), str(with_stars.summary), str(default.summary)


@case
def random_against_fixed():
    y, X = panel()
    fixed = PanelOLS(y, X, entity_effects=True).fit(cov_type="unadjusted")
    random = RandomEffects(y, X).fit(cov_type="unadjusted")

    def hausman(fe, re):  # xpd: hand-written-hausman
        b = fe.params - re.params
        v = fe.cov - re.cov
        return float(b @ np.linalg.inv(v) @ b)

    def difference(fe, re):
        return fe.params - re.params

    # The Hausman and Taylor estimator is not a test, and a table of the test is not one either.
    def hausman_taylor(fe, re):
        return (fe.params + re.params) / 2

    def hausman_table(fe, re):
        return pd.concat([fe.params, re.params], axis=1)  # xpd: concat-columns-unchecked

    return hausman(fixed, random), difference(fixed, random), random.hausman(fixed), hausman_taylor(fixed, random), hausman_table(fixed, random).shape


def raises_on_7_1():
    y, X = panel()
    data = women()
    model = PanelOLS(y, X, entity_effects=True)
    fixed = model.fit(cov_type="robust")
    print(fixed.summary())  # xpd: statsmodels-result-name
    print(PooledOLS(y, X).fit(cov_type="robust").summary())  # xpd: statsmodels-result-name
    errors = fixed.bse  # xpd: statsmodels-result-name
    t = fixed.tvalues  # xpd: statsmodels-result-name
    residuals = fixed.resid  # xpd: statsmodels-result-name
    fitted = fixed.fittedvalues  # xpd: statsmodels-result-name
    criteria = fixed.aic, fixed.bic  # xpd: statsmodels-result-name, statsmodels-result-name
    covariance = fixed.cov_params()  # xpd: statsmodels-result-name
    likelihood = fixed.llf  # xpd: statsmodels-result-name
    guarded = fixed.tstats if hasattr(fixed, "tstats") else fixed.tvalues
    two_stage = IV2SLS(data.lwage, data[["const", "exper"]], data.educ, data[["fatheduc", "motheduc"]]).fit()
    f = two_stage.fvalue  # xpd: statsmodels-result-name
    adjusted = two_stage.rsquared_adj
    compare({"fixed": fixed}, precision="se")  # xpd: invalid-compare-precision
    compare_iv({"iv": two_stage}, precision="tstats")
    lmp.compare([fixed], precision="stderr")  # xpd: invalid-compare-precision
    choice = "se"
    compare({"fixed": fixed}, precision=choice)

    # A name that is also assigned from something else is not known to be a linearmodels result.
    either = model.fit(cov_type="robust")
    either = sm.OLS(y, X).fit()
    print(either.summary(), either.bse)
    for looped in (sm.OLS(y, X).fit(),):
        print(looped.summary())
    return errors, t, residuals, fitted, criteria, covariance, likelihood, guarded, f, adjusted


def passed_in(fixed):
    return fixed.bse, fixed.summary()
