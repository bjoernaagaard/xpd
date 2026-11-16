"""A model that arch rescaled, in a file that never reads the scale.

A line that xpd reports ends with a comment that names the rule. The rule reads the whole file:
it reports rescale=True only when no line reads an attribute named scale, so the file that reads
it is samples/arch/rescale-read.py. Each case returns what the code computes.
"""

from arch import arch_model
from arch.data import sp500
from arch.univariate import ConstantMean

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def decimals(rows=1500):
    return sp500.load()["Adj Close"].pct_change().dropna().iloc[-rows:]


@case
def forecast_in_rescaled_units(rescale=True):
    res = arch_model(decimals(), rescale=True).fit(disp="off")  # xpd: rescaled-units
    by_class = ConstantMean(decimals(), rescale=True).fit(disp="off")  # xpd: rescaled-units
    from_variable = arch_model(decimals(), rescale=rescale).fit(disp="off")
    variance = res.forecast(horizon=1).variance.iloc[-1, 0]
    return float(variance), float(res.conditional_volatility.iloc[-1]), by_class.params.tolist(), from_variable.params.tolist()
