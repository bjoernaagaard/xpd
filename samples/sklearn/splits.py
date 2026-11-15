"""Splits that put related rows on both sides, splits that cannot run, and the honest forms.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes. A form that raises on scikit-learn 1.9.1 sits in a function that is not a case.
"""

import numpy as np
import pandas as pd
from sklearn.ensemble import RandomForestClassifier
from sklearn.linear_model import LinearRegression, LogisticRegression, Ridge
from sklearn.model_selection import (
    GridSearchCV,
    GroupKFold,
    GroupShuffleSplit,
    KFold,
    LeaveOneOut,
    StratifiedKFold,
    cross_val_predict,
    cross_val_score,
    train_test_split,
)
from sklearn.metrics import r2_score

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def visits(n_patients=30, rows=6, seed=0):
    r = np.random.default_rng(seed)
    patient = np.repeat(np.arange(n_patients), rows)
    centre = r.normal(size=(n_patients, 3))
    X = centre[patient] + r.normal(scale=0.3, size=(len(patient), 3))
    y = r.permutation(np.repeat([0, 1], n_patients // 2))[patient]
    order = r.permutation(len(patient))
    return X[order], y[order], patient[order]


def days(n=200, seed=0):
    r = np.random.default_rng(seed)
    date = pd.date_range("2024-01-01", periods=n, freq="D")
    frame = pd.DataFrame({"date": r.permutation(date), "load": r.normal(size=n).cumsum()})
    frame["target"] = (frame["load"] > frame["load"].median()).astype(int)
    return frame


@case
def groups_ignored():
    X, y, patient = visits()
    model = RandomForestClassifier(n_estimators=10, random_state=0)
    a = cross_val_score(model, X, y, groups=patient, cv=KFold(5))  # xpd: groups-ignored
    b = cross_val_score(model, X, y, groups=patient)  # xpd: groups-ignored
    c = cross_val_score(model, X, y, groups=patient, cv=GroupKFold(5))
    d = cross_val_score(model, X, y, groups=patient, cv=5)  # xpd: groups-ignored
    folds = KFold(5, shuffle=True, random_state=0)
    shuffled = [test for _, test in folds.split(X, y, patient)]  # xpd: groups-ignored
    direct = [test for _, test in StratifiedKFold(3).split(X, y, patient)]  # xpd: groups-ignored
    by_patient = GroupKFold(5)
    grouped = [test for _, test in by_patient.split(X, y, patient)]
    kept = cross_val_score(model, X, y, groups=patient, cv=folds)  # xpd: groups-ignored
    by_group = cross_val_score(model, X, y, groups=patient, cv=by_patient)
    return model, a, b, c, d, shuffled, direct, grouped, kept, by_group


def missing_groups():
    X, y, patient = visits()
    model = LogisticRegression()
    scores = cross_val_score(model, X, y, cv=GroupKFold(5))  # xpd: missing-groups
    folds = list(GroupShuffleSplit(3, random_state=0).split(X, y))  # xpd: missing-groups
    by_patient = GroupKFold(n_splits=5)
    more = list(by_patient.split(X, y))  # xpd: missing-groups
    search = GridSearchCV(model, {"C": [0.1, 1.0]}, cv=GroupKFold(5))
    search.fit(X, y)  # xpd: missing-groups
    kept = cross_val_score(model, X, y, cv=by_patient)  # xpd: missing-groups
    return scores, folds, more, search, kept


@case
def with_groups():
    X, y, patient = visits()
    model = LogisticRegression()
    scores = cross_val_score(model, X, y, groups=patient, cv=GroupKFold(5))
    folds = [test for _, test in GroupShuffleSplit(3, random_state=0).split(X, y, patient)]
    by_patient = GroupKFold(n_splits=5)
    more = [test for _, test in by_patient.split(X, y, patient)]
    search = GridSearchCV(model, {"C": [0.1, 1.0]}, cv=GroupKFold(5))
    search.fit(X, y, groups=patient)
    plain = GridSearchCV(model, {"C": [0.1, 1.0]}, cv=5)
    plain.fit(X, y)
    kept = cross_val_score(model, X, y, groups=patient, cv=by_patient)
    folds_by_row = KFold(5, shuffle=True, random_state=0)
    by_row = cross_val_score(model, X, y, cv=folds_by_row)
    return scores, folds, more, search.best_estimator_, plain.best_estimator_, kept, by_row


@case
def lag_then_shuffled_split():
    lagged = days()
    lagged["load_before"] = lagged["load"].shift(1)
    lagged = lagged.dropna()
    X_train, X_test, y_train, y_test = train_test_split(lagged[["load_before"]], lagged["target"], random_state=0)
    lagged.sort_values("date", inplace=True)  # xpd: no-inplace
    model = RandomForestClassifier(n_estimators=10, random_state=0).fit(X_train, y_train)
    return model, model.predict(X_test)


@case
def sorted_in_place():
    frame = days()
    frame.sort_values(by="date", inplace=True)  # xpd: no-inplace
    shown = frame.sort_values("date").head(5)  # xpd: sort-then-head
    X_train, X_test, y_train, y_test = train_test_split(frame[["load"]], frame["target"], random_state=0)
    model = RandomForestClassifier(n_estimators=10, random_state=0).fit(X_train, y_train)
    return model, model.predict(X_test), shown


@case
def sorted_for_display():
    frame = days()
    frame.sort_values("date").head(5)  # xpd: sort-then-head
    first = frame.sort_values("date").iloc[0]  # xpd: sort-then-head
    X_train, X_test, y_train, y_test = train_test_split(frame[["load"]], frame["target"], random_state=0)
    model = RandomForestClassifier(n_estimators=10, random_state=0).fit(X_train, y_train)
    return model, model.predict(X_test), first


@case
def leave_one_out_score():
    r = np.random.default_rng(0)
    X = r.normal(size=(30, 2))
    y = X[:, 0] + r.normal(size=30)
    c = (y > 0).astype(int)
    a = cross_val_score(LinearRegression(), X, y, cv=LeaveOneOut())  # xpd: leave-one-out-score
    b = cross_val_score(LogisticRegression(), X, c, cv=LeaveOneOut(), scoring="roc_auc")  # xpd: leave-one-out-score
    search = GridSearchCV(Ridge(), {"alpha": [0.1, 1.0]}, cv=LeaveOneOut()).fit(X, y)  # xpd: leave-one-out-score
    return a, b, search.best_estimator_


@case
def leave_one_out_by_row():
    r = np.random.default_rng(0)
    X = r.normal(size=(30, 2))
    y = X[:, 0] + r.normal(size=30)
    c = (y > 0).astype(int)
    a = r2_score(y, cross_val_predict(LinearRegression(), X, y, cv=LeaveOneOut()))
    b = cross_val_score(LogisticRegression(), X, c, cv=LeaveOneOut())
    d = cross_val_score(LinearRegression(), X, y, cv=LeaveOneOut(), scoring="neg_mean_squared_error")
    search = GridSearchCV(Ridge(), {"alpha": [0.1, 1.0]}, cv=LeaveOneOut(), scoring="neg_mean_squared_error").fit(X, y)
    return a, b, d, search.best_estimator_
