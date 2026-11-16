"""Forecasts that raise, forecasts computed many times over, and arguments that do nothing.

A line that xpd reports ends with a comment that names the rule. A form that raises on arch 9.0.0
sits in a function that is not a case and that nothing calls. Each case returns what the code
computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import contextlib
import io

import numpy as np
from arch import arch_model
from arch.data import sp500
from arch.univariate import GARCH

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def returns(rows=1500):
    return 100 * sp500.load()["Adj Close"].pct_change().dropna().iloc[-rows:]


def no_analytic_forecast(horizon, method):
    y = returns()
    egarch = arch_model(y, vol="EGARCH", p=1, o=1, q=1)
    egarch_res = egarch.fit(disp="off")
    first = egarch_res.forecast(horizon=5)  # xpd: analytic-forecast-horizon
    second = arch_model(y, vol="APARCH").fit(disp="off").forecast(horizon=10, method="analytic")  # xpd: analytic-forecast-horizon
    tarch = arch_model(y, p=1, o=1, q=1, power=1.0).fit(disp="off")
    third = tarch.forecast(horizon=5)  # xpd: analytic-forecast-horizon
    fourth = egarch_res.forecast(horizon=horizon)
    fifth = egarch_res.forecast(horizon=5, method=method)
    # hedgehog_plot forecasts 10 steps with the analytic method unless the call says otherwise.
    sixth = egarch_res.hedgehog_plot()  # xpd: analytic-forecast-horizon
    seventh = egarch_res.hedgehog_plot(horizon=1)
    eighth = egarch_res.hedgehog_plot(method="simulation", simulations=50)
    return first, second, third, fourth, fifth, sixth, seventh, eighth


@case
def forecasts_these_models_have():
    y = returns()
    fitted = arch_model(y, vol="EGARCH", p=1, o=1, q=1).fit(disp="off")
    one_step = fitted.forecast(horizon=1)
    simulated = fitted.forecast(horizon=5, method="simulation", simulations=50, rng=np.random.default_rng(0).standard_normal)
    gjr = arch_model(y, p=1, o=1, q=1).fit(disp="off")
    analytic = gjr.forecast(horizon=5)
    # power is not read with vol="arch", so this model has an analytic forecast. arch-ignored-argument reports the power.
    arch_only = arch_model(y, vol="ARCH", p=2, power=1.0).fit(disp="off")  # xpd: arch-ignored-argument
    also_analytic = arch_only.forecast(horizon=5)
    # The file gives this model another volatility process than the call wrote.
    replaced = arch_model(y, vol="EGARCH")
    replaced.volatility = GARCH(1, 0, 1)
    replaced_res = replaced.fit(disp="off")
    from_garch = replaced_res.forecast(horizon=5)
    values = [f.variance.values.tolist() for f in (one_step, analytic, also_analytic, from_garch)]
    return values, simulated.variance.shape


@case
def one_forecast_per_origin():
    y = returns()
    full = arch_model(y).fit(disp="off")
    rows = []
    for origin in range(len(y) - 20, len(y)):
        rows.append(full.forecast(horizon=3, start=origin).variance.iloc[0].tolist())  # xpd: forecast-per-origin
    listed = [full.forecast(horizon=3, start=i).variance.iloc[0].tolist() for i in range(len(y) - 5, len(y))]  # xpd: forecast-per-origin
    at_once = full.forecast(horizon=3, start=len(y) - 20).variance.values.tolist()
    chosen = [full.forecast(horizon=3, start=origin).variance.iloc[0].tolist() for origin in (0, 700, len(y) - 1)]
    return rows, listed, at_once, chosen


@case
def rolling_estimate():
    y = returns()
    am = arch_model(y)
    rows = []
    for origin in range(1400, 1405):
        window_res = am.fit(disp="off", last_obs=origin)
        rows.append(window_res.forecast(horizon=3, start=origin).variance.iloc[0].tolist())
    return rows


@case
def one_result_per_origin():
    y = returns()
    am = arch_model(y)
    # Each origin has its own result, so one call would not return these rows.
    fits = {origin: am.fit(disp="off", last_obs=origin) for origin in range(1400, 1403)}
    rows = []
    for origin in range(1400, 1403):
        rows.append(fits[origin].forecast(horizon=3, start=origin).variance.iloc[0].tolist())
    return rows


@case
def reindex_argument():
    y = returns()
    am = arch_model(y)
    res = am.fit(disp="off")
    short = res.forecast(horizon=3, reindex=False)  # xpd: reindex-is-default
    first_argument = res.forecast(reindex=False, horizon=3)  # xpd: reindex-is-default
    by_model = am.forecast(res.params, horizon=3, reindex=False)  # xpd: reindex-is-default
    long = res.forecast(horizon=3, reindex=True)
    plain = res.forecast(horizon=3)
    return short.variance.values.tolist(), first_argument.variance.values.tolist(), by_model.variance.values.tolist(), long.variance.shape, plain.variance.values.tolist()


@case
def other_objects_with_reindex(frame=None):
    # reindex is also a pandas method and may be a keyword of another library's forecast.
    frame = returns().to_frame("r") if frame is None else frame
    return frame.reindex(frame.index[:5]).shape


@case
def fit_inside_a_loop():
    y = returns()
    am = arch_model(y)
    printed = io.StringIO()
    with contextlib.redirect_stdout(printed):
        noisy = [am.fit(last_obs=1400 + i).params.tolist() for i in range(3)]  # xpd: fit-output-in-loop
        for window in range(3):
            noisy.append(arch_model(y.iloc[window : 1400 + window]).fit(update_freq=0).params.tolist())  # xpd: fit-output-in-loop
    quiet = []
    for window in range(3):
        quiet.append(am.fit(disp="off", last_obs=1400 + window).params.tolist())
    once = am.fit(disp="off").params.tolist()
    return noisy, quiet, once, len(printed.getvalue().splitlines())
