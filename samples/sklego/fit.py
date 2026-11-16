"""scikit-lego steps that read the wrong rows, cannot be fitted, or do slow work, next to the
forms that work.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes. A form that raises is built through `attempt`, which returns the error's name.
"""

import logging
import warnings

import numpy as np
import pandas as pd
from sklearn.cluster import KMeans
from sklearn.compose import ColumnTransformer
from sklearn.datasets import make_regression
from sklearn.ensemble import IsolationForest, RandomForestClassifier, RandomForestRegressor
from sklearn.linear_model import LinearRegression, LogisticRegression, Ridge
from sklearn.model_selection import train_test_split
from sklearn.neighbors import KNeighborsRegressor
from sklearn.pipeline import Pipeline, make_pipeline
from sklearn.preprocessing import StandardScaler
from sklego import pandas_utils
from sklego.dummy import RandomRegressor
from sklego.meta import DecayEstimator, ZeroInflatedRegressor
from sklego.model_selection import GroupTimeSeriesSplit
from sklego.pandas_utils import add_lags, log_step, log_step_extra
from sklego.pipeline import DebugPipeline, make_debug_pipeline
from sklego.preprocessing import ColumnDropper, ColumnSelector, OutlierRemover, RandomAdder, TypeSelector

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def attempt(build):
    try:
        return build()
    except Exception as error:
        return type(error).__name__


def data():
    warnings.simplefilter("ignore")
    return make_regression(n_samples=200, n_features=4, noise=1.0, random_state=0)


@case
def lagged_columns():
    frame = pd.DataFrame({"day": range(1, 9), "sales": range(10, 90, 10)})
    array = frame.to_numpy()
    next_row = add_lags(frame, "sales", 1)  # xpd: add-lags-positive
    next_rows = add_lags(frame, "sales", [1, 2])  # xpd: add-lags-positive
    mixed = add_lags(array, 1, [-1, 1])  # xpd: add-lags-positive
    named = pandas_utils.add_lags(frame, cols=["sales"], lags=[3], drop_na=False)  # xpd: add-lags-positive
    ranged = add_lags(frame, "sales", list(range(1, 3)))  # xpd: add-lags-positive
    row_before = add_lags(frame, "sales", -1)
    rows_before = add_lags(frame, "sales", [-1, -2])
    steps = [-1, -7]
    from_name = add_lags(frame, "sales", steps)
    return next_row, next_rows, mixed, named, ranged, row_before, rows_before, from_name


@case
def removed_outliers():
    X, y = data()
    regression = Pipeline([("remove", OutlierRemover(IsolationForest(random_state=0), refit=True)), ("model", LinearRegression())])  # xpd: outlier-remover-before-model
    forest = make_pipeline(StandardScaler(), OutlierRemover(IsolationForest(random_state=0)), RandomForestClassifier(n_estimators=5, random_state=0))  # xpd: outlier-remover-before-model
    clusters = make_pipeline(OutlierRemover(IsolationForest(random_state=0), refit=True), KMeans(2, n_init=1, random_state=0)).fit(X)
    alone = OutlierRemover(IsolationForest(random_state=0), refit=True).fit(X)
    mask = IsolationForest(random_state=0).fit_predict(X) != -1
    masked = LinearRegression().fit(X[mask], y[mask])
    return attempt(lambda: regression.fit(X, y)), forest, clusters[-1].labels_, alone.transform(X).shape, masked.coef_


@case
def decayed_rows():
    X, y = data()
    X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=0)  # xpd: decay-on-shuffled-rows
    X_a, X_b, y_a, y_b = train_test_split(X, y, test_size=0.3, shuffle=True)  # xpd: decay-on-shuffled-rows
    X_old, X_new, y_old, y_new = train_test_split(X, y, shuffle=False)
    shuffled = DecayEstimator(LinearRegression(), decay_kwargs={"decay_rate": 0.99}).fit(X_train, y_train)
    ordered = DecayEstimator(LinearRegression(), decay_kwargs={"decay_rate": 0.99}).fit(X_old, y_old)
    return shuffled.predict(X_test), ordered.predict(X_new), X_a.shape, X_b.shape, y_a.shape, y_b.shape, y_test.shape, y_new.shape


@case
def decayed_models():
    X, y = data()
    piped = attempt(lambda: DecayEstimator(make_pipeline(StandardScaler(), Ridge())).fit(X, y))  # xpd: decay-without-sample-weight
    named = attempt(lambda: DecayEstimator(model=Pipeline([("scale", StandardScaler()), ("model", Ridge())])).fit(X, y))  # xpd: decay-without-sample-weight
    neighbours = attempt(lambda: DecayEstimator(KNeighborsRegressor()).fit(X, y))  # xpd: decay-without-sample-weight
    inside = make_pipeline(StandardScaler(), DecayEstimator(Ridge())).fit(X, y)
    weighted = DecayEstimator(RandomForestRegressor(n_estimators=5, random_state=0)).fit(X, y)
    return piped, named, neighbours, inside.predict(X), weighted.predict(X)


