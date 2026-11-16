"""Names that xpd follows from an arch model to a method call, and names it leaves alone.

A rule that reads a method such as forecast needs to know that the object is arch's. It follows
a name that the file assigns from an arch model or result, and it is silent when the same name is
bound in any other way anywhere in the file: by a second assignment from something else, as a
parameter, or as a loop variable. A line that xpd reports ends with a comment that names the
rule. Each case returns what the code computes.
"""

from types import SimpleNamespace

from arch import arch_model
from arch.data import sp500

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def returns(rows=1000):
    return 100 * sp500.load()["Adj Close"].pct_change().dropna().iloc[-rows:]


@case
def followed_through_two_names():
    am = arch_model(returns())
    fitted = am.fit(disp="off")
    return fitted.forecast(horizon=3, reindex=False).variance.values.tolist()  # xpd: reindex-is-default


@case
def assigned_from_something_else():
    shared = arch_model(returns()).fit(disp="off")
    first = shared.forecast(horizon=3, reindex=False).variance.values.tolist()
    shared = SimpleNamespace(forecast=lambda **keywords: sorted(keywords))
    second = shared.forecast(horizon=3, reindex=False)
    return first, second


def forecast_of(passed):
    return passed.forecast(horizon=3, reindex=False)


@case
def also_a_parameter():
    passed = arch_model(returns()).fit(disp="off")
    return forecast_of(passed).variance.values.tolist(), passed.forecast(horizon=3, reindex=False).variance.values.tolist()


@case
def also_a_loop_variable():
    looped = arch_model(returns()).fit(disp="off")
    last = looped.forecast(horizon=3, reindex=False).variance.values.tolist()
    for looped in (SimpleNamespace(forecast=lambda **keywords: sorted(keywords)),):
        last = looped.forecast(horizon=3, reindex=False)
    return last
