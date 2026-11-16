"""Forms that run the model again for a result MAPIE already has, and the forms that run it once.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from mapie.classification import SplitConformalClassifier
from mapie.regression import ConformalizedQuantileRegressor, CrossConformalRegressor, SplitConformalRegressor
from sklearn.ensemble import RandomForestClassifier, RandomForestRegressor
from sklearn.linear_model import QuantileRegressor

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def table(n=600, seed=0):
    r = np.random.default_rng(seed)
    X = r.normal(size=(n, 4))
    y = X[:, 0] + r.normal(size=n)
    return X, y


def conformal_regressor(X, y):
    model = RandomForestRegressor(n_estimators=20, random_state=0).fit(X[:300], y[:300])
    return SplitConformalRegressor(model, confidence_level=0.9).conformalize(X[300:450], y[300:450])


@case
def predict_then_predict_interval():
    X, y = table()
    conformal = conformal_regressor(X, y)
    X_new = X[450:]
    y_pred = conformal.predict(X_new)  # xpd: predict-and-predict-interval
    _, y_intervals = conformal.predict_interval(X_new)
    return y_pred, y_intervals


@case
def predict_interval_once():
    X, y = table()
    conformal = conformal_regressor(X, y)
    X_new, X_other = X[450:], X[:50]
    y_pred, y_intervals = conformal.predict_interval(X_new)
    other = conformal.predict(X_other)
    return y_pred, y_intervals, other


@case
def predict_then_predict_set():
    X, y = table()
    labels = np.digitize(y, [-0.5, 0.5])
    model = RandomForestClassifier(n_estimators=20, random_state=0).fit(X[:300], labels[:300])
    conformal = SplitConformalClassifier(model, confidence_level=0.9).conformalize(X[300:450], labels[300:450])
    X_new = X[450:]
    _, y_sets = conformal.predict_set(X_new)
    y_pred = conformal.predict(X_new)  # xpd: predict-and-predict-interval
    return y_pred, y_sets


@case
def one_object_per_level():
    X, y = table()
    intervals = {}
    for level in (0.8, 0.9, 0.95):
        cross = CrossConformalRegressor(RandomForestRegressor(n_estimators=10, random_state=0), confidence_level=level, cv=5, random_state=0)  # xpd: refit-per-confidence-level
        cross.fit_conformalize(X[:450], y[:450])
        intervals[level] = cross.predict_interval(X[450:])[1][:, :, 0]
    model = RandomForestRegressor(n_estimators=10, random_state=0).fit(X[:300], y[:300])
    built = [SplitConformalRegressor(model, confidence_level=level) for level in (0.8, 0.9)]  # xpd: refit-per-confidence-level
    return intervals, built


@case
def one_object_for_all_levels():
    X, y = table()
    levels = [0.8, 0.9, 0.95]
    cross = CrossConformalRegressor(RandomForestRegressor(n_estimators=10, random_state=0), confidence_level=levels, cv=5, random_state=0)
    cross.fit_conformalize(X[:450], y[:450])
    _, y_intervals = cross.predict_interval(X[450:])
    quantile = {}
    for level in (0.8, 0.9):
        quantile[level] = ConformalizedQuantileRegressor(QuantileRegressor(alpha=0.0), confidence_level=level)
    by_depth = {}
    for depth in (2, 4):
        by_depth[depth] = SplitConformalRegressor(RandomForestRegressor(max_depth=depth, random_state=0), confidence_level=0.9, prefit=False)
    return {level: y_intervals[:, :, i] for i, level in enumerate(levels)}, quantile, by_depth
