"""A model that arch rescaled, in a file that reads the scale and divides by it.

xpd reports no line of this file. samples/arch/rescale.py holds the same model without the scale.
Each case returns what the code computes.
"""

from arch import arch_model
from arch.data import sp500

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def decimals(rows=1500):
    return sp500.load()["Adj Close"].pct_change().dropna().iloc[-rows:]


@case
def forecast_in_the_units_of_the_data():
    res = arch_model(decimals(), rescale=True).fit(disp="off")
    variance = res.forecast(horizon=1).variance.iloc[-1, 0] / res.scale**2
    volatility = res.conditional_volatility.iloc[-1] / res.scale
    return float(res.scale), float(variance), float(volatility)
