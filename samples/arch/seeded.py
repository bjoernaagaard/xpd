"""Simulated forecasts and series from a model whose distribution has a seed.

xpd reports no line of this file. The rule reads the whole file: a seeded distribution anywhere
in it makes arch-unseeded-draws leave simulate and forecast with method="simulation" alone,
because a pattern cannot see which model gets the distribution. samples/arch/random-state.py
holds the same calls without a seed. Each case returns what the code computes.
"""

from arch import arch_model
from arch.data import sp500
from arch.univariate import Normal

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def returns(rows=1000):
    return 100 * sp500.load()["Adj Close"].pct_change().dropna().iloc[-rows:]


@case
def simulation_from_a_seeded_distribution():
    am = arch_model(returns())
    am.distribution = Normal(seed=0)
    res = am.fit(disp="off")
    simulated = res.forecast(horizon=3, method="simulation", simulations=50)
    sim = arch_model(None)
    sim.distribution = Normal(seed=0)
    path = sim.simulate([0.0, 0.1, 0.1, 0.8], 100)
    return simulated.variance.values.tolist(), path["data"].tolist()
