"""scikit-lego arguments that give a wrong number or raise, next to the forms that work.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes. A form that raises is built through `attempt`, which returns the error's name.
"""

import warnings

import numpy as np
import pandas as pd
from sklearn.datasets import make_classification
from sklearn.ensemble import RandomForestClassifier, StackingClassifier
from sklearn.linear_model import LinearRegression, LogisticRegression
from sklearn.metrics import accuracy_score, make_scorer
from sklearn.model_selection import FixedThresholdClassifier, GridSearchCV, cross_val_score, train_test_split
from sklearn.pipeline import FeatureUnion, Pipeline, make_pipeline
from sklearn.preprocessing import MinMaxScaler, StandardScaler
from sklego import metrics
from sklego.datasets import load_arrests, load_penguins
from sklego.decomposition import PCAOutlierDetection, UMAPOutlierDetection
from sklego.meta import (EstimatorTransformer, GroupedClassifier, GroupedPredictor, GroupedRegressor, HierarchicalClassifier,
                         HierarchicalRegressor, RegressionOutlierDetector, Thresholder)
from sklego.metrics import correlation_score, equal_opportunity_score, p_percent_score, subset_score
from sklego.mixture import BayesianGMMOutlierDetector, GMMOutlierDetector
from sklego.preprocessing import ColumnCapper, DictMapper

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
    return make_classification(n_samples=300, n_features=6, weights=[0.8], random_state=0)


def grouped_frame():
    X, y = data()
    frame = pd.DataFrame(X[:, :3], columns=["a", "b", "c"])
    return frame.assign(h=np.arange(len(frame)) % 2, g=np.arange(len(frame)) % 4), y


@case
def thresholder_scores():
    X, y = data()
    X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=0)
    model = Thresholder(LogisticRegression(), threshold=0.1).fit(X_train, y_train)
    wrong = model.score(X_test, y_test)  # xpd: thresholder-score
    chained = Thresholder(LogisticRegression(), threshold=0.9).fit(X_train, y_train).score(X_test, y_test)  # xpd: thresholder-score
    right = accuracy_score(y_test, model.predict(X_test))
    fixed = FixedThresholdClassifier(LogisticRegression(), threshold=0.1, response_method="predict_proba").fit(X_train, y_train)
    return wrong, chained, right, fixed.score(X_test, y_test)


@case
def thresholder_search():
    X, y = data()
    grid = {"threshold": [0.1, 0.5, 0.9]}
    flat = GridSearchCV(Thresholder(LogisticRegression(), threshold=0.5), grid, cv=3).fit(X, y)  # xpd: thresholder-score
    piped = GridSearchCV(make_pipeline(StandardScaler(), Thresholder(LogisticRegression(), threshold=0.5)), {"thresholder__threshold": [0.1, 0.9]}, cv=3).fit(X, y)  # xpd: thresholder-score
    folds = cross_val_score(Thresholder(LogisticRegression(), threshold=0.1), X, y, cv=3)  # xpd: thresholder-score
    scored = GridSearchCV(Thresholder(LogisticRegression(), threshold=0.5), grid, cv=3, scoring="accuracy").fit(X, y)
    scored_folds = cross_val_score(Thresholder(LogisticRegression(), threshold=0.1), X, y, cv=3, scoring="accuracy")
    other = GridSearchCV(LogisticRegression(), {"C": [0.1, 1.0]}, cv=3).fit(X, y)
    return (flat.cv_results_["mean_test_score"], piped.cv_results_["mean_test_score"], folds, scored.cv_results_["mean_test_score"],
            scored_folds, other.best_score_)


@case
def proba_as_features():
    X, y = data()
    stacked = EstimatorTransformer(LogisticRegression(), predict_func="predict_proba").fit(X, y)  # xpd: estimator-transformer-proba
    logs = EstimatorTransformer(LogisticRegression(), "predict_log_proba").fit(X, y)  # xpd: estimator-transformer-proba
    union = FeatureUnion([("proba", EstimatorTransformer(RandomForestClassifier(n_estimators=5, random_state=0), predict_func="predict_proba")), ("scale", StandardScaler())])  # xpd: estimator-transformer-proba
    margin = EstimatorTransformer(LogisticRegression(), predict_func="decision_function").fit(X, y)
    labels = EstimatorTransformer(LogisticRegression()).fit(X, y)
    stack = StackingClassifier([("lr", LogisticRegression())], final_estimator=LogisticRegression(), stack_method="predict_proba", cv=3).fit(X, y)
    return (stacked.transform(X).shape, logs.transform(X).shape, attempt(lambda: union.fit_transform(X, y)), margin.transform(X).shape,
            labels.transform(X).shape, stack.transform(X).shape)