@case
def zero_inflated():
    X, y = data()
    y = np.maximum(y, 0)
    swapped = attempt(lambda: ZeroInflatedRegressor(LinearRegression(), LogisticRegression()).fit(X, y))  # xpd: zero-inflated-roles, zero-inflated-roles
    two_regressors = attempt(lambda: ZeroInflatedRegressor(classifier=RandomForestRegressor(n_estimators=5, random_state=0), regressor=Ridge()).fit(X, y))  # xpd: zero-inflated-roles
    two_classifiers = attempt(lambda: ZeroInflatedRegressor(LogisticRegression(), regressor=RandomForestClassifier(n_estimators=5, random_state=0)).fit(X, y))  # xpd: zero-inflated-roles
    right = ZeroInflatedRegressor(LogisticRegression(), LinearRegression()).fit(X, y)
    named = ZeroInflatedRegressor(classifier=RandomForestClassifier(n_estimators=5, random_state=0), regressor=Ridge()).fit(X, y)
    return swapped, two_regressors, two_classifiers, right.predict(X), named.predict(X)


@case
def grouped_time_folds():
    groups = np.sort(np.random.default_rng(0).integers(0, 12, size=300))
    X = np.zeros((300, 1))
    many = GroupTimeSeriesSplit(n_splits=6).split(X, None, groups)  # xpd: group-time-series-split-folds
    more = GroupTimeSeriesSplit(10)  # xpd: group-time-series-split-folds
    few = GroupTimeSeriesSplit(n_splits=3).split(X, None, groups)
    return len(many), more.n_splits, len(few)


@case
def selected_columns():
    X, _ = data()
    frame = pd.DataFrame(X, columns=["a", "b", "c", "d"])
    chained = attempt(lambda: make_pipeline(ColumnSelector(["a", "b"]), StandardScaler()).set_output(transform="pandas"))  # xpd: selector-without-feature-names
    pipe = make_pipeline(ColumnDropper(["d"]), StandardScaler())
    names = attempt(lambda: pipe.fit(frame).get_feature_names_out())  # xpd: selector-without-feature-names
    typed = Pipeline([("pick", TypeSelector(include="number")), ("scale", StandardScaler())])
    output = attempt(lambda: typed.set_output(transform="pandas"))  # xpd: selector-without-feature-names
    plain = make_pipeline(StandardScaler(), Ridge())
    plain_names = plain[:-1].set_output(transform="pandas").fit(frame).get_feature_names_out()
    passthrough = make_pipeline(ColumnTransformer([("keep", "passthrough", ["a", "b"])]), StandardScaler()).set_output(transform="pandas")
    return chained, names, output, plain_names, passthrough.fit_transform(frame), pipe.fit_transform(frame)


@case
def logged_steps():
    X, y = data()
    shown = []

    @log_step_extra  # xpd: log-step-extra-bare
    def bare(frame):
        return frame.assign(b=1)

    @log_step_extra(len, print_fn=shown.append)
    def counted(frame):
        return frame.assign(b=1)

    @log_step(print_fn=shown.append)
    def step(frame):
        return frame.assign(b=1)

    @log_step
    def bare_step(frame):
        return frame

    empty = attempt(lambda: log_step_extra())  # xpd: log-step-extra-bare
    frame = pd.DataFrame({"a": [1, 2]})
    return callable(bare(frame)), counted(frame), step(frame), bare_step(frame).shape, empty, len(shown)


@case
def unseeded():
    X, y = data()
    noise = RandomRegressor().fit(X, y)  # xpd: unseeded-random-estimator
    normal = RandomRegressor(strategy="normal").fit(X, y)  # xpd: unseeded-random-estimator
    adder = RandomAdder(noise=2).fit(X, y)  # xpd: unseeded-random-estimator
    seeded = RandomRegressor(random_state=0).fit(X, y)
    positional = RandomRegressor("normal", 0).fit(X, y)
    seeded_adder = RandomAdder(random_state=0).fit(X, y)
    return noise.predict(X).shape, normal.predict(X).shape, adder.transform(X).shape, seeded.predict(X), positional.predict(X), seeded_adder.transform(X)


def configured_logging(X, y):
    logging.basicConfig(level=logging.INFO)
    logged = DebugPipeline([("scale", StandardScaler()), ("model", LinearRegression())], log_callback="default")
    made = make_debug_pipeline(StandardScaler(), LinearRegression(), log_callback="default")
    return logged.fit(X, y), made.fit(X, y)
