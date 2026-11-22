"""Multiple comparison tests read before compute, and a kernel bandwidth passed back.

A line that xpd reports ends with a comment that names the rule. A form that raises on arch 9.0.0
sits in a function that is not a case and that nothing calls. Each case returns what the code
computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from arch.bootstrap import MCS, SPA, StepM
from arch.covariance.kernel import Bartlett, Parzen, QuadraticSpectral
from arch.data import sp500

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def losses(seed=0):
    return np.random.default_rng(seed).standard_normal((300, 4)) ** 2


def returns(rows=1500):
    return 100 * sp500.load()["Adj Close"].pct_change().dropna().iloc[-rows:]


def read_before_compute(other):
    loss = losses()
    never = SPA(loss[:, 0], loss[:, 1:], reps=100, seed=0)
    pvalues = never.pvalues  # xpd: results-before-compute
    better = never.better_models()  # xpd: results-before-compute
    critical = never.critical_values(0.05)  # xpd: results-before-compute
    chained = MCS(loss, 0.1, reps=100, seed=0).included  # xpd: results-before-compute
    forgotten = StepM(loss[:, 0], loss[:, 1:], reps=100, seed=0)
    superior = forgotten.superior_models  # xpd: results-before-compute
    # An object that the file does not build from an arch class is left alone.
    elsewhere = other.pvalues
    return pvalues, better, critical, chained, superior, elsewhere


@case
def read_after_compute():
    loss = losses()
    spa = SPA(loss[:, 0], loss[:, 1:], reps=100, seed=0)
    spa.compute()
    mcs = MCS(loss, 0.1, reps=100, seed=0)
    mcs.compute()
    stepm = StepM(loss[:, 0], loss[:, 1:], reps=100, seed=0)
    stepm.compute()
    return spa.pvalues.tolist(), spa.critical_values(0.05).tolist(), mcs.included, mcs.excluded, list(stepm.superior_models)


@case
def compute_through_another_name():
    loss = losses()
    looped = SPA(loss[:, 0], loss[:, 1:], reps=100, seed=0)
    kept = MCS(loss, 0.1, reps=100, seed=0)
    # The loop and the helper call compute, and neither does it on the name that is read.
    for test in (looped, kept):
        test.compute()
    helped = computed(StepM(loss[:, 0], loss[:, 1:], reps=100, seed=0))
    passed = StepM(loss[:, 0], loss[:, 1:], reps=100, seed=0)
    computed(passed)
    return looped.pvalues.tolist(), kept.included, list(helped.superior_models), list(passed.superior_models)


def computed(test):
    test.compute()
    return test


@case
def bandwidth_passed_back():
    y = returns()
    first = Bartlett(y)
    again = Bartlett(y, bandwidth=first.opt_bandwidth)  # xpd: opt-bandwidth-as-bandwidth
    parzen = Parzen(y)
    parzen_again = Parzen(y, bandwidth=parzen.opt_bandwidth)  # xpd: opt-bandwidth-as-bandwidth
    same = Bartlett(y, bandwidth=first.bandwidth)
    spectral = QuadraticSpectral(y)
    spectral_again = QuadraticSpectral(y, bandwidth=spectral.opt_bandwidth)
    return [float(np.asarray(e.cov.long_run).ravel()[0]) for e in (first, again, parzen, parzen_again, same, spectral, spectral_again)]  # xpd: float-around-number