@case
def capped_columns():
    X = np.random.default_rng(0).normal(size=(1000, 2))
    fractions = ColumnCapper(quantile_range=(0.05, 0.95)).fit(X)  # xpd: column-capper-fraction
    listed = ColumnCapper(quantile_range=[0.01, 0.99]).fit(X)  # xpd: column-capper-fraction
    positional = ColumnCapper((0.1, 0.9)).fit(X)  # xpd: column-capper-fraction
    whole = ColumnCapper(quantile_range=(0, 1)).fit(X)  # xpd: column-capper-fraction
    percent = ColumnCapper(quantile_range=(5, 95)).fit(X)
    narrow = ColumnCapper(quantile_range=(0.5, 99.5)).fit(X)
    default = ColumnCapper().fit(X)
    return (fractions.transform(X), listed.transform(X), positional.transform(X), whole.transform(X), percent.transform(X),
            narrow.transform(X), default.transform(X))


@case
def mapped_values():
    X = np.array([["a"], ["b"], ["c"]])
    floats = DictMapper({"a": 0.5, "b": 1.7}, default=-1).fit(X)  # xpd: dict-mapper-not-int
    named = DictMapper(mapper={"a": 1, "b": 2}, default=0.5).fit(X)  # xpd: dict-mapper-not-int
    negative = DictMapper({"a": -0.25, "b": 2}, 0).fit(X)  # xpd: dict-mapper-not-int
    text = DictMapper({"a": "low", "b": "high"}, default="none").fit(X)  # xpd: dict-mapper-not-int
    missing = DictMapper({"a": 1, "b": 2}, default=np.nan).fit(X)  # xpd: dict-mapper-not-int
    whole = DictMapper({"a": 1, "b": 2}, default=-1).fit(X)
    whole_floats = DictMapper({"a": 1.0, "b": 2.0}, default=0).fit(X)
    mapping = {"a": 0.5}
    unseen = DictMapper(mapping, 0).fit(X)
    digits = DictMapper({"a": "1", "b": "2"}, default=0).fit(X)
    series = pd.Series(["a", "b", "c"]).map({"a": 0.5, "b": 1.7}).fillna(-1)
    return (floats.transform(X), named.transform(X), negative.transform(X), attempt(lambda: text.transform(X)),
            attempt(lambda: missing.transform(X)), whole.transform(X), whole_floats.transform(X), unseen.transform(X), series,
            digits.transform(X))


@case
def regression_outliers():
    rng = np.random.default_rng(0)
    frame = pd.DataFrame(rng.normal(size=(300, 3)), columns=["x1", "x2", "y"])
    frame["y"] = 2 * frame["x1"] + rng.normal(scale=0.5, size=300)
    frame.loc[:9, "y"] += 10
    by_name = RegressionOutlierDetector(LinearRegression(), column="y").fit(frame)  # xpd: regression-outlier-column-name
    positional = RegressionOutlierDetector(LinearRegression(), "y").fit(frame)  # xpd: regression-outlier-column-name
    by_position = RegressionOutlierDetector(LinearRegression(), column=2).fit(frame.to_numpy())
    return by_name.predict(frame), positional.predict(frame), by_position.predict(frame.to_numpy())


@case
def reconstruction_outliers():
    warnings.simplefilter("ignore")
    rng = np.random.default_rng(0)
    X = rng.normal(loc=10, size=(300, 1)) * np.ones((1, 5)) + rng.normal(scale=0.05, size=(300, 5))
    relative = make_pipeline(StandardScaler(), PCAOutlierDetection(n_components=1, threshold=0.05)).fit(X)  # xpd: pca-outlier-after-scaler
    steps = Pipeline([("scale", StandardScaler()), ("detect", UMAPOutlierDetection(n_components=2, threshold=0.1, random_state=0))])  # xpd: pca-outlier-after-scaler
    absolute = make_pipeline(StandardScaler(), PCAOutlierDetection(n_components=1, threshold=2.0, variant="absolute")).fit(X)
    uncentred = make_pipeline(StandardScaler(with_mean=False), PCAOutlierDetection(n_components=1, threshold=0.05)).fit(X)
    positive = make_pipeline(MinMaxScaler(), PCAOutlierDetection(n_components=1, threshold=0.05)).fit(X)
    raw = PCAOutlierDetection(n_components=1, threshold=0.05).fit(X)
    return relative.predict(X), steps, absolute.predict(X), uncentred.predict(X), positive.predict(X), raw.predict(X)


@case
def thresholds():
    X, _ = data()
    no_threshold = attempt(lambda: PCAOutlierDetection(n_components=2).fit(X))  # xpd: outlier-threshold
    no_threshold_umap = attempt(lambda: UMAPOutlierDetection().fit(X))  # xpd: outlier-threshold
    searched = GridSearchCV(PCAOutlierDetection(n_components=2), {"threshold": [0.1, 0.2]}, scoring=lambda est, X, y=None: 0.0, cv=2)
    second = PCAOutlierDetection(2, 0.1).fit(X)
    share = GMMOutlierDetector(threshold=0.05, random_state=0).fit(X)  # xpd: outlier-threshold
    bayes = BayesianGMMOutlierDetector(0.01, random_state=0).fit(X)  # xpd: outlier-threshold
    large = attempt(lambda: GMMOutlierDetector(threshold=2).fit(X))  # xpd: outlier-threshold
    larger = attempt(lambda: BayesianGMMOutlierDetector(threshold=1.5).fit(X))  # xpd: outlier-threshold
    kept = GMMOutlierDetector(threshold=0.95, random_state=0).fit(X)
    default = GMMOutlierDetector(random_state=0).fit(X)
    deviations = GMMOutlierDetector(threshold=2, method="stddev", random_state=0).fit(X)
    small_deviations = GMMOutlierDetector(threshold=0.4, method="stddev", random_state=0).fit(X)
    method = "stddev"
    by_name = GMMOutlierDetector(threshold=0.4, method=method, random_state=0).fit(X)
    half = GMMOutlierDetector(threshold=0.5, random_state=0).fit(X)
    return (no_threshold, no_threshold_umap, searched, second.predict(X), share.predict(X), bayes.predict(X), large, larger,
            kept.predict(X), default.predict(X), deviations.predict(X), small_deviations.predict(X), by_name.predict(X),
            half.predict(X))


