"""Calls that exercise the statsmodels diagnostics."""

from math import log, log as natural_log

import numpy
import numpy as np
import statsmodels.api as sm
import statsmodels.formula.api as smf
from statsmodels.regression.rolling import RollingOLS, RollingWLS
from statsmodels.tsa.ar_model import AutoReg, ar_select_order
from statsmodels.tsa.stattools import grangercausalitytests, kpss


def regression(y, X, data, weights):
    sm.OLS(y, X, weights=weights)  # xpd: ols-ignored-weights
    smf.ols("y ~ x", data, weights=weights)  # xpd: ols-ignored-weights
    sm.OLS.from_formula("y ~ x", data, weights=weights)  # xpd: ols-ignored-weights
    smf.ols("y ~ x + x**2 + z ** 10", data)  # xpd: statsmodels-formula-power-term
    sm.OLS.from_formula(formula='y ~ x ** 3 + z', data=data)  # xpd: statsmodels-formula-power-term
    smf.ols(r"y ~ x**2", data)  # xpd: statsmodels-formula-power-term
    smf.ols("y ~ x**2 + I(z**2)", data)  # xpd: statsmodels-formula-power-term
    smf.ols("y ~ x**2 + (a + b)**2", data)  # xpd: statsmodels-formula-power-term
    smf.ols("y ~ I(z + a**2 + b) + (a + b)**2 + x**2 + z**3", data)  # xpd: statsmodels-formula-power-term


def generalized(y, X, data, exposure):
    family = sm.families.Poisson()
    sm.GLM(y, X, family=family, exposure=np.log(exposure))  # xpd: glm-logged-exposure
    smf.glm("y ~ x", data, family=family, exposure=np.log(exposure))  # xpd: glm-logged-exposure
    sm.GLM.from_formula("y ~ x", data, family=family, exposure=natural_log(2))  # xpd: glm-logged-exposure
    sm.GLM(y, X, family=family, exposure=numpy.log(exposure))  # xpd: glm-logged-exposure
    sm.GLM(y, X, family=family, exposure=log(2))  # xpd: glm-logged-exposure
    sm.GLM(y, X, family, None, np.log(exposure))  # xpd: glm-logged-exposure


def compare(y, X, reduced, groups):
    full = sm.OLS(y, X).fit(cov_type="HC3")
    restricted = sm.OLS(y, reduced).fit()
    full.compare_f_test(restricted)  # xpd: compare-f-test-robust-covariance
    full = sm.OLS(y, X).fit()
    restricted = sm.OLS(y, reduced).fit(cov_type='HC1')
    full.compare_f_test(restricted=restricted)  # xpd: compare-f-test-robust-covariance
    restricted = sm.OLS(y, reduced).fit()
    full = sm.OLS(y, X).fit("pinv", "HC3")
    full.compare_f_test(restricted)  # xpd: compare-f-test-robust-covariance
    full = sm.OLS(y, X).fit(cov_type="cluster-jk", cov_kwds={"groups": groups})
    full.compare_f_test(restricted)  # xpd: compare-f-test-robust-covariance
    full = sm.OLS(y, X).fit(cov_type="cluster-crv3", cov_kwds={"groups": groups})
    full.compare_f_test(restricted)  # xpd: compare-f-test-robust-covariance
    full = sm.OLS(y, X).fit().get_robustcov_results(cov_type="HC3")
    full.compare_f_test(restricted)  # xpd: compare-f-test-robust-covariance


def rolling(y, X, weights):
    RollingOLS(y, X, window=2).fit().params  # xpd: rolling-fit-params-only
    model = RollingWLS(y, X, weights=weights, window=12)
    model.fit().params  # xpd: rolling-fit-params-only


def removed(y, series):
    AutoReg(y, lags=2, old_names=True)  # xpd: removed-tsa-arguments
    ar_select_order(y, maxlag=2, old_names=False)  # xpd: removed-tsa-arguments
    grangercausalitytests(series, maxlag=2, verbose=False)  # xpd: removed-tsa-arguments
    kpss(y, nlags=None, result_object=True)  # xpd: removed-tsa-arguments
    kpss(y, "c", None, result_object=True)  # xpd: removed-tsa-arguments
    kpss(y, nlags=(None), result_object=True)  # xpd: removed-tsa-arguments


def random_methods(series, seed):
    result = sm.tsa.VAR(series).fit(1)
    result.simulate_var(steps=10, seed=seed)  # xpd: deprecated-var-seed
    sm.tsa.VAR(series).fit(1).irf_errband_mc(False, 4, 2, 0.05, seed=17, burn=3)  # xpd: deprecated-var-seed
