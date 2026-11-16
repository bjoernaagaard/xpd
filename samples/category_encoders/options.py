"""Encoder arguments that give codes the reader does not expect, next to the forms that state them.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes.
"""

import category_encoders as ce
import numpy as np
import pandas as pd
from category_encoders.wrapper import NestedCVWrapper
from sklearn.model_selection import KFold

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def frame(n=200, seed=0):
    r = np.random.default_rng(seed)
    X = pd.DataFrame({"city": r.choice(["berlin", "paris", "rome", "oslo"], n), "shop": r.choice(list("xyz"), n)})
    return X, pd.Series(r.integers(0, 2, n))


@case
def ordinal_mapping():
    X, _ = frame()
    codes = {"berlin": 1, "paris": 2, "rome": 3}
    silent = ce.OrdinalEncoder(cols=["city"], mapping=[{"col": "city", "mapping": codes}])  # xpd: ordinal-mapping-unknown
    strict = ce.OrdinalEncoder(cols=["city"], mapping=[{"col": "city", "mapping": {**codes, "oslo": 4}}], handle_unknown="error")
    stated = ce.OrdinalEncoder(cols=["city"], mapping=[{"col": "city", "mapping": codes}], handle_unknown="value")
    learned = ce.OrdinalEncoder(cols=["city"], handle_unknown="error")
    return [e.fit_transform(X)["city"].min() for e in (silent, strict, stated, learned)]


@case
def hashing_components():
    X, _ = frame()
    default = ce.HashingEncoder(cols=["city", "shop"], max_process=1)  # xpd: hashing-default-components
    bare = ce.HashingEncoder()  # xpd: hashing-default-components
    stated = ce.HashingEncoder(cols=["city", "shop"], max_process=1, n_components=64)
    positional = ce.HashingEncoder(1, 0, 0, 32)
    return default.fit_transform(X).shape, bare.n_components, stated.fit_transform(X).shape, positional.n_components, isinstance(stated, ce.HashingEncoder)


@case
def zero_regularization():
    pure = pd.DataFrame({"city": ["berlin"] * 5 + ["paris"] * 5})
    y = pd.Series([1] * 5 + [0, 1, 0, 1, 0])
    unbounded = ce.WOEEncoder(cols=["city"], regularization=0).fit_transform(pure, y)  # xpd: zero-regularization
    bounded = ce.WOEEncoder(cols=["city"], regularization=0.5).fit_transform(pure, y)
    counted = ce.CountTargetEncoder(cols=["city"], smoothing=0)  # xpd: zero-regularization
    smoothed = ce.CountTargetEncoder(cols=["city"], smoothing=0.5).fit_transform(pure, y)
    return unbounded, bounded, counted.get_params(), smoothed


@case
def noise_without_seed():
    X, y = frame()
    encoders = [
        ce.WOEEncoder(cols=["city"], randomized=True),  # xpd: encoder-noise-without-seed
        ce.JamesSteinEncoder(cols=["city"], randomized=True, sigma=0.1),  # xpd: encoder-noise-without-seed
        ce.LeaveOneOutEncoder(cols=["city"], sigma=0.05),  # xpd: encoder-noise-without-seed
        ce.CatBoostEncoder(cols=["city"], sigma=0.05),  # xpd: encoder-noise-without-seed
        NestedCVWrapper(ce.MEstimateEncoder(cols=["city"])),  # xpd: encoder-noise-without-seed
        NestedCVWrapper(ce.MEstimateEncoder(cols=["city"]), cv=3),  # xpd: encoder-noise-without-seed
    ]
    seeded = [
        ce.WOEEncoder(cols=["city"], randomized=True, random_state=0),
        ce.WOEEncoder(cols=["city"], randomized=False),
        ce.LeaveOneOutEncoder(cols=["city"], sigma=0.05, random_state=0),
        ce.LeaveOneOutEncoder(cols=["city"], sigma=None),
        ce.CatBoostEncoder(cols=["city"]),
        NestedCVWrapper(ce.MEstimateEncoder(cols=["city"]), random_state=0),
        NestedCVWrapper(ce.MEstimateEncoder(cols=["city"]), shuffle=False),
        NestedCVWrapper(ce.MEstimateEncoder(cols=["city"]), cv=KFold(3)),
    ]
    return len(encoders), [e.fit_transform(X, y) for e in seeded]
