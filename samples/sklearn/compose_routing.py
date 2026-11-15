"""Metadata routing forms that raise once routing is on, on scikit-learn 1.9.1.

The file turns routing on, so it sits apart from compose.py, where routing stays off. The case
turns it on with config_context, so the setting does not leak into other samples. A line that
xpd reports ends with a comment that names the rule. A form that raises sits in a plain
function, which the checker never calls.
"""

import numpy as np
import sklearn
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import GroupKFold, cross_val_score, cross_validate
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import StandardScaler
from imblearn.over_sampling import SMOTE
from imblearn.pipeline import make_pipeline as resampling_pipeline

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def data(seed=0, n=60):
    r = np.random.default_rng(seed)
    return r.normal(size=(n, 3)), np.tile([0, 1], n // 2), r.uniform(0.5, 2.0, n), np.arange(n) % 6


def routing_is_on():
    X, y, w, g = data()
    cross_val_score(LogisticRegression(), X, y, groups=g, cv=GroupKFold(3))  # xpd: metadata-routing
    make_pipeline(StandardScaler(), LogisticRegression()).fit(X, y, logisticregression__sample_weight=w)  # xpd: metadata-routing


def nothing_requested():
    X, y, w, g = data()
    cross_validate(LogisticRegression(), X, y, params={"sample_weight": w})  # xpd: metadata-routing


@case
def routing_on_forms_that_run():
    with sklearn.config_context(enable_metadata_routing=True):
        return routed_fits()


def routed_fits():
    X, y, w, g = data()
    scores = cross_val_score(LogisticRegression(), X, y, params={"groups": g}, cv=GroupKFold(3))
    model = LogisticRegression().set_fit_request(sample_weight=True).set_score_request(sample_weight=True)
    weighted = cross_validate(model, X, y, params={"sample_weight": w})["test_score"]
    pipe = make_pipeline(StandardScaler().set_fit_request(sample_weight=False), LogisticRegression().set_fit_request(sample_weight=True))
    pipe.fit(X, y, sample_weight=w)
    resampled = resampling_pipeline(SMOTE(random_state=0), LogisticRegression()).fit(X, y)
    return scores, weighted, pipe.predict(X), resampled.predict(X)
