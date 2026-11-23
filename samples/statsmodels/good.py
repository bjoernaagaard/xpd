"""Supported API forms and lookalikes the statsmodels pack should leave alone."""

from math import log

import numpy
import numpy as np
import statsmodels.api as sm
import statsmodels.formula.api as smf
from statsmodels.regression.rolling import RollingOLS, RollingWLS
from statsmodels.tsa.ar_model import AutoReg, ar_select_order
from statsmodels.tsa.stattools import grangercausalitytests, kpss


def regression(y, X, data, weights):
    sm.WLS(y, X, weights=weights)
    smf.wls("y ~ x", data, weights=weights)
    smf.ols("y ~ x + I(x**2)", data)
    smf.ols("y ~ (x + z)**2", data)
    smf.ols("y ~ x**1", data)
    smf.ols(r"y ~ I(z + x**2 + a) + (a + b)**2", data)
    smf.ols(f"y ~ {data.attrs['term']} + x**2", data)


def generalized(y, X, data, exposure):
    family = sm.families.Poisson()
    sm.GLM(y, X, family=family, exposure=exposure)
    smf.glm("y ~ x", data, family=family, offset=np.log(exposure))
    sm.GLM(y, X, family, np.log(exposure))
    sm.GLM(y, X, family, None, exposure)


def compare(y, X, reduced, other):
    full = sm.OLS(y, X).fit(cov_type="HC3")
    full.wald_test("x = 0", scalar=True)
    full = sm.OLS(y, X).fit()
    restricted = sm.OLS(y, reduced).fit()
    full.compare_f_test(restricted)
    sm.OLS(y, X).fit("pinv", "nonrobust").compare_f_test(restricted)
    full = other
    full.compare_f_test(sm.OLS(y, reduced).fit(cov_type="HC3"))


def rolling(y, X, weights):
    RollingOLS(y, X, window=12).fit(params_only=True).params
    RollingOLS(y, X, window=12).fit(method="pinv").params
    RollingWLS(y, X, weights=weights, window=12).fit().bse


def current(y, series):
    AutoReg(y, lags=2)
    ar_select_order(y, maxlag=2)
    grangercausalitytests(series, maxlag=2)
    kpss(y, nlags="auto", result_object=True)
    kpss(y, "c", "auto", False, result_object=None)


def random_methods(series, options):
    result = sm.tsa.VAR(series).fit(1)
    result.simulate_var(steps=10, rng=17)
    result.irf_errband_mc(repl=4, steps=2, rng=17, burn=3)
    result.simulate_var(10, None, 17)
    result.simulate_var(steps=10, **options)


def shadowed(sm, smf, kpss, result, y, X, data, weights):
    sm.OLS(y, X, weights=weights)
    smf.ols("y ~ x**2", data)
    kpss(y, nlags=None)
    result.simulate_var(seed=17)


def other_log(np, y, X, exposure):
    sm.GLM(y, X, exposure=np.log(exposure))


def shadowed_module(numpy, y, X, exposure):
    sm.GLM(y, X, exposure=numpy.log(exposure))


def shadowed_log(y, X, log_transform):
    log = log_transform
    sm.GLM(y, X, exposure=log(2))