@case
def shrinkage():
    frame, y = grouped_frame()
    target = frame["a"] * 2 + frame["g"]
    dropped = GroupedClassifier(LogisticRegression(), groups=["h", "g"], shrinkage="relative").fit(frame, y)  # xpd: grouped-shrinkage-arguments
    dropped_setting = GroupedClassifier(LogisticRegression(), ["h", "g"], alpha=0.1).fit(frame, y)  # xpd: grouped-shrinkage-arguments
    plain = GroupedClassifier(estimator=LogisticRegression(), groups=["h", "g"], use_global_model=True, check_X=True).fit(frame, y)
    shrunk = HierarchicalClassifier(LogisticRegression(), groups=["h", "g"], shrinkage="relative").fit(frame, y)
    no_setting = attempt(lambda: GroupedRegressor(LinearRegression(), groups=["h", "g"], shrinkage="constant").fit(frame, target))  # xpd: grouped-shrinkage-arguments
    no_count = attempt(lambda: HierarchicalRegressor(LinearRegression(), groups=["h", "g"], shrinkage="min_n_obs").fit(frame, target))  # xpd: grouped-shrinkage-arguments
    bare = attempt(lambda: GroupedPredictor(LinearRegression(), groups=["h", "g"], shrinkage="constant", alpha=0.1))  # xpd: grouped-shrinkage-arguments
    bare_count = attempt(lambda: HierarchicalRegressor(LinearRegression(), groups=["h", "g"], shrinkage="min_n_obs", min_n_obs=20))  # xpd: grouped-shrinkage-arguments
    setting = GroupedPredictor(LinearRegression(), groups=["h", "g"], shrinkage="constant", shrinkage_kwargs={"alpha": 0.1}).fit(frame, target)
    relative = GroupedRegressor(LinearRegression(), groups=["h", "g"], shrinkage="relative").fit(frame, target)
    return (dropped.predict_proba(frame), dropped_setting.predict_proba(frame), plain.predict_proba(frame), shrunk.predict_proba(frame),
            no_setting, no_count, bare, bare_count, setting.predict(frame), relative.predict(frame))


@case
def fairness_scores():
    X, y = data()
    frame = pd.DataFrame(X, columns=list("abcdef")).assign(s=(X[:, 0] > 0).astype(int))
    model = LogisticRegression().fit(frame, y)
    y_true, y_pred = y, model.predict(frame)
    wrapped = attempt(lambda: make_scorer(p_percent_score("s"))(model, frame, y))  # xpd: scorer-called-as-metric
    wrapped_kw = attempt(lambda: make_scorer(equal_opportunity_score("s"), greater_is_better=True)(model, frame, y))  # xpd: scorer-called-as-metric
    wrapped_module = attempt(lambda: make_scorer(metrics.correlation_score("s"))(model, frame, y))  # xpd: scorer-called-as-metric
    wrapped_subset = attempt(lambda: make_scorer(subset_score(lambda X, y: X["s"] == 1, accuracy_score))(model, frame, y))  # xpd: scorer-called-as-metric
    as_metric = p_percent_score(y, y_pred)  # xpd: scorer-called-as-metric
    as_metric_too = metrics.equal_opportunity_score(y_true, y_pred)  # xpd: scorer-called-as-metric
    search = GridSearchCV(LogisticRegression(), {"C": [1.0]}, scoring=p_percent_score("s"), cv=3).fit(frame, y)
    right = p_percent_score("s")(model, frame, y)
    labelled = p_percent_score("s", 1)(model, frame)
    correlation = correlation_score("a")(model, frame)
    plain = make_scorer(accuracy_score)(model, frame, y)
    return wrapped, wrapped_kw, wrapped_module, wrapped_subset, callable(as_metric), callable(as_metric_too), search.best_score_, right, labelled, correlation, plain


def unpack(pair):
    X, y = pair
    return X.shape, y.shape


@case
def datasets():
    both = attempt(lambda: unpack(load_penguins(return_X_y=True, as_frame=True)))  # xpd: dataset-frame-and-xy
    frame = load_arrests(as_frame=True, return_X_y=True)  # xpd: dataset-frame-and-xy
    X, y = load_penguins(return_X_y=True)
    whole = load_penguins(as_frame=True)
    explicit = load_arrests(return_X_y=True, as_frame=False)
    return both, frame.shape, X.shape, y.shape, whole.shape, explicit[0].shape
