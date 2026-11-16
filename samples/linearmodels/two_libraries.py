"""A file that imports IV2SLS and OLS from both linearmodels and statsmodels.

The two names are classes of both libraries. With both imports in the file, nothing shows which
class a call builds, so the rules about them report nothing here. The names that only
linearmodels has are still read.
"""

import statsmodels.api as sm
from linearmodels import IV2SLS, OLS, PanelOLS
from linearmodels.datasets import mroz, wage_panel
from statsmodels.regression.linear_model import OLS  # noqa: F811
from statsmodels.sandbox.regression.gmm import IV2SLS  # noqa: F811

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def statsmodels_classes():
    data = mroz.load().dropna().assign(const=1.0)
    exog = data[["const", "exper", "educ"]]
    instrument = data[["const", "exper", "fatheduc", "motheduc"]]
    # statsmodels takes (endog, exog, instrument), and its results have summary() and bse.
    two_stage = IV2SLS(data.lwage, exog, instrument).fit()
    least_squares = OLS(data.lwage, exog).fit(cov_type="HC1")
    module = sm.OLS(data.lwage, exog).fit(cov_type="cluster", cov_kwds={"groups": data.kidslt6})
    return two_stage.params, least_squares.bse, module.bse, str(least_squares.summary())


@case
def linearmodels_classes():
    data = wage_panel.load().set_index(["nr", "year"])
    pooled = PanelOLS(data.lwage, sm.add_constant(data[["exper"]])).fit(cov_type="robust")  # xpd: panel-ols-without-effects
    return pooled.params


@case
def lambda_parameter():
    data = wage_panel.load().set_index(["nr", "year"])
    result = PanelOLS(data.lwage, data[["exper"]], entity_effects=True).fit(cov_type="robust")
    ols = sm.OLS(data.lwage, data[["exper"]]).fit()
    # The lambda's parameter has the name of the linearmodels result and holds a statsmodels one.
    read = lambda result: result.bse  # noqa: E731
    return result.std_errors, read(ols)
