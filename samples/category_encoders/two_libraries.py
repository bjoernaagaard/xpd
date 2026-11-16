"""A file that imports OneHotEncoder from category_encoders and from scikit-learn.

Python keeps the last import, and xpd cannot tell which one a call reaches, so no rule that
needs the category_encoders class reports here. The names that only category_encoders has
are still reported, and the line ends with a comment that names the rule.
"""

import pandas as pd
from category_encoders import HashingEncoder, OneHotEncoder, TargetEncoder
from sklearn.compose import make_column_transformer
from sklearn.preprocessing import OneHotEncoder, TargetEncoder

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def last_import_wins():
    X = pd.DataFrame({"city": list("abcabcab"), "zip": [1, 2, 3, 1, 2, 3, 1, 2]})
    y = pd.Series([0, 1, 0, 1, 1, 0, 1, 0])
    ignoring = OneHotEncoder(handle_unknown="ignore", sparse_output=False).fit(X[["city"]])
    in_columns = make_column_transformer((OneHotEncoder(), ["zip"]), remainder="drop").fit_transform(X)
    target = TargetEncoder(smooth=2.0).fit(X[["city"]], y)
    hashed = HashingEncoder(cols=["city"], max_process=1)  # xpd: hashing-default-components
    return ignoring.categories_, in_columns.shape, target.categories_, hashed.fit_transform(X).shape
