"""Deprecated forms that category_encoders 2.11.2 still takes, next to the forms that replace them.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file. This file
imports no scikit-learn, because the scikit-learn pack reports get_feature_names itself.
"""

import warnings

import category_encoders as ce
import pandas as pd

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def frame():
    return pd.DataFrame({"city": list("abcabcab"), "shop": list("xyxyxyxy"), "n": [1, 2, 3, 1, 2, 3, 1, 2]})


@case
def feature_names():
    warnings.simplefilter("ignore", FutureWarning)
    X = frame()
    encoder = ce.OneHotEncoder(cols=["city"], use_cat_names=True, handle_unknown="indicator").fit(X)
    old = encoder.get_feature_names()  # xpd: deprecated-in-category-encoders
    inline = ce.BinaryEncoder(cols=["city"]).fit(X).get_feature_names()  # xpd: deprecated-in-category-encoders
    new = encoder.get_feature_names_out()
    return old, inline, new


def other_object(report):
    """`report` is built from no encoder in this file, so the call is left alone."""
    return report.get_feature_names()


@case
def hashing_max_sample():
    X = frame()
    old = ce.HashingEncoder(cols=["city"], max_process=1, max_sample=10, n_components=16)  # xpd: deprecated-in-category-encoders
    only = ce.HashingEncoder(max_sample=10, n_components=16)  # xpd: deprecated-in-category-encoders
    new = ce.HashingEncoder(cols=["city"], max_process=1, n_components=16)
    return old.fit_transform(X), only.get_params()["n_components"], new.fit_transform(X)
