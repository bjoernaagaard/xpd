"""Metrics given the wrong input, and the same metrics given the right one.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file. A form
that raises on scikit-learn 1.9.1 sits in a function that is not a case.
"""

import numpy as np
from sklearn import metrics
from sklearn.isotonic import IsotonicRegression
from sklearn.linear_model import LinearRegression, LogisticRegression
from sklearn.metrics import (
    accuracy_score,
    average_precision_score,
    brier_score_loss,
    log_loss,
    precision_recall_curve,
    roc_auc_score,
    roc_curve,
)
from sklearn.model_selection import train_test_split

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def data(n=300, classes=2, seed=0):
    r = np.random.default_rng(seed)
    X = r.normal(size=(n, 3))
    y = np.digitize(X[:, 0] + r.normal(scale=0.7, size=n), np.linspace(-0.6, 0.6, classes - 1)) if classes > 2 else (X[:, 0] + r.normal(scale=0.7, size=n) > 0.3).astype(int)
    return train_test_split(X, y, test_size=0.3, random_state=0)


def fitted(classes=2):
    X_train, X_test, y_train, y_test = data(classes=classes)
    return LogisticRegression().fit(X_train, y_train), X_test, y_test


@case
def auc_from_labels():
    model, X_test, y_test = fitted()
    return (
        model,
        roc_auc_score(y_test, model.predict(X_test)),  # xpd: auc-from-labels
        average_precision_score(y_test, model.predict(X_test), pos_label=1),  # xpd: auc-from-labels
        metrics.roc_curve(y_test, model.predict(X_test)),  # xpd: auc-from-labels
        roc_auc_score(y_test, model.predict_proba(X_test)[:, 0]),  # xpd: auc-from-labels
        roc_auc_score(y_test, model.predict_proba(X_test)[:, 1]),
        roc_auc_score(y_test, model.decision_function(X_test)),
        accuracy_score(y_test, model.predict(X_test)),
    )


@case
def auc_from_a_label_variable():
    model, X_test, y_test = fitted()
    y_pred = model.predict(X_test)  # xpd: auc-from-labels
    accuracy = accuracy_score(y_test, y_pred)
    precision, recall, _ = precision_recall_curve(y_test, y_pred)
    return model, accuracy, precision, recall


@case
def auc_from_a_name_assigned_again():
    model, X_test, y_test = fitted()
    y_pred = model.predict(X_test)
    accuracy = accuracy_score(y_test, y_pred)
    y_pred = model.predict_proba(X_test)[:, 1]
    fpr, tpr, _ = roc_curve(y_test, y_pred)
    return model, accuracy, fpr, tpr


@case
def auc_from_a_calibrator():
    model, X_test, y_test = fitted()
    y_score = model.decision_function(X_test)
    calibrator = IsotonicRegression(out_of_bounds="clip").fit(y_score, y_test)
    return model, roc_auc_score(y_test, calibrator.predict(y_score))


@case
def loss_from_labels():
    model, X_test, y_test = fitted()
    return (
        model,
        log_loss(y_test, model.predict(X_test)),  # xpd: auc-from-labels
        brier_score_loss(y_test, model.predict(X_test)),  # xpd: auc-from-labels
        log_loss(y_test, model.predict_proba(X_test)),
        brier_score_loss(y_test, model.predict_proba(X_test)[:, 1]),
    )


@case
def loss_from_a_label_variable():
    model, X_test, y_test = fitted()
    labels = model.predict(X_test)  # xpd: auc-from-labels
    probabilities = model.predict_proba(X_test)
    return model, log_loss(y_test, labels), log_loss(y_test, probabilities)


def auc_input_that_raises():
    model, X_test, y_test = fitted()
    roc_auc_score(y_test, model.predict_proba(X_test))  # xpd: auc-from-labels
    roc_curve(y_test, model.predict_proba(X_test))  # xpd: auc-from-labels
    log_loss(y_test, model.decision_function(X_test))  # xpd: auc-from-labels
    many, X_many, y_many = fitted(classes=3)
    roc_auc_score(y_many, many.predict(X_many), multi_class="ovr")  # xpd: auc-from-labels


@case
def auc_on_more_classes():
    model, X_test, y_test = fitted(classes=3)
    return model, roc_auc_score(y_test, model.predict_proba(X_test), multi_class="ovr")


@case
def metric_arguments_swapped():
    model, X_test, y_test = fitted()
    y_true = y_test
    y_pred = model.predict(X_test)
    return (
        model,
        metrics.precision_score(y_pred, y_test),  # xpd: metric-arguments-swapped
        metrics.classification_report(y_pred, y_true, output_dict=True),  # xpd: metric-arguments-swapped
        metrics.precision_recall_fscore_support(y_pred, y_test, average="macro"),  # xpd: metric-arguments-swapped
        metrics.recall_score(y_pred, y_test),  # xpd: metric-arguments-swapped
        metrics.recall_score(y_pred, y_true, average="macro"),  # xpd: metric-arguments-swapped
        metrics.confusion_matrix(y_pred, y_test),  # xpd: metric-arguments-swapped
    )


