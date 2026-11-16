"""The arch API that was removed by 9.0.0, and the forms that replace it.

A line that xpd reports ends with a comment that names the rule. A removed form raises on arch
9.0.0, so it sits in a function that is not a case and that nothing calls. Each case returns what
the code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from arch import arch_model
from arch.bootstrap import MCS, SPA, IIDBootstrap, StationaryBootstrap
from arch.data import sp500
from arch.univariate import Normal, StudentsT
from arch.utility.cov import cov_nw

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def returns(rows=1000):
    return 100 * sp500.load()["Adj Close"].pct_change().dropna().iloc[-rows:]


def draws(n=300, seed=0):
    return np.random.default_rng(seed).standard_normal(n)


def removed_future_import():
    from arch.__future__ import reindexing  # xpd: removed-in-arch
    import arch.__future__.reindexing  # xpd: removed-in-arch

    return reindexing, arch


def kept_for_old_releases():
    try:
        from arch.__future__ import reindexing
    except ImportError:
        reindexing = None
    return reindexing


def removed_plot_keyword(other):
    res = arch_model(returns()).fit(disp="off")
    first = res.hedgehog_plot(type="mean")  # xpd: removed-in-arch
    second = other.hedgehog_plot(type="mean")  # xpd: removed-in-arch
    third = res.hedgehog_plot(plot_type="mean")
    fourth = other.plot(type="mean")
    return first, second, third, fourth


def removed_random_state():
    rs = np.random.RandomState(0)  # xpd: legacy-random
    bs = IIDBootstrap(draws(), random_state=rs)  # xpd: removed-in-arch
    block = StationaryBootstrap(10, draws(), random_state=rs)  # xpd: removed-in-arch
    dist = Normal(random_state=rs)  # xpd: removed-in-arch
    tail = StudentsT(rs)  # xpd: removed-in-arch
    return bs, block, dist, tail


def removed_seed_methods():
    bs = IIDBootstrap(draws())
    bs.seed(1234)  # xpd: removed-in-arch
    state = bs.get_state()  # xpd: removed-in-arch
    bs.set_state(state)  # xpd: removed-in-arch
    generator = bs.random_state  # xpd: removed-in-arch
    losses = draws(900).reshape(300, 3) ** 2
    spa = SPA(losses[:, 0], losses[:, 1:], reps=100)
    spa.seed(1234)  # xpd: removed-in-arch
    return generator, spa


def removed_cov_nw_export():
    from arch.utility import cov_nw as newey_west  # xpd: removed-in-arch
    from arch.utility import cov_nw  # xpd: removed-in-arch

    return cov_nw, newey_west


def read_only_attributes():
    block = StationaryBootstrap(10, draws(), seed=0)
    block.block_size = 20  # xpd: removed-in-arch
    losses = draws(900).reshape(300, 3) ** 2
    mcs = MCS(losses, 0.1, reps=100, seed=0)
    mcs.bootstrap = block  # xpd: removed-in-arch
    return block, mcs


def removed_argument_pair():
    losses = draws(900).reshape(300, 3) ** 2
    first = SPA(losses[:, 0], losses[:, 1:], reps=100, studentize=False, nested=True, seed=0)  # xpd: removed-in-arch
    second = SPA(losses[:, 0], losses[:, 1:], reps=100, nested=True, seed=0)
    return first, second


@case
def seed_keyword():
    rs = np.random.RandomState(0)  # xpd: legacy-random
    bs = IIDBootstrap(draws(), seed=rs)
    block = StationaryBootstrap(10, draws(), seed=0)
    dist = Normal(seed=rs)
    state = bs.state
    bs.state = state
    return bs.apply(np.mean, 5).ravel().tolist(), block.apply(np.mean, 5).ravel().tolist(), float(dist.simulate([])(1)[0]), type(bs.generator).__name__


@case
def other_objects_keep_their_names(rng=None):
    # A NumPy generator and a scikit-learn splitter have a seed or a random_state of their own.
    from numpy.random import RandomState  # xpd: legacy-random

    rng = rng or RandomState(0)
    rng.seed(1)
    state = rng.get_state()
    rng.set_state(state)
    return float(rng.standard_normal())


@case
def cov_nw_from_its_module():
    return float(cov_nw(draws(), lags=3))
