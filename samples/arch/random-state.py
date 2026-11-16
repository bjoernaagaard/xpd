"""arch objects that draw random numbers without a seed, and the same objects with one.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from arch import arch_model
from arch.bootstrap import MCS, SPA, CircularBlockBootstrap, IIDBootstrap, MovingBlockBootstrap, StationaryBootstrap, StepM
from arch.data import sp500

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def returns(rows=1000):
    return 100 * sp500.load()["Adj Close"].pct_change().dropna().iloc[-rows:]


def draws(n=300, seed=0):
    return np.random.default_rng(seed).standard_normal(n)


def losses(seed=0):
    return np.random.default_rng(seed).standard_normal((300, 4)) ** 2


@case
def bootstraps_without_a_seed():
    np.random.seed(0)  # xpd: legacy-random
    x = draws()
    iid = IIDBootstrap(x)  # xpd: arch-unseeded-draws
    stationary = StationaryBootstrap(10, x)  # xpd: arch-unseeded-draws
    circular = CircularBlockBootstrap(10, x)  # xpd: arch-unseeded-draws
    moving = MovingBlockBootstrap(10, x)  # xpd: arch-unseeded-draws
    return [bs.conf_int(np.mean, 100).shape for bs in (iid, stationary, circular, moving)]


@case
def tests_without_a_seed():
    loss = losses()
    spa = SPA(loss[:, 0], loss[:, 1:], reps=100)  # xpd: arch-unseeded-draws
    stepm = StepM(loss[:, 0], loss[:, 1:], reps=100)  # xpd: arch-unseeded-draws
    mcs = MCS(loss, 0.1, reps=100)  # xpd: arch-unseeded-draws
    spa.compute()
    stepm.compute()
    mcs.compute()
    return spa.pvalues.shape, len(stepm.superior_models), mcs.pvalues.shape


@case
def forecasts_without_a_seed():
    res = arch_model(returns()).fit(disp="off")
    simulated = res.forecast(horizon=3, method="simulation", simulations=50)  # xpd: arch-unseeded-draws
    resampled = res.forecast(horizon=3, method="bootstrap", simulations=50)  # xpd: arch-unseeded-draws
    analytic = res.forecast(horizon=3)
    return simulated.variance.shape, resampled.variance.shape, analytic.variance.values.tolist()


@case
def simulated_series_without_a_seed():
    am = arch_model(None)
    path = am.simulate([0.0, 0.1, 0.1, 0.8], 100)  # xpd: arch-unseeded-draws
    return path.shape


@case
def with_a_seed(seed=0):
    x = draws()
    loss = losses()
    iid = IIDBootstrap(x, seed=seed)
    stationary = StationaryBootstrap(10, x, seed=np.random.default_rng(0))
    # A bootstrap that is given the state of a seeded one draws the same samples.
    restored = StationaryBootstrap(10, x)
    restored.state = stationary.state
    spa = SPA(loss[:, 0], loss[:, 1:], reps=100, seed=0)
    spa.compute()
    mcs = MCS(loss, 0.1, reps=100, seed=0)
    mcs.compute()
    res = arch_model(returns()).fit(disp="off")
    simulated = res.forecast(horizon=3, method="simulation", simulations=50, rng=np.random.default_rng(0).standard_normal)
    resampled = res.forecast(horizon=3, method="bootstrap", simulations=50, random_state=np.random.RandomState(0))  # xpd: legacy-random
    return (
        iid.conf_int(np.mean, 100).ravel().tolist(),
        stationary.conf_int(np.mean, 100).ravel().tolist(),
        restored.conf_int(np.mean, 100).ravel().tolist(),
        spa.pvalues.tolist(),
        mcs.pvalues.values.ravel().tolist(),
        simulated.variance.values.tolist(),
        resampled.variance.values.tolist(),
    )
