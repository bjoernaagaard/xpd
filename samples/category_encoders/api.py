"""Forms that category_encoders 2.11.2 raises on, next to the forms that replace them.

A line that xpd reports ends with a comment that names the rule. A removed form raises, so its
case builds it through `attempt`, which returns None when it raises.
"""

import category_encoders as ce
import pandas as pd
from category_encoders import utils
from category_encoders.wrapper import NestedCVWrapper, PolynomialWrapper
from sklearn.preprocessing import OneHotEncoder

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def frame():
    X = pd.DataFrame({"city": list("abcabcab"), "shop": list("xyxyxyxy"), "n": [1, 2, 3, 1, 2, 3, 1, 2]})
    return X, pd.Series([0, 1, 0, 1, 1, 0, 1, 0])


def attempt(build):
    """What build returns, or None when it raises, as a removed form does on 2.11.2."""
    try:
        return build()
    except (ImportError, TypeError, ValueError, AttributeError, KeyError):
        return None


@case
def removed_keywords():
    old = attempt(lambda: ce.OneHotEncoder(impute_missing=True))  # xpd: removed-in-category-encoders
    noise = attempt(lambda: ce.LeaveOneOutEncoder(randomized=True, sigma=0.05, random_state=0))  # xpd: removed-in-category-encoders
    new = ce.OneHotEncoder(handle_missing="value")
    seeded = ce.LeaveOneOutEncoder(sigma=0.05, random_state=0)
    woe = ce.WOEEncoder(randomized=True, random_state=0)
    return old, noise, new.get_params(), seeded.get_params(), woe.get_params()


@case
def both_releases():
    """Code in a try block with an except clause handles the error, so it is left alone."""
    try:
        encoder = ce.OneHotEncoder(impute_missing=True)
    except TypeError:
        encoder = ce.OneHotEncoder(handle_missing="value")
    try:
        cols = utils.get_obj_cols
    except AttributeError:
        cols = utils.get_categorical_cols
    return encoder.get_params(), cols.__name__


@case
def removed_helpers():
    X, _ = frame()
    old = attempt(lambda: utils.get_obj_cols(X))  # xpd: removed-in-category-encoders
    mixin = attempt(lambda: utils.TransformerWithTargetMixin)  # xpd: removed-in-category-encoders
    return old, mixin, utils.get_categorical_cols(X), utils.SupervisedTransformerMixin


@case
def intercept_column():
    X, _ = frame()
    coded = ce.HelmertEncoder(cols=["city"]).fit_transform(X)
    dropped = attempt(lambda: coded.drop(columns=["intercept"]))  # xpd: removed-in-category-encoders
    by_axis = attempt(lambda: coded.drop("intercept", axis=1))  # xpd: removed-in-category-encoders
    tolerant = coded.drop(columns=["intercept"], errors="ignore")
    other = coded.drop(columns=["n"])
    return dropped, by_axis, tolerant, other, coded


@case
def names_are_an_array():
    X, _ = frame()
    encoder = ce.BinaryEncoder(cols=["city"]).fit(X)
    old = attempt(lambda: encoder.feature_names_out_.index("n"))  # xpd: removed-in-category-encoders
    called = attempt(lambda: encoder.get_feature_names_out().index("n"))  # xpd: removed-in-category-encoders
    return old, called, list(encoder.feature_names_out_).index("n"), encoder.feature_names_in_.index("n")


@case
def handle_options():
    X, y = frame()
    ignore = attempt(lambda: ce.OneHotEncoder(cols=["city"], handle_unknown="ignore").fit(X))  # xpd: invalid-handle-option
    impute = attempt(lambda: ce.OrdinalEncoder(handle_unknown="impute").fit(X))  # xpd: invalid-handle-option
    indicator = attempt(lambda: ce.TargetEncoder(handle_missing="indicator").fit(X, y))  # xpd: invalid-handle-option
    woe = attempt(lambda: ce.WOEEncoder(handle_unknown="ignore").fit(X, y))  # xpd: invalid-handle-option
    binary = attempt(lambda: ce.BinaryEncoder(handle_missing="ignore").fit(X))  # xpd: invalid-handle-option
    rank = attempt(lambda: ce.RankHotEncoder(handle_unknown="indicator").fit(X))  # xpd: invalid-handle-option
    hashing = attempt(lambda: ce.HashingEncoder(handle_unknown="value", n_components=16))  # xpd: invalid-handle-option
    missing = attempt(lambda: ce.OneHotEncoder(handle_missing="impute").fit(X))  # xpd: invalid-handle-option
    of_sklearn = OneHotEncoder(handle_unknown="ignore").fit(X[["city"]])
    valid = [
        ce.OneHotEncoder(cols=["city"], handle_unknown="indicator", handle_missing="ignore").fit(X),
        ce.BinaryEncoder(handle_unknown="indicator").fit(X),
        ce.RankHotEncoder(handle_missing="ignore").fit(X),
        ce.OrdinalEncoder(handle_unknown="return_nan", handle_missing=-5).fit(X),
        ce.CountEncoder(handle_unknown=0).fit(X),
    ]
    return ignore, impute, indicator, woe, binary, rank, hashing, missing, of_sklearn.categories_, [v.get_feature_names_out() for v in valid]


@case
def supervised_without_target():
    X, y = frame()
    inline = attempt(lambda: ce.CatBoostEncoder(cols=["city"]).fit_transform(X))  # xpd: supervised-encoder-without-target
    target = attempt(lambda: ce.TargetEncoder(cols=["city"], smoothing=5.0).fit(X))  # xpd: supervised-encoder-without-target
    encoder = ce.JamesSteinEncoder(cols=["city"])
    named = attempt(lambda: encoder.fit_transform(X))  # xpd: supervised-encoder-without-target
    ordinal = ce.OrdinalEncoder(cols=["city"], handle_unknown="error").fit_transform(X)
    return inline, target, named, ordinal, encoder.fit_transform(X, y), ce.CatBoostEncoder(cols=["city"]).fit_transform(X, y)


@case
def wrapper_fit():
    X, _ = frame()
    y = pd.Series([0, 1, 2, 0, 1, 2, 0, 1])
    chained = attempt(lambda: PolynomialWrapper(ce.TargetEncoder(cols=["city"], smoothing=5.0)).fit(X, y).transform(X))  # xpd: wrapper-fit-returns-none
    fitted = NestedCVWrapper(ce.TargetEncoder(cols=["city"], smoothing=5.0), cv=2, random_state=0).fit(X, y)  # xpd: wrapper-fit-returns-none
    wrapper = PolynomialWrapper(ce.TargetEncoder(cols=["city"], smoothing=5.0))
    nothing = wrapper.fit(X, y)  # xpd: wrapper-fit-returns-none
    wrapper.fit(X, y)
    return chained, fitted, nothing, wrapper.transform(X), wrapper.fit_transform(X, y)


@case
def mapping_as_pairs():
    X, _ = frame()
    pairs = ce.OrdinalEncoder(cols=["city"], handle_unknown="value", mapping=[{"col": "city", "mapping": [("a", 1), ("b", 2), ("c", 3)]}])  # xpd: ordinal-mapping-pairs
    as_dict = ce.OrdinalEncoder(cols=["city"], handle_unknown="value", mapping=[{"col": "city", "mapping": {"a": 1, "b": 2, "c": 3}}])
    return pairs.fit_transform(X)["city"].tolist(), as_dict.fit_transform(X)["city"].tolist()
