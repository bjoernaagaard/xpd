"""Fit one linear model per group for 24 groups of made up rows."""

import numpy as np
import pandas as pd
from sklearn.linear_model import LinearRegression
from sklego.meta import GroupedPredictor

X = pd.DataFrame({"group": np.repeat(np.arange(24), 8), "x": np.tile(np.arange(8), 24)})
y = X["group"] * 10 + X["x"] * 2
model = GroupedPredictor(estimator=LinearRegression(), groups=["group"], use_global_model=False)
model.fit(X, y)
result = pd.Series(model.predict(X), index=X.index)
print(
    f"groups={X['group'].nunique()}, models={len(model.estimators_)}, "
    f"predictions={len(result)}, "
    f"max_error={abs(result - y).max():.6f}"
)
