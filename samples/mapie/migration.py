"""The MAPIE 0.x API, which raises on MAPIE 1, and the forms that replace it.

A line that xpd reports ends with a comment that names the rule. A removed form raises on MAPIE
1.6.0, so it sits in a function that is not a case and that nothing calls. Each case returns what
the code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from mapie.classification import SplitConformalClassifier
from mapie.metrics.regression import coverage_width_based, regression_coverage_score, regression_mean_width_score, regression_mwi_score
from mapie.regression import CrossConformalRegressor, JackknifeAfterBootstrapRegressor, SplitConformalRegressor
from mapie.risk_control import BinaryClassificationController, BinaryClassificationRisk, BinaryRisk  # xpd: deprecated-in-mapie
from mapie.utils import train_conformalize_test_split
from sklearn.linear_model import LinearRegression, LogisticRegression

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def table(n=600, seed=0):
    r = np.random.default_rng(seed)
    X = r.normal(size=(n, 3))
    y = X[:, 0] + r.normal(size=n)
    return X, y


def three_parts(seed=0):
    X, y = table(seed=seed)
    return train_conformalize_test_split(X, y, train_size=0.5, conformalize_size=0.25, test_size=0.25, random_state=seed)


def removed_classes():
    from mapie.regression import MapieRegressor  # xpd: removed-in-mapie
    from mapie.classification import MapieClassifier  # xpd: removed-in-mapie
    from mapie.regression import MapieQuantileRegressor, MapieTimeSeriesRegressor  # xpd: removed-in-mapie, removed-in-mapie
    from mapie.quantile_regression import MapieQuantileRegressor as OldQuantile  # xpd: removed-in-mapie
    from mapie.calibration import MapieCalibrator  # xpd: removed-in-mapie
    from mapie.multi_label_classification import MapieMultiLabelClassifier  # xpd: removed-in-mapie
    from mapie.risk_control import PrecisionRecallController  # xpd: removed-in-mapie
    from mapie.mondrian import MondrianCP  # xpd: removed-in-mapie
    from mapie.conformity_scores import ConformityScore  # xpd: removed-in-mapie

    return MapieRegressor, MapieClassifier, MapieQuantileRegressor, MapieTimeSeriesRegressor, OldQuantile, MapieCalibrator, MapieMultiLabelClassifier, PrecisionRecallController, MondrianCP, ConformityScore


def removed_modules():
    import mapie.time_series_regression  # xpd: removed-in-mapie
    from mapie._typing import ArrayLike, NDArray  # xpd: removed-in-mapie
    from mapie.conformity_scores.residual_conformity_scores import GammaConformityScore  # xpd: removed-in-mapie
    from mapie.control_risk.ltt import ltt_procedure  # xpd: removed-in-mapie
    from mapie.metrics import regression_coverage_score as coverage  # xpd: removed-in-mapie
    from mapie.metrics import classification_coverage_score, classification_mean_width_score  # xpd: removed-in-mapie
    from mapie.metrics import expected_calibration_error, top_label_ece  # xpd: removed-in-mapie
    from mapie.metrics import regression_coverage_score_v2, top_label_ece as ece  # xpd: removed-in-mapie
    from mapie.metrics.regression import regression_coverage_score_v2 as coverage_v2  # xpd: removed-in-mapie

    return mapie.time_series_regression, ArrayLike, NDArray, GammaConformityScore, ltt_procedure, coverage, classification_coverage_score, classification_mean_width_score, expected_calibration_error, top_label_ece, regression_coverage_score_v2, ece, coverage_v2


@case
def imports_that_still_work():
    from mapie.conformity_scores import AbsoluteConformityScore, GammaConformityScore
    from mapie.metrics import calibration, classification, regression
    from mapie.metrics.calibration import expected_calibration_error, top_label_ece
    from mapie.metrics.classification import classification_coverage_score, classification_mean_width_score
    from numpy.typing import ArrayLike, NDArray

    try:
        from mapie.regression import MapieRegressor
    except ImportError:
        MapieRegressor = None
    names = [AbsoluteConformityScore, GammaConformityScore, calibration, classification, regression, expected_calibration_error, top_label_ece]
    return [n.__name__ for n in names], classification_coverage_score.__name__, classification_mean_width_score.__name__, str(ArrayLike) != str(NDArray), MapieRegressor


def removed_keywords():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    model = LinearRegression().fit(X_train, y_train)
    split = SplitConformalRegressor(model, alpha=0.1)  # xpd: removed-in-mapie
    prefit = SplitConformalRegressor(model, cv="prefit")  # xpd: removed-in-mapie
    cross = CrossConformalRegressor(LinearRegression(), cv="split", random_state=0)  # xpd: removed-in-mapie
    naive = CrossConformalRegressor(LinearRegression(), method="naive", random_state=0)  # xpd: removed-in-mapie
    scored = SplitConformalClassifier(LogisticRegression().fit(X_train, y_train > 0), method="score")  # xpd: removed-in-mapie
    conformal = SplitConformalRegressor(model, confidence_level=0.9).conformalize(X_conf, y_conf)
    y_pred, y_intervals = conformal.predict_interval(X_test, alpha=0.1)  # xpd: removed-in-mapie
    weights = np.ones(len(y_train))
    weighted = CrossConformalRegressor(LinearRegression(), random_state=0).fit_conformalize(X_train, y_train, sample_weight=weights)  # xpd: removed-in-mapie
    learner = SplitConformalRegressor(LinearRegression(), prefit=False)
    learner.fit(X_train, y_train, sample_weight=weights)  # xpd: removed-in-mapie
    return split, prefit, cross, naive, scored, y_pred, y_intervals, weighted, learner


def removed_metric_arguments():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    model = LinearRegression().fit(X_train, y_train)
    conformal = SplitConformalRegressor(model, confidence_level=0.9).conformalize(X_conf, y_conf)
    _, y_intervals = conformal.predict_interval(X_test)
    low, up = y_intervals[:, 0, 0], y_intervals[:, 1, 0]
    coverage = regression_coverage_score(y_test, low, up)  # xpd: removed-in-mapie
    width = regression_mean_width_score(low, up)  # xpd: removed-in-mapie
    winkler = regression_mwi_score(y_test, y_intervals, alpha=0.1)  # xpd: removed-in-mapie
    criterion = coverage_width_based(y_test, low, up, eta=10, alpha=0.1)  # xpd: removed-in-mapie
    return coverage, width, winkler, criterion


@case
def metrics_on_mapie_1():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    model = LinearRegression().fit(X_train, y_train)
    conformal = SplitConformalRegressor(model, confidence_level=0.9).conformalize(X_conf, y_conf)
    _, y_intervals = conformal.predict_interval(X_test)
    low, up = y_intervals[:, 0, 0], y_intervals[:, 1, 0]
    coverage = regression_coverage_score(y_test, y_intervals)[0]
    stacked = regression_coverage_score(y_test, np.column_stack([low, up]))[0]
    width = regression_mean_width_score(y_intervals)[0]
    stacked_width = regression_mean_width_score(np.column_stack([low, up])[:, :, np.newaxis])[0]
    winkler = regression_mwi_score(y_test, y_intervals, confidence_level=1 - 0.1)
    criterion = coverage_width_based(y_test, low, up, eta=10, confidence_level=1 - 0.1)
    level = SplitConformalRegressor(model, confidence_level=1 - 0.1).conformalize(X_conf, y_conf)
    return coverage, stacked, width, stacked_width, winkler, criterion, level.predict_interval(X_test)[1]


@case
def weights_in_fit_params():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    weights = np.ones(len(y_train))
    cross = CrossConformalRegressor(LinearRegression(), random_state=0).fit_conformalize(X_train, y_train, fit_params={"sample_weight": weights})
    learner = SplitConformalRegressor(LinearRegression(), prefit=False)
    learner.fit(X_train, y_train, fit_params={"sample_weight": weights})
    plain = LinearRegression()
    plain.fit(X_train, y_train, sample_weight=weights)
    return cross.predict_interval(X_test), learner.conformalize(X_conf, y_conf).predict_interval(X_test), plain.coef_


@case
def deprecated_risk_class():
    X, y = table()
    labels = (y > 0).astype(int)
    model = LogisticRegression().fit(X[:200], labels[:200])
    specificity = BinaryClassificationRisk(  # xpd: deprecated-in-mapie
        risk_occurrence=lambda y_true, y_pred: y_pred == 0,
        risk_condition=lambda y_true, y_pred: y_true == 0,
        higher_is_better=True,
    )
    controller = BinaryClassificationController(model.predict_proba, specificity, target_level=0.6, confidence_level=0.8, best_predict_param_choice="recall")
    controller.calibrate(X[200:400], labels[200:400])
    return controller.valid_predict_params, controller.best_predict_param, controller.p_values, controller.predict(X[400:])


@case
def current_risk_class():
    X, y = table()
    labels = (y > 0).astype(int)
    model = LogisticRegression().fit(X[:200], labels[:200])
    specificity = BinaryRisk(
        risk_occurrence=lambda y_true, y_pred: y_pred == 0,
        risk_condition=lambda y_true, y_pred: y_true == 0,
        higher_is_better=True,
    )
    controller = BinaryClassificationController(model.predict_proba, specificity, target_level=0.6, confidence_level=0.8, best_predict_param_choice="recall")
    controller.calibrate(X[200:400], labels[200:400])
    return controller.valid_predict_params, controller.best_predict_param, controller.p_values, controller.predict(X[400:])


@case
def deprecated_predict_keywords():
    X, y = table()
    cross = CrossConformalRegressor(LinearRegression(), cv=5, random_state=0).fit_conformalize(X[:400], y[:400])
    median = cross.predict_interval(X[400:], aggregate_predictions="median")  # xpd: deprecated-in-mapie
    single = cross.predict(X[400:], aggregate_predictions=None)  # xpd: deprecated-in-mapie
    jackknife = JackknifeAfterBootstrapRegressor(LinearRegression(), random_state=0)
    jackknife.fit_conformalize(X[:400], y[:400])
    apart = jackknife.predict_interval(X[400:], ensemble=False)  # xpd: deprecated-in-mapie
    together = jackknife.predict(X[400:], ensemble=True)  # xpd: deprecated-in-mapie
    return median, single, apart, together


@case
def current_predict_keywords():
    X, y = table()
    cross = CrossConformalRegressor(LinearRegression(), cv=5, random_state=0).fit_conformalize(X[:400], y[:400])
    median = cross.predict_interval(X[400:], aggregate_point_predictions="median")
    jackknife = JackknifeAfterBootstrapRegressor(LinearRegression(), random_state=0)
    jackknife.fit_conformalize(X[:400], y[:400])
    apart = jackknife.predict_interval(X[400:], aggregate_point_predictions=False)
    return median, apart
