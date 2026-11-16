"""Encoders inside scikit-learn pipelines and column transformers, in forms that go wrong and right.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes. A form that raises is built through `attempt`, which returns None when it raises.
"""

import category_encoders as ce
import numpy as np
import pandas as pd
from sklearn.compose import ColumnTransformer, make_column_transformer
from sklearn.ensemble import HistGradientBoostingClassifier
from sklearn.impute import SimpleImputer
from sklearn.linear_model import LogisticRegression
from sklearn.pipeline import Pipeline, make_pipeline
from sklearn.preprocessing import OneHotEncoder, StandardScaler

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def frame(n=200, seed=0):
    r = np.random.default_rng(seed)
    X = pd.DataFrame({"city": r.choice(["berlin", "paris", "rome", "oslo"], n), "zip": r.integers(0, 5, n), "x": r.normal(size=n)})
    return X, pd.Series(r.integers(0, 2, n))


def attempt(build):
    """What build returns, or None when it raises."""
    try:
        return build()
    except (TypeError, ValueError):
        return None


@case
def nan_for_unseen_category():
    X, y = frame()
    new = pd.DataFrame({"city": ["berlin", "madrid"], "zip": [1, 2], "x": [0.0, 1.0]})
    raising = make_pipeline(ce.BinaryEncoder(cols=["city"], handle_unknown="return_nan"), LogisticRegression())  # xpd: nan-into-pipeline
    named = Pipeline([("encode", ce.CatBoostEncoder(cols=["city"], handle_missing="return_nan")), ("model", LogisticRegression())])  # xpd: nan-into-pipeline
    counted = make_pipeline(ce.CountEncoder(cols=["city"], handle_unknown=None), LogisticRegression())  # xpd: nan-into-pipeline
    zeros = make_pipeline(ce.BinaryEncoder(cols=["city"], handle_unknown="value"), LogisticRegression())
    imputed = make_pipeline(ce.BinaryEncoder(cols=["city"], handle_unknown="return_nan"), SimpleImputer(), LogisticRegression())
    takes_nan = make_pipeline(ce.BinaryEncoder(cols=["city"], handle_unknown="return_nan"), HistGradientBoostingClassifier(random_state=0))
    alone = ce.BinaryEncoder(cols=["city"], handle_unknown="return_nan").fit(X)
    return (
        attempt(lambda: raising.fit(X, y).predict(new)),
        attempt(lambda: counted.fit(X, y).predict(new)),
        named.fit(X, y).predict(X),
        zeros.fit(X, y).predict(new),
        imputed.fit(X, y).predict(new),
        takes_nan.fit(X, y).predict(new),
        alone.transform(new),
    )


@case
def cols_left_to_the_dtype():
    X, y = frame()
    passed_through = make_column_transformer((ce.BinaryEncoder(), ["zip"]), remainder="drop")  # xpd: cols-left-to-dtype
    named = ColumnTransformer([("zip", ce.OneHotEncoder(use_cat_names=True, handle_unknown="indicator"), ["zip"])])  # xpd: cols-left-to-dtype
    every = make_column_transformer((ce.BinaryEncoder(cols="all"), ["zip"]), remainder="drop")
    of_sklearn = make_column_transformer((OneHotEncoder(), ["zip"]), remainder="drop")
    outside = ce.BinaryEncoder().fit(X)
    return passed_through.fit_transform(X).shape, named.fit_transform(X).shape, every.fit_transform(X).shape, of_sklearn.fit_transform(X).shape, outside.cols


@case
def named_cols_after_an_array():
    X, y = frame()
    after_imputer = make_pipeline(SimpleImputer(strategy="most_frequent"), ce.BinaryEncoder(cols=["city"]))  # xpd: named-cols-after-array-step
    after_scaler = Pipeline([
        ("scale", make_column_transformer((StandardScaler(), ["x"]), remainder="passthrough")),
        ("encode", ce.CatBoostEncoder(cols=["city"])),  # xpd: named-cols-after-array-step
    ])
    first = make_pipeline(ce.BinaryEncoder(cols=["city"]), SimpleImputer(strategy="most_frequent"))
    by_position = make_pipeline(SimpleImputer(strategy="most_frequent"), ce.BinaryEncoder(cols=[0]))
    return attempt(lambda: after_imputer.fit(X, y)), attempt(lambda: after_scaler.fit(X, y)), first.fit_transform(X, y).shape, by_position.fit_transform(X, y).shape


@case
def encoders_that_sklearn_ships():
    X, y = frame()
    one_hot = ce.OneHotEncoder(cols=["city"])  # xpd: sklearn-ships-encoder
    ordinal = ce.OrdinalEncoder(cols=["city"], return_df=False)  # xpd: sklearn-ships-encoder
    target = ce.TargetEncoder(cols=["city"])  # xpd: sklearn-ships-encoder
    only_here = [
        ce.OneHotEncoder(cols=["city"], handle_missing="indicator"),
        ce.OrdinalEncoder(cols=["city"], mapping=[{"col": "city", "mapping": {"berlin": 1, "paris": 2, "rome": 3, "oslo": 4}}], handle_unknown="error"),
        ce.TargetEncoder(cols=["city"], smoothing=5.0),
        ce.BinaryEncoder(cols=["city"]),
    ]
    of_sklearn = OneHotEncoder(sparse_output=False).fit_transform(X[["city"]])
    return one_hot.fit_transform(X), ordinal.fit_transform(X), target.fit_transform(X, y), [e.fit_transform(X, y) for e in only_here], of_sklearn