@case
def regression_arguments_swapped():
    r = np.random.default_rng(0)
    X = r.normal(size=(200, 2))
    y = X[:, 0] + r.normal(size=200)
    X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=0)
    model = LinearRegression().fit(X_train, y_train)
    y_hat = model.predict(X_test)
    return (
        model,
        metrics.r2_score(y_hat, y_test),  # xpd: metric-arguments-swapped
        metrics.r2_score(y_test, y_hat),
        metrics.mean_squared_error(y_hat, y_test),
    )


@case
def metric_arguments_in_order():
    model, X_test, y_test = fitted()
    y_pred = model.predict(X_test)
    y_test_pred = y_pred
    y_score = model.predict_proba(X_test)[:, 1]
    return (
        model,
        metrics.precision_score(y_test, y_pred),
        metrics.recall_score(y_test, y_test_pred),
        accuracy_score(y_pred, y_test),
        metrics.f1_score(y_pred, y_test),
        roc_auc_score(y_test, y_score),
    )


def scores_first_raises():
    model, X_test, y_test = fitted()
    y_score = model.predict_proba(X_test)[:, 1]
    roc_auc_score(y_score, y_test)  # xpd: metric-arguments-swapped
    log_loss(model.predict_proba(X_test), y_test)  # xpd: metric-arguments-swapped
    precision_recall_curve(model.decision_function(X_test), y_test)  # xpd: metric-arguments-swapped


def removed_metric_api():
    from sklearn.metrics.ranking import roc_auc_score  # xpd: removed-metric-api
    from sklearn.metrics.scorer import SCORERS  # xpd: removed-metric-api
    from sklearn.metrics import jaccard_similarity_score  # xpd: removed-metric-api
    from sklearn.metrics import accuracy_score, plot_confusion_matrix  # xpd: removed-metric-api
    from sklearn.metrics import RocCurveDisplay, make_scorer, mean_squared_error

    model, X_test, y_test = fitted()
    y_score = model.predict_proba(X_test)[:, 1]
    print(sorted(metrics.SCORERS))  # xpd: removed-metric-api
    metrics.plot_roc_curve(model, X_test, y_test)  # xpd: removed-metric-api
    mean_squared_error(y_test, y_score, squared=False)  # xpd: removed-metric-api
    metrics.mean_squared_log_error(y_test, y_score, squared=False)  # xpd: removed-metric-api
    make_scorer(mean_squared_error, squared=False, greater_is_better=False)  # xpd: removed-metric-api
    log_loss(y_test, y_score, eps=1e-15)  # xpd: removed-metric-api
    make_scorer(roc_auc_score, needs_proba=True)  # xpd: removed-metric-api
    metrics.make_scorer(roc_auc_score, needs_threshold=True)  # xpd: removed-metric-api
    fpr, tpr, _ = roc_curve(y_test, y_score)
    RocCurveDisplay(fpr=fpr, tpr=tpr, estimator_name="model")  # xpd: removed-metric-api
    RocCurveDisplay.from_predictions(y_test, y_pred=y_score)  # xpd: removed-metric-api
    RocCurveDisplay.from_estimator(model, X_test, y_test, label="test", color="red")  # xpd: removed-metric-api
    RocCurveDisplay(fpr=fpr, tpr=tpr).plot(color="red")  # xpd: removed-metric-api
    RocCurveDisplay.from_estimator(model, X_test, y_test, name="test", curve_kwargs={"color": "red"})
    RocCurveDisplay(fpr=fpr, tpr=tpr).plot(curve_kwargs={"color": "red"})
    return accuracy_score, SCORERS, jaccard_similarity_score, plot_confusion_matrix


@case
def current_metric_api():
    from sklearn.metrics import RocCurveDisplay, get_scorer_names, jaccard_score, make_scorer, root_mean_squared_error

    model, X_test, y_test = fitted()
    y_score = model.predict_proba(X_test)[:, 1]
    fpr, tpr, _ = roc_curve(y_test, y_score)
    return (
        model,
        get_scorer_names(),
        jaccard_score(y_test, model.predict(X_test)),
        root_mean_squared_error(y_test, y_score),
        metrics.mean_squared_error(y_test, y_score),
        log_loss(y_test, y_score),
        make_scorer(roc_auc_score, response_method="predict_proba")(model, X_test, y_test),
        RocCurveDisplay(fpr=fpr, tpr=tpr, name="model").name,
    )
