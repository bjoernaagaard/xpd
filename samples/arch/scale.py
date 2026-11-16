"""Returns that reach arch as decimals, and results that arch rescaled.

A line that xpd reports ends with a comment that names the rule. Each case returns what the code
computes, so a copy fixed with `xpd check --fix` can be compared with this file. The cases that
fit decimals make arch warn, which is the point of them.
"""

import warnings

import numpy as np
from arch import arch_model
from arch.data import sp500
from arch.univariate import ConstantMean

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def prices(rows=1500):
    return sp500.load()["Adj Close"].iloc[-rows:]


def to_percent(series):
    return series * 100


def fitted(model):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        res = model.fit(disp="off")
    return res.convergence_flag, res.params.tolist()


@case
def decimals_in_the_call():
    close = prices()
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        simple = arch_model(close.pct_change().dropna())  # xpd: unscaled-returns
        logs = arch_model(np.log(close).diff().dropna(), vol="GARCH")  # xpd: unscaled-returns
        ratio = ConstantMean(np.log(close / close.shift(1)).dropna())  # xpd: unscaled-returns
        keyword = arch_model(y=close.pct_change().dropna())  # xpd: unscaled-returns
    return [fitted(m) for m in (simple, logs, ratio, keyword)]


@case
def decimals_through_a_name():
    frame = prices().to_frame("close")
    rets = frame["close"].pct_change().dropna()
    frame["ret"] = np.log(frame["close"]).diff()
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        by_name = arch_model(rets)  # xpd: unscaled-returns
        cleaned = arch_model(rets.dropna(), dist="t")  # xpd: unscaled-returns
        by_column = arch_model(frame["ret"].dropna())  # xpd: unscaled-returns
    return [fitted(m) for m in (by_name, cleaned, by_column)]


@case
def percent_returns():
    close = prices()
    percent = 100 * close.pct_change().dropna()
    in_call = arch_model(close.pct_change().dropna() * 100)
    by_method = arch_model(close.pct_change().mul(100).dropna())
    by_name = arch_model(percent)
    scaled = close.pct_change().dropna()
    scaled = scaled * 100
    reassigned = arch_model(scaled)
    grown = close.pct_change().dropna()
    grown *= 100
    in_place = arch_model(grown)
    # Returns over their standard deviation have a variance of 1, and a helper may convert them.
    standardized = close.pct_change().dropna() / close.pct_change().std()
    unit_variance = arch_model(standardized)
    converted = to_percent(close.pct_change().dropna())
    through_helper = arch_model(converted)
    return [fitted(m) for m in (in_call, by_method, by_name, reassigned, in_place, unit_variance, through_helper)]


@case
def scale_stated_in_the_call():
    decimals = prices().pct_change().dropna()
    as_given = arch_model(decimals, rescale=False)
    return fitted(as_given)
