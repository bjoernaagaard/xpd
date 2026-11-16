"""Arguments that MAPIE refuses when the object is used, and the arguments it takes.

A line that xpd reports ends with a comment that names the rule. A refused form raises on MAPIE
1.6.0, so it sits in a function that is not a case and that nothing calls. Each case returns what
the code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from mapie.classification import SplitConformalClassifier
from mapie.conditional_conformal_prediction import ConditionalSplitConformalRegressor
from mapie.regression import ConformalizedQuantileRegressor, CrossConformalizedQuantileRegressor, SplitConformalRegressor
from mapie.risk_control import BinaryClassificationController, MultiLabelClassificationController
from sklearn.ensemble import GradientBoostingRegressor, HistGradientBoostingRegressor, RandomForestRegressor
from sklearn.linear_model import LinearRegression, LogisticRegression, QuantileRegressor
from sklearn.multioutput import MultiOutputClassifier
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import StandardScaler

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def table(n=600, seed=0):
    r = np.random.default_rng(seed)
    X = r.normal(size=(n, 3))
    y = X[:, 0] + r.normal(size=n)
    return X, y


def label_table(n=600, seed=0):
    X, y = table(n, seed)
    first = (y > 0).astype(int)
    return X, np.column_stack([first, 1 - first, (X[:, 1] > 0).astype(int)])


def intercept_and_first(X):
    return np.column_stack([np.ones(len(X)), X[:, 0]])


def estimator_that_is_not_fitted():
    X, y = table()
    new = SplitConformalRegressor(LinearRegression(), confidence_level=0.9)  # xpd: prefit-estimator-not-fitted
    named = SplitConformalClassifier(estimator=LogisticRegression(), prefit=True)  # xpd: prefit-estimator-not-fitted
    piped = SplitConformalRegressor(make_pipeline(StandardScaler(), LinearRegression()))  # xpd: prefit-estimator-not-fitted
    model = LinearRegression()
    held = SplitConformalRegressor(model, confidence_level=0.9)
    held.fit(X[:300], y[:300])  # xpd: prefit-estimator-not-fitted
    chained = SplitConformalRegressor(model).fit(X[:300], y[:300])  # xpd: prefit-estimator-not-fitted
    one = ConformalizedQuantileRegressor(QuantileRegressor().fit(X[:300], y[:300]), prefit=True)  # xpd: prefit-estimator-not-fitted
    conditional = ConditionalSplitConformalRegressor(feature_map=intercept_and_first, estimator=LinearRegression())  # xpd: prefit-estimator-not-fitted
    return new, named, piped, held, chained, one, conditional


@case
def estimator_fitted_before_or_by_mapie():
    X, y = table()
    model = LinearRegression().fit(X[:300], y[:300])
    prefit = SplitConformalRegressor(model, confidence_level=0.9).conformalize(X[300:450], y[300:450])
    inline = SplitConformalRegressor(LinearRegression().fit(X[:300], y[:300])).conformalize(X[300:450], y[300:450])
    learner = SplitConformalRegressor(LinearRegression(), confidence_level=0.9, prefit=False)
    learner.fit(X[:300], y[:300])
    learner.conformalize(X[300:450], y[300:450])
    quantiles = [QuantileRegressor(quantile=q, alpha=0.0).fit(X[:300], y[:300]) for q in (0.05, 0.95, 0.5)]
    three = ConformalizedQuantileRegressor(quantiles, confidence_level=0.9, prefit=True).conformalize(X[300:450], y[300:450])
    conditional = ConditionalSplitConformalRegressor(intercept_and_first, model, confidence_level=0.9)
    return [m.predict_interval(X[450:])[1] for m in (prefit, inline, learner, three)], type(conditional).__name__


def estimator_without_a_quantile_loss():
    plain = ConformalizedQuantileRegressor(GradientBoostingRegressor(random_state=0))  # xpd: quantile-estimator-loss
    squared = ConformalizedQuantileRegressor(estimator=HistGradientBoostingRegressor(loss="squared_error", random_state=0))  # xpd: quantile-estimator-loss
    forest = ConformalizedQuantileRegressor(RandomForestRegressor(random_state=0), confidence_level=0.9)  # xpd: quantile-estimator-loss
    cross = CrossConformalizedQuantileRegressor(LinearRegression(), random_state=0)  # xpd: quantile-estimator-loss
    return plain, squared, forest, cross


@case
def estimator_with_a_quantile_loss():
    X, y = table()
    boosted = ConformalizedQuantileRegressor(GradientBoostingRegressor(loss="quantile", n_estimators=20, random_state=0), confidence_level=0.9)
    boosted.fit(X[:300], y[:300]).conformalize(X[300:450], y[300:450])
    linear = ConformalizedQuantileRegressor(QuantileRegressor(alpha=0.0), confidence_level=0.9)
    linear.fit(X[:300], y[:300]).conformalize(X[300:450], y[300:450])
    point = SplitConformalRegressor(GradientBoostingRegressor(n_estimators=20, random_state=0), prefit=False)
    return boosted.predict_interval(X[450:])[1], linear.predict_interval(X[450:])[1], point


def controller_given_labels():
    X, y = table()
    model = LogisticRegression().fit(X[:300], y[:300] > 0)
    first = BinaryClassificationController(model.predict, "precision", target_level=0.7)  # xpd: predict-function-returns-labels
    named = BinaryClassificationController(predict_function=model.predict, risk="recall", target_level=0.7)  # xpd: predict-function-returns-labels
    return first, named


@case
def controller_given_probabilities():
    X, y = table()
    model = LogisticRegression().fit(X[:300], y[:300] > 0)
    controller = BinaryClassificationController(model.predict_proba, "recall", target_level=0.7, confidence_level=0.9)
    controller.calibrate(X[300:450], y[300:450] > 0)
    return controller.best_predict_param, model.predict(X[450:])


def method_that_does_not_fit_the_risk():
    X, Y = label_table()
    model = MultiOutputClassifier(LogisticRegression()).fit(X[:300], Y[:300])
    precise = MultiLabelClassificationController(model.predict_proba, risk="precision", method="crc")  # xpd: risk-control-method
    tested = MultiLabelClassificationController(model.predict_proba, method="ltt", confidence_level=0.9)  # xpd: risk-control-method
    bounded = MultiLabelClassificationController(model.predict_proba, method="rcps", target_level=0.8)  # xpd: risk-control-method
    silent = MultiLabelClassificationController(model.predict_proba, risk="precision", target_level=0.8)  # xpd: risk-control-method
    return precise, tested, bounded, silent


@case
def method_that_fits_the_risk():
    X, Y = label_table()
    model = MultiOutputClassifier(LogisticRegression()).fit(X[:300], Y[:300])
    ignored = MultiLabelClassificationController(model.predict_proba, method="crc", target_level=0.8, confidence_level=0.9)  # xpd: risk-control-method
    ignored.calibrate(X[300:450], Y[300:450])
    expected = MultiLabelClassificationController(model.predict_proba, method="crc", target_level=0.8)
    expected.calibrate(X[300:450], Y[300:450])
    bounded = MultiLabelClassificationController(model.predict_proba, method="rcps", target_level=0.8, confidence_level=0.9)
    bounded.calibrate(X[300:450], Y[300:450])
    precise = MultiLabelClassificationController(model.predict_proba, risk="precision", target_level=0.6, confidence_level=0.9)
    return ignored.predict(X[450:]), expected.predict(X[450:]), bounded.predict(X[450:]), precise
