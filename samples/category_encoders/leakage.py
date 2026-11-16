"""Target encoders that see the labels of rows they should not, next to the forms that do not.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import category_encoders as ce
import numpy as np
import pandas as pd
from category_encoders import TargetEncoder
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import cross_val_score, train_test_split
from sklearn.pipeline import make_pipeline

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def frame(n=300, seed=0):
    r = np.random.default_rng(seed)
    city = r.choice(["berlin", "paris", "rome", "oslo"], n, p=[0.4, 0.3, 0.25, 0.05])
    x1 = r.normal(size=n)
    y = ((city == "berlin") * 1.5 + x1 + r.normal(scale=0.5, size=n) > 0.8).astype(int)
    return pd.DataFrame({"city": city, "x1": x1}), pd.Series(y)


@case
def leave_one_out_with_test_target():
    X, y = frame()
    X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=0)
    encoder = ce.LeaveOneOutEncoder(cols=["city"])
    train = encoder.fit_transform(X_train, y_train)
    leaked = encoder.transform(X_test, y_test)  # xpd: transform-with-test-target
    named = encoder.transform(X_test, y=y_test)  # xpd: transform-with-test-target
    honest = encoder.transform(X_test)
    with_training_target = encoder.transform(X_train, y_train)
    return train, leaked, named, honest, with_training_target


@case
def other_encoders_with_test_target():
    X, y = frame()
    X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=0)
    woe = ce.WOEEncoder(cols=["city"]).fit(X_train, y_train)
    target = ce.TargetEncoder(cols=["city"], smoothing=5.0)
    target.fit(X_train, y_train)
    ordinal = ce.OrdinalEncoder(cols=["city"], handle_unknown="error").fit(X_train)
    a = woe.transform(X_test, y_test)  # xpd: transform-with-test-target
    b = target.transform(X_test, y_test)  # xpd: transform-with-test-target
    frame_out = ordinal.transform(X_test, True)
    return a, b, frame_out, woe.transform(X_test), target.transform(X_train, y_train)


@case
def encoded_before_split():
    X, y = frame()
    X_encoded = TargetEncoder(cols=["city"], smoothing=5.0).fit_transform(X, y)  # xpd: encode-before-split
    X_train, X_test, y_train, y_test = train_test_split(X_encoded, y, random_state=0)
    return LogisticRegression().fit(X_train, y_train).score(X_test, y_test)


@case
def encoded_after_split():
    X, y = frame()
    X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=0)
    encoder = TargetEncoder(cols=["city"], smoothing=5.0)
    X_train = encoder.fit_transform(X_train, y_train)
    X_test = encoder.transform(X_test)
    return LogisticRegression().fit(X_train, y_train).score(X_test, y_test)


@case
def encoded_before_cross_validation():
    X, y = frame()
    X_encoded = TargetEncoder(cols=["city"], smoothing=5.0).fit_transform(X, y)  # xpd: encode-before-split
    stored = cross_val_score(LogisticRegression(), X_encoded, y, cv=3)
    inline = cross_val_score(LogisticRegression(), TargetEncoder(cols=["city"], smoothing=5.0).fit_transform(X, y), y, cv=3)  # xpd: encode-before-split
    piped = cross_val_score(make_pipeline(TargetEncoder(cols=["city"], smoothing=5.0), LogisticRegression()), X, y, cv=3)
    return stored, inline, piped
