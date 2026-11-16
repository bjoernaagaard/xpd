"""MAPIE objects that draw random numbers without a seed, and the same objects with one.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from mapie.classification import CrossConformalClassifier, SplitConformalClassifier
from mapie.regression import CrossConformalRegressor, JackknifeAfterBootstrapRegressor
from mapie.subsample import BlockBootstrap, Subsample
from mapie.utils import train_conformalize_test_split
from sklearn.linear_model import LinearRegression, LogisticRegression
from sklearn.model_selection import KFold

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def table(n=400, seed=0):
    r = np.random.default_rng(seed)
    X = r.normal(size=(n, 3))
    y = X[:, 0] + r.normal(size=n)
    return X, y


@case
def without_a_seed():
    X, y = table()
    labels = np.digitize(y, [-0.5, 0.5])
    parts = train_conformalize_test_split(X, y, train_size=0.6, conformalize_size=0.2, test_size=0.2)  # xpd: unseeded-resampling
    cross = CrossConformalRegressor(LinearRegression(), confidence_level=0.9)  # xpd: unseeded-resampling
    folds = CrossConformalClassifier(LogisticRegression(), cv=5)  # xpd: unseeded-resampling
    jackknife = JackknifeAfterBootstrapRegressor(LinearRegression(), resampling=20)  # xpd: unseeded-resampling
    samples = Subsample(n_resamplings=10)  # xpd: unseeded-resampling
    blocks = BlockBootstrap(n_resamplings=10, length=20)  # xpd: unseeded-resampling
    adaptive = SplitConformalClassifier(LogisticRegression().fit(X, labels), conformity_score="aps")  # xpd: unseeded-resampling
    return parts, cross, folds, jackknife, samples, blocks, adaptive


@case
def with_a_seed():
    X, y = table()
    labels = np.digitize(y, [-0.5, 0.5])
    parts = train_conformalize_test_split(X, y, train_size=0.6, conformalize_size=0.2, test_size=0.2, random_state=0)
    in_order = train_conformalize_test_split(X, y, train_size=0.6, conformalize_size=0.2, test_size=0.2, shuffle=False)
    cross = CrossConformalRegressor(LinearRegression(), confidence_level=0.9, random_state=0)
    ordered = CrossConformalRegressor(LinearRegression(), cv=KFold(5))
    jackknife = JackknifeAfterBootstrapRegressor(LinearRegression(), resampling=Subsample(n_resamplings=10, random_state=0))
    least = SplitConformalClassifier(LogisticRegression().fit(X, labels), conformity_score="lac")
    adaptive = SplitConformalClassifier(LogisticRegression().fit(X, labels), conformity_score="aps", random_state=0)
    return parts, in_order, cross, ordered, jackknife, least, adaptive
