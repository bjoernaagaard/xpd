"""String options that arch does not have, and arguments that it accepts and does not read.

A line that xpd reports ends with a comment that names the rule. An unknown option raises on arch
9.0.0, so it sits in a function that is not a case and that nothing calls. Each case returns what
the code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
import pandas as pd
from arch import arch_model
from arch.bootstrap import MCS, SPA, IIDBootstrap
from arch.data import sp500
from arch.unitroot import ADF, DFGLS, KPSS, PhillipsPerron, VarianceRatio, ZivotAndrews, auto_bandwidth
from arch.unitroot.cointegration import DynamicOLS, engle_granger, phillips_ouliaris

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def prices(rows=600):
    return np.log(sp500.load()["Adj Close"]).iloc[-rows:]


def returns(rows=600):
    return 100 * prices(rows + 1).diff().dropna()


def regressors(index, columns=("a", "b"), seed=0):
    walk = np.cumsum(np.random.default_rng(seed).standard_normal((len(index), len(columns))), 0)
    return pd.DataFrame(walk, index=index, columns=list(columns))


def unknown_model_names(trend):
    y = returns()
    gjr = arch_model(y, vol="gjr")  # xpd: arch-unknown-option
    tarch = arch_model(y, vol="TARCH", o=1)  # xpd: arch-unknown-option
    student = arch_model(y, dist="student")  # xpd: arch-unknown-option
    moving = arch_model(y, mean="MA")  # xpd: arch-unknown-option
    name = "gjr"
    silent = arch_model(y, vol=name, dist=f"{trend}")
    return gjr, tarch, student, moving, silent


def unknown_test_options(trend):
    p = prices()
    x = regressors(p.index)
    first = ADF(p, trend="nc")  # xpd: arch-unknown-option
    second = KPSS(p, trend="n")  # xpd: arch-unknown-option
    third = DFGLS(p, trend="ctt")  # xpd: arch-unknown-option
    fourth = PhillipsPerron(p, trend="ctt", test_type="z")  # xpd: arch-unknown-option, arch-unknown-option
    fifth = VarianceRatio(p, trend="ct")  # xpd: arch-unknown-option
    sixth = ZivotAndrews(p, trend="n", method="hqic")  # xpd: arch-unknown-option, arch-unknown-option
    seventh = engle_granger(p, x, trend="nc", method="hqic")  # xpd: arch-unknown-option, arch-unknown-option
    eighth = phillips_ouliaris(p, x, test_type="zrho")  # xpd: arch-unknown-option
    ninth = DynamicOLS(p, x, trend="t", method="t-stat")  # xpd: arch-unknown-option, arch-unknown-option
    tenth = auto_bandwidth(p.diff().dropna(), kernel="quadratic")  # xpd: arch-unknown-option
    silent = ADF(p, trend=trend)
    return first, second, third, fourth, fifth, sixth, seventh, eighth, ninth, tenth, silent


def unknown_fit_and_forecast_options(other):
    am = arch_model(returns())
    res = am.fit(disp="none", cov_type="mle")  # xpd: arch-unknown-option, arch-unknown-option
    first = res.forecast(horizon=2, method="monte carlo")  # xpd: arch-unknown-option
    second = res.forecast(horizon=2, align="end")  # xpd: arch-unknown-option
    third = res.hedgehog_plot(plot_type="variance")  # xpd: arch-unknown-option
    fourth = arch_model(returns()).fit(cov_type="hc0")  # xpd: arch-unknown-option
    # Another library's fit has cov_type values of its own, so an object that is not an arch model is left alone.
    fifth = other.fit(cov_type="HC0", disp="none")
    return first, second, third, fourth, fifth


def unknown_bootstrap_options(x, other):
    losses = np.random.default_rng(0).standard_normal((300, 4)) ** 2
    mcs = MCS(losses, 0.1, reps=100, method="range", seed=0)  # xpd: arch-unknown-option
    spa = SPA(losses[:, 0], losses[:, 1:], reps=100, bootstrap="block", seed=0)  # xpd: arch-unknown-option
    bs = IIDBootstrap(x, seed=0)
    first = bs.conf_int(np.mean, 100, method="percentile-t")  # xpd: arch-unknown-option
    second = bs.conf_int(np.mean, 100, tail="both")  # xpd: arch-unknown-option
    third = other.conf_int(np.mean, 100, method="percentile-t")
    return mcs, spa, first, second, third


@case
def options_arch_has():
    y = returns()
    p = prices()
    gjr = arch_model(y, vol="GARCH", o=1, dist="StudentsT", mean="Zero").fit(disp="off", cov_type="classic")
    adf = ADF(p, trend="ct", method="t-stat")
    kpss = KPSS(p, trend="ct")
    pp = PhillipsPerron(p, trend="n", test_type="rho")
    forecast = gjr.forecast(horizon=3, method="analytic", align="target")
    losses = np.random.default_rng(0).standard_normal((300, 4)) ** 2
    mcs = MCS(losses, 0.1, reps=100, method="max", bootstrap="cbb", seed=0)
    mcs.compute()
    bs = IIDBootstrap(losses[:, 0], seed=0)
    interval = bs.conf_int(np.mean, 100, method="bca", tail="upper")
    return gjr.params.tolist(), float(adf.stat), float(kpss.stat), float(pp.stat), forecast.variance.values.tolist(), mcs.pvalues.values.ravel().tolist(), interval.ravel().tolist()


@case
def arguments_arch_model_does_not_read():
    y = returns()
    x = regressors(y.index, columns=("x",))
    lagged = arch_model(y, lags=2).fit(disp="off")  # xpd: arch-ignored-argument
    zero = arch_model(y, mean="Zero", lags=[1, 5]).fit(disp="off")  # xpd: arch-ignored-argument
    with_x = arch_model(y, x=x, mean="AR", lags=1).fit(disp="off")  # xpd: arch-ignored-argument
    positional_x = arch_model(y, x).fit(disp="off")  # xpd: arch-ignored-argument
    arch_q = arch_model(y, vol="ARCH", p=1, q=1).fit(disp="off")  # xpd: arch-ignored-argument
    arch_power = arch_model(y, vol="arch", p=2, power=1.0).fit(disp="off")  # xpd: arch-ignored-argument
    egarch_power = arch_model(y, vol="EGARCH", p=1, o=1, q=1, power=1.0).fit(disp="off")  # xpd: arch-ignored-argument
    figarch_o = arch_model(y, vol="FIGARCH", p=1, o=1, q=1).fit(disp="off")  # xpd: arch-ignored-argument
    return [r.params.tolist() for r in (lagged, zero, with_x, positional_x, arch_q, arch_power, egarch_power, figarch_o)]


@case
def arguments_arch_model_reads(lags=2, mean="constant"):
    y = returns()
    x = regressors(y.index, columns=("x",))
    ar = arch_model(y, mean="AR", lags=2).fit(disp="off")
    arx = arch_model(y, x=x, mean="ARX", lags=1).fit(disp="off")
    no_lags = arch_model(y, lags=0, x=None).fit(disp="off")
    from_variables = arch_model(y, mean=mean, lags=lags).fit(disp="off")
    arch_only = arch_model(y, vol="ARCH", p=2, o=0, q=0, power=2.0).fit(disp="off")
    tarch = arch_model(y, vol="GARCH", p=1, o=1, q=1, power=1.0).fit(disp="off")
    return [r.params.tolist() for r in (ar, arx, no_lags, from_variables, arch_only, tarch)]


@case
def lag_search_arguments(lags=4):
    p = prices()
    fixed = ADF(p, lags=4, method="bic")  # xpd: arch-ignored-argument
    capped = DFGLS(p, lags=4, max_lags=12)  # xpd: arch-ignored-argument
    positional = ZivotAndrews(p, 4, method="t-stat")  # xpd: arch-ignored-argument
    searched = ADF(p, method="bic", max_lags=12)
    from_variable = ADF(p, lags=lags, method="bic")
    return float(fixed.stat), float(capped.stat), float(positional.stat), float(searched.stat), float(from_variable.stat)


@case
def seeds_forecast_does_not_read():
    res = arch_model(returns()).fit(disp="off")
    simulated = res.forecast(horizon=3, method="simulation", simulations=50, random_state=np.random.RandomState(0))  # xpd: arch-ignored-argument, legacy-random
    resampled = res.forecast(horizon=3, method="bootstrap", simulations=50, rng=np.random.default_rng(0).standard_normal)  # xpd: arch-ignored-argument
    seeded_simulation = res.forecast(horizon=3, method="simulation", simulations=50, rng=np.random.default_rng(0).standard_normal)
    seeded_bootstrap = res.forecast(horizon=3, method="bootstrap", simulations=50, random_state=np.random.RandomState(0))  # xpd: legacy-random
    return simulated.variance.shape, resampled.variance.shape, seeded_simulation.variance.values.tolist(), seeded_bootstrap.variance.values.tolist()
