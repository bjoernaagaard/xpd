"""A series split at random and the 0.x methods of the time series class, and the forms that keep time order.

A line that xpd reports ends with a comment that names the rule. A removed form raises on MAPIE
1.6.0, so it sits in a function that is not a case and that nothing calls. Each case returns what
the code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from mapie.metrics.regression import regression_coverage_score
from mapie.regression import TimeSeriesRegressor
from mapie.subsample import BlockBootstrap
from mapie.utils import train_conformalize_test_split
from sklearn.ensemble import RandomForestRegressor
from sklearn.model_selection import train_test_split

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def random_walk(n=600, seed=0):
    r = np.random.default_rng(seed)
    t = np.arange(n)
    y = np.cumsum(r.normal(size=n))
    X = np.column_stack([t, np.sin(t / 20)])
    return X, y


def fitted(X, y, method="enbpi"):
    blocks = BlockBootstrap(n_resamplings=10, n_blocks=5, overlapping=False, random_state=0)
    series = TimeSeriesRegressor(RandomForestRegressor(n_estimators=10, random_state=0), method=method, cv=blocks, random_state=0)
    series.fit(X, y)
    return series


@case
def shuffled_split():
    X, y = random_walk()
    X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.3, random_state=0)  # xpd: shuffled-time-series-split
    series = TimeSeriesRegressor(RandomForestRegressor(n_estimators=10, random_state=0), random_state=0)
    series.fit(X_train, y_train)
    _, y_intervals = series.predict(X_test, confidence_level=0.9, ensemble=True)
    return float(regression_coverage_score(y_test, y_intervals)[0])


@case
def shuffled_three_parts():
    X, y = random_walk()
    X_past, X_conf, X_new, y_past, y_conf, y_new = train_conformalize_test_split(X, y, train_size=0.6, conformalize_size=0.2, test_size=0.2, random_state=0)  # xpd: shuffled-time-series-split
    blocks = BlockBootstrap(n_resamplings=10, n_blocks=5, overlapping=False, random_state=0)
    series = TimeSeriesRegressor(RandomForestRegressor(n_estimators=10, random_state=0), cv=blocks, random_state=0)
    series.fit(X_past, y_past)
    _, y_intervals = series.predict(X_new, confidence_level=0.9, ensemble=True)
    X_a, X_b, y_a, y_b = train_test_split(X_conf, y_conf, test_size=0.5, random_state=0)
    return float(regression_coverage_score(y_new, y_intervals)[0]), len(X_a), len(X_b), len(y_a), len(y_b)


@case
def split_in_time_order():
    X, y = random_walk()
    X_early, X_late, y_early, y_late = train_test_split(X, y, test_size=0.3, shuffle=False)
    series = TimeSeriesRegressor(RandomForestRegressor(n_estimators=10, random_state=0), random_state=0)
    series.fit(X_early, y_early)
    _, y_intervals = series.predict(X_late, confidence_level=0.9, ensemble=True)
    parts = train_conformalize_test_split(X, y, train_size=0.6, conformalize_size=0.2, test_size=0.2, shuffle=False)
    return float(regression_coverage_score(y_late, y_intervals)[0]), [len(part) for part in parts]


def removed_time_series_methods():
    X, y = random_walk()
    series = fitted(X[:400], y[:400])
    series.partial_fit(X[400:450], y[400:450])  # xpd: removed-in-mapie
    _, y_intervals = series.predict(X[450:], alpha=0.1, ensemble=True)  # xpd: removed-in-mapie
    weighted = TimeSeriesRegressor(RandomForestRegressor(n_estimators=10, random_state=0), random_state=0)
    weighted.fit(X[:400], y[:400], sample_weight=np.ones(400))  # xpd: removed-in-mapie
    return y_intervals, weighted


@case
def update_that_adapts_nothing():
    X, y = random_walk()
    series = fitted(X[:400], y[:400], method="aci")
    series.update(X[400:450], y[400:450], gamma=0.05)  # xpd: deprecated-in-mapie
    series.update(X[450:500], y[450:500], confidence_level=0.9, optimize_beta=True)  # xpd: deprecated-in-mapie, deprecated-in-mapie
    _, y_intervals = series.predict(X[500:], confidence_level=0.9, ensemble=True)
    return y_intervals


@case
def update_and_adapt():
    X, y = random_walk()
    series = fitted(X[:400], y[:400], method="aci")
    series.update(X[400:450], y[400:450])
    series.adapt_conformal_inference(X[400:450], y[400:450], gamma=0.05)
    settings = {"cv": 5}
    settings.update(gamma=0.05)
    _, y_intervals = series.predict(X[450:], confidence_level=1 - 0.1, ensemble=True)
    weighted = TimeSeriesRegressor(RandomForestRegressor(n_estimators=10, random_state=0), random_state=0)
    weighted.fit(X[:400], y[:400], fit_params={"sample_weight": np.ones(400)})
    return y_intervals, settings, weighted.predict(X[450:])
