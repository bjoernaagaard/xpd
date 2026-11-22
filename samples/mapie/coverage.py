"""Conformal objects calibrated or scored on rows they have seen, and the same objects on held-out rows.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from mapie.calibration import TopLabelCalibrator, VennAbersCalibrator
from mapie.classification import SplitConformalClassifier
from mapie.metrics.classification import classification_coverage_score
from mapie.metrics.regression import coverage_width_based, regression_coverage_score, regression_mwi_score
from mapie.regression import CrossConformalRegressor, SplitConformalRegressor
from mapie.risk_control import BinaryClassificationController
from mapie.utils import train_conformalize_test_split
from sklearn.ensemble import RandomForestClassifier, RandomForestRegressor
from sklearn.model_selection import GridSearchCV
from sklearn.preprocessing import StandardScaler
from sklearn.tree import DecisionTreeRegressor

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def three_parts(seed=0, n=900, classes=False):
    r = np.random.default_rng(seed)
    X = r.normal(size=(n, 4))
    y = X[:, 0] * 2 + np.sin(3 * X[:, 1]) + r.normal(size=n)
    if classes:
        y = np.digitize(y, [-1.0, 1.0])
    return train_conformalize_test_split(X, y, train_size=0.4, conformalize_size=0.3, test_size=0.3, random_state=seed)


def coverage(y, intervals):
    return float(regression_coverage_score(y, intervals)[0])  # xpd: float-around-number


@case
def conformalize_on_training_rows():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    model = RandomForestRegressor(n_estimators=20, random_state=0).fit(X_train, y_train)
    narrow = SplitConformalRegressor(model, confidence_level=0.9, prefit=True).conformalize(X_train, y_train)  # xpd: calibrate-on-training-rows
    honest = SplitConformalRegressor(model, confidence_level=0.9, prefit=True).conformalize(X_conf, y_conf)
    return coverage(y_test, narrow.predict_interval(X_test)[1]), coverage(y_test, honest.predict_interval(X_test)[1])


@case
def classifier_on_training_rows():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts(classes=True)
    model = RandomForestClassifier(n_estimators=20, random_state=0).fit(X_train, y_train)
    narrow = SplitConformalClassifier(model, confidence_level=0.9)
    narrow.conformalize(X_train, y_train)  # xpd: calibrate-on-training-rows
    honest = SplitConformalClassifier(model, confidence_level=0.9)
    honest.conformalize(X_conf, y_conf)
    return narrow.predict_set(X_test)[1].sum(), honest.predict_set(X_test)[1].sum()


@case
def conformalize_on_the_fitted_rows():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    X_fit, y_fit = X_train, y_train
    learner = SplitConformalRegressor(RandomForestRegressor(n_estimators=20, random_state=0), confidence_level=0.9, prefit=False)
    learner.fit(X_fit, y_fit)
    learner.conformalize(X_fit, y_fit)  # xpd: calibrate-on-training-rows
    chained = SplitConformalRegressor(RandomForestRegressor(n_estimators=20, random_state=0), confidence_level=0.9, prefit=False)
    chained = chained.fit(X_conf, y_conf).conformalize(X_conf, y_conf)  # xpd: calibrate-on-training-rows
    return coverage(y_test, learner.predict_interval(X_test)[1]), coverage(y_test, chained.predict_interval(X_test)[1])


@case
def other_rows_under_the_same_name():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    X_part, y_part = X_train, y_train
    model = RandomForestRegressor(n_estimators=20, random_state=0)
    model.fit(X_part, y_part)
    X_part, y_part = X_conf, y_conf
    held_out = SplitConformalRegressor(model, confidence_level=0.9, prefit=True)
    held_out.conformalize(X_part, y_part)
    return coverage(y_test, held_out.predict_interval(X_test)[1])


@case
def search_on_the_conformalize_rows():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    grid = {"max_depth": [2, 4, None]}
    search = GridSearchCV(DecisionTreeRegressor(random_state=0), grid, cv=3)
    search.fit(X_conf, y_conf)
    tuned = SplitConformalRegressor(search, confidence_level=0.9, prefit=True)
    tuned.conformalize(X_conf, y_conf)  # xpd: calibrate-on-training-rows
    return coverage(y_test, tuned.predict_interval(X_test)[1])


@case
def held_out_rows_for_each_step():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    scaler = StandardScaler()
    scaler.fit(X_conf)
    search = GridSearchCV(DecisionTreeRegressor(random_state=0), {"max_depth": [2, 4, None]}, cv=3)
    search.fit(X_train, y_train)
    tuned = SplitConformalRegressor(search, confidence_level=0.9, prefit=True)
    tuned.conformalize(X_conf, y_conf)
    X_train_and_conf = np.concatenate([X_train, X_conf])
    y_train_and_conf = np.concatenate([y_train, y_conf])
    cross = CrossConformalRegressor(DecisionTreeRegressor(random_state=0), confidence_level=0.9, cv=5, random_state=0)
    cross.fit_conformalize(X_train_and_conf, y_train_and_conf)
    return coverage(y_test, tuned.predict_interval(X_test)[1]), coverage(y_test, cross.predict_interval(X_test)[1])


@case
def controller_on_training_rows():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    model = RandomForestClassifier(n_estimators=20, random_state=0).fit(X_train, y_train > 0)
    eager = BinaryClassificationController(model.predict_proba, "precision", target_level=0.8, confidence_level=0.9)
    eager.calibrate(X_train, y_train > 0)  # xpd: calibrate-on-training-rows
    careful = BinaryClassificationController(model.predict_proba, "precision", target_level=0.8, confidence_level=0.9)
    careful.calibrate(X_conf, y_conf > 0)
    return eager.best_predict_param, careful.best_predict_param


@case
def calibrator_on_training_rows():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts(classes=True)
    model = RandomForestClassifier(n_estimators=20, random_state=0).fit(X_train, y_train)
    sure = TopLabelCalibrator(model, cv="prefit").fit(X_train, y_train)  # xpd: calibrate-on-training-rows
    venn = VennAbersCalibrator(model, cv="prefit")
    venn.fit(X_train, y_train)  # xpd: calibrate-on-training-rows
    held_out = TopLabelCalibrator(model, cv="prefit").fit(X_conf, y_conf)
    splitting = TopLabelCalibrator(RandomForestClassifier(n_estimators=20, random_state=0), cv="split").fit(X_train, y_train, random_state=0)
    return sure.predict_proba(X_test), venn.predict_proba(X_test), held_out.predict_proba(X_test), splitting.predict_proba(X_test)


@case
def coverage_on_seen_rows():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    cross = CrossConformalRegressor(RandomForestRegressor(n_estimators=20, random_state=0), confidence_level=0.9, cv=5, random_state=0)
    cross.fit_conformalize(X_train, y_train)
    _, seen = cross.predict_interval(X_train)  # xpd: coverage-on-seen-rows
    _, unseen = cross.predict_interval(X_test)
    learner = SplitConformalRegressor(RandomForestRegressor(n_estimators=20, random_state=0), confidence_level=0.9, prefit=False)
    learner.fit(X_train, y_train)
    learner.conformalize(X_conf, y_conf)
    _, fitted = learner.predict_interval(X_train, allow_infinite_bounds=True)  # xpd: coverage-on-seen-rows
    _, conformalized = learner.predict_interval(X_conf)  # xpd: coverage-on-seen-rows
    _, held_out = learner.predict_interval(X_test)
    on_seen = regression_coverage_score(y_train, seen)[0], regression_coverage_score(y_train, fitted)[0], regression_coverage_score(y_conf, conformalized[:, :, 0])[0]
    inline = regression_coverage_score(y_train, cross.predict_interval(X_train)[1])[0]  # xpd: coverage-on-seen-rows
    on_new = regression_coverage_score(y_test, unseen)[0], regression_coverage_score(y_test, held_out)[0]
    return [float(score) for score in (*on_seen, inline, *on_new)]  # xpd: float-around-number


@case
def sets_on_seen_rows():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts(classes=True)
    learner = SplitConformalClassifier(RandomForestClassifier(n_estimators=20, random_state=0), confidence_level=0.9, prefit=False)
    learner.fit(X_train, y_train)
    learner.conformalize(X_conf, y_conf)
    _, seen = learner.predict_set(X_train)  # xpd: coverage-on-seen-rows
    _, held_out = learner.predict_set(X_test)
    return float(classification_coverage_score(y_train, seen)[0]), float(classification_coverage_score(y_test, held_out)[0])  # xpd: float-around-number, float-around-number


@case
def seen_rows_that_are_not_scored():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    cross = CrossConformalRegressor(RandomForestRegressor(n_estimators=20, random_state=0), confidence_level=0.9, cv=5, random_state=0)
    cross.fit_conformalize(X_train, y_train)
    _, drawn = cross.predict_interval(X_train)
    _, scored = cross.predict_interval(X_test)
    return drawn.shape, float(regression_coverage_score(y_test, scored)[0])  # xpd: float-around-number


@case
def alpha_as_confidence_level():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    model = RandomForestRegressor(n_estimators=20, random_state=0).fit(X_train, y_train)
    alpha = 0.1
    tiny = SplitConformalRegressor(model, confidence_level=0.1).conformalize(X_conf, y_conf)  # xpd: alpha-as-confidence-level
    named = SplitConformalRegressor(model, confidence_level=alpha).conformalize(X_conf, y_conf)  # xpd: alpha-as-confidence-level
    several = SplitConformalRegressor(model, confidence_level=[0.05, 0.1]).conformalize(X_conf, y_conf)  # xpd: alpha-as-confidence-level
    wide = SplitConformalRegressor(model, confidence_level=1 - alpha).conformalize(X_conf, y_conf)
    levels = SplitConformalRegressor(model, confidence_level=[0.8, 0.9]).conformalize(X_conf, y_conf)
    half = SplitConformalRegressor(model, confidence_level=0.5).conformalize(X_conf, y_conf)
    _, y_intervals = wide.predict_interval(X_test)
    score = regression_mwi_score(y_test, y_intervals, 0.1)  # xpd: alpha-as-confidence-level
    by_name = regression_mwi_score(y_test, y_intervals, alpha)  # xpd: alpha-as-confidence-level
    right = regression_mwi_score(y_test, y_intervals, 1 - alpha)
    width = coverage_width_based(y_test, y_intervals[:, 0, 0], y_intervals[:, 1, 0], 10, 0.1)  # xpd: alpha-as-confidence-level
    spread = [m.predict_interval(X_test)[1] for m in (tiny, named, several, levels, half)]
    return spread, score, by_name, right, width


def percent_as_confidence_level():
    X_train, X_conf, X_test, y_train, y_conf, y_test = three_parts()
    model = RandomForestRegressor(n_estimators=20, random_state=0).fit(X_train, y_train)
    percent = SplitConformalRegressor(model, confidence_level=90).conformalize(X_conf, y_conf)  # xpd: alpha-as-confidence-level
    controller = BinaryClassificationController(model.predict, "precision", target_level=0.8, confidence_level=95.0)  # xpd: alpha-as-confidence-level, predict-function-returns-labels
    return percent.predict_interval(X_test), controller
