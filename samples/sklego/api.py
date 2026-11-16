"""Forms that scikit-lego 0.9.10 warns about or has removed, next to the forms that replace them.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file. A removed
form raises, so it sits in a function that no case calls.
"""

import warnings
from datetime import timedelta

import numpy as np
import pandas as pd
from sklearn.cluster import KMeans
from sklearn.datasets import make_classification, make_regression
from sklearn.linear_model import LinearRegression, QuantileRegressor
from sklearn.preprocessing import FunctionTransformer, SplineTransformer
from sklego.dummy import RandomRegressor
from sklego.linear_model import DeadZoneRegressor, DemographicParityClassifier, EqualOpportunityClassifier, LADRegression, QuantileRegression
from sklego.meta import DecayEstimator, HierarchicalRegressor
from sklego.mixture import GMMOutlierDetector
from sklego.model_selection import ClusterFoldValidation, TimeGapSplit
from sklego.model_selection import KlusterFoldValidation  # xpd: deprecated-in-sklego
from sklego.naive_bayes import GaussianMixtureNB
from sklego.preprocessing import ColumnCapper, DictMapper, IdentityTransformer, IntervalEncoder, RandomAdder, RepeatingBasisFunction, TypeSelector
from sklego.preprocessing import PandasTypeSelector  # xpd: deprecated-in-sklego

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def data():
    warnings.simplefilter("ignore")
    return make_classification(n_samples=120, n_features=6, random_state=0)


def reg_data():
    warnings.simplefilter("ignore")
    return make_regression(n_samples=120, n_features=6, noise=1.0, random_state=0)


def folds(splitter, X):
    return [(train.tolist(), test.tolist()) for train, test in splitter.split(X)]


@case
def cluster_folds():
    X, _ = data()
    old = KlusterFoldValidation(cluster_method=KMeans(3, n_init=1, random_state=0))  # xpd: deprecated-in-sklego
    new = ClusterFoldValidation(cluster_method=KMeans(3, n_init=1, random_state=0))
    return folds(old, X), folds(new, X)


@case
def type_selector():
    warnings.simplefilter("ignore")
    frame = pd.DataFrame({"a": [1, 2, 3], "b": ["x", "y", "z"], "c": [0.5, 1.5, 2.5]})
    old = PandasTypeSelector(include="number").fit(frame)  # xpd: deprecated-in-sklego
    new = TypeSelector(include="number").fit(frame)
    return old.transform(frame), new.transform(frame), old.feature_names_


@case
def time_gap():
    warnings.simplefilter("ignore")
    dates = pd.Series(pd.date_range("2024-01-01", periods=40, freq="D"))
    frame = pd.DataFrame({"x": range(40)})
    old = TimeGapSplit(date_serie=dates, train_duration=timedelta(days=10), valid_duration=timedelta(days=5))  # xpd: deprecated-in-sklego
    new = TimeGapSplit(date_series=dates, train_duration=timedelta(days=10), valid_duration=timedelta(days=5))
    return folds(old, frame), folds(new, frame)


@case
def renamed_attributes():
    X, y = reg_data()
    Xc, yc = data()
    adder = RandomAdder(random_state=0).fit(X, y)
    noise = RandomRegressor(random_state=0).fit(X, y)
    mapper = DictMapper({1: 2}, 0).fit(np.array([[1], [3]]))
    capper = ColumnCapper().fit(X)
    encoder = IntervalEncoder(n_chunks=5).fit(X, y)
    bayes = GaussianMixtureNB(random_state=0).fit(Xc, yc)
    zone = DeadZoneRegressor().fit(X, y)
    frame = pd.DataFrame(X[:, :2], columns=["a", "b"]).assign(g=np.arange(len(X)) % 3)
    levels = HierarchicalRegressor(LinearRegression(), groups=["g"]).fit(frame, y)
    old = (
        adder.dim_,  # xpd: deprecated-in-sklego
        noise.dim_,  # xpd: deprecated-in-sklego
        mapper.dim_,  # xpd: deprecated-in-sklego
        capper.n_columns_,  # xpd: deprecated-in-sklego
        encoder.num_cols_,  # xpd: deprecated-in-sklego
        bayes.num_fit_cols_,  # xpd: deprecated-in-sklego
        zone.coefs_,  # xpd: deprecated-in-sklego
        levels.n_levels_,  # xpd: deprecated-in-sklego
    )
    new = (adder.n_features_in_, capper.n_features_in_, zone.coef_, levels.n_fitted_levels_)
    return old, new


@case
def allowed_values():
    warnings.simplefilter("ignore")
    old = (
        RandomRegressor().allowed_strategies,  # xpd: deprecated-in-sklego, unseeded-random-estimator
        IntervalEncoder().allowed_methods,  # xpd: deprecated-in-sklego
        GMMOutlierDetector().allowed_methods,  # xpd: deprecated-in-sklego
        DeadZoneRegressor().allowed_effects,  # xpd: deprecated-in-sklego
    )
    new = ("uniform", "normal")
    return old, new


class OwnEstimator:
    """A user's class may have attributes of the old names, which are not scikit-lego's."""

    def fit(self, X):
        self.dim_ = X.shape[1]
        self.n_levels_ = 2
        return self.dim_ + self.n_levels_


@case
def own_attributes():
    own = OwnEstimator()
    own.fit(np.zeros((4, 3)))
    return own.dim_, own.n_levels_


@case
def fair_penalty():
    warnings.simplefilter("ignore")
    X, y = data()
    old = DemographicParityClassifier(covariance_threshold=0.5, sensitive_cols=[0], penalty="none").fit(X, y)  # xpd: deprecated-in-sklego, superseded-estimator
    new = DemographicParityClassifier(covariance_threshold=0.5, sensitive_cols=[0], penalty=None).fit(X, y)  # xpd: superseded-estimator
    equal = EqualOpportunityClassifier(covariance_threshold=0.5, positive_target=1, sensitive_cols=[0], penalty="none").fit(X, y)  # xpd: deprecated-in-sklego, superseded-estimator
    return old.estimators_[0].coef_, old.predict(X), new.estimators_[0].coef_, equal.estimators_[0].coef_, equal.predict(X)


@case
def superseded():
    warnings.simplefilter("ignore")
    X, y = reg_data()
    quantile = QuantileRegression(quantile=0.8).fit(X, y)  # xpd: superseded-estimator
    median = LADRegression().fit(X, y)  # xpd: superseded-estimator
    basis = RepeatingBasisFunction(n_periods=4, column=0, input_range=(-3, 3)).fit(X)  # xpd: superseded-estimator
    same = IdentityTransformer().fit(X)  # xpd: superseded-estimator
    checked = IdentityTransformer(check_X=True).fit(X)
    new_quantile = QuantileRegressor(quantile=0.8, alpha=0).fit(X, y)
    new_basis = SplineTransformer(n_knots=5, extrapolation="periodic").fit(X[:, [0]])
    new_same = FunctionTransformer().fit(X)
    return (quantile.coef_, median.coef_, basis.transform(X), same.transform(X), checked.transform(X), new_quantile.coef_,
            new_basis.transform(X[:, [0]]), new_same.transform(X))


def removed_names():
    from sklego.preprocessing import PatsyTransformer  # xpd: removed-in-sklego
    from sklego.preprocessing.patsytransformer import PatsyTransformer as Patsy  # xpd: removed-in-sklego, removed-in-sklego
    from sklego.linear_model import FairClassifier  # xpd: removed-in-sklego
    from sklego.meta import GroupedEstimator  # xpd: removed-in-sklego
    from sklego.meta import OutlierRemover  # xpd: removed-in-sklego
    from sklego.meta import Thresholder, OutlierRemover as Remover  # xpd: removed-in-sklego
    from sklego.meta.outlier_remover import OutlierRemover as Old  # xpd: removed-in-sklego
    import sklego.meta.grouped_estimator  # xpd: removed-in-sklego
    fair = FairClassifier(covariance_threshold=0.5, sensitive_cols=[0])  # xpd: removed-in-sklego
    grouped = GroupedEstimator(LinearRegression(), groups=["g"])  # xpd: removed-in-sklego
    return PatsyTransformer, Patsy, fair, grouped, OutlierRemover, Thresholder, Remover, Old, sklego


def removed_keywords(X, y):
    from sklego.datasets import load_chicken, make_simpleseries
    frame = load_chicken(give_pandas=True)  # xpd: removed-in-sklego
    series = make_simpleseries(give_pandas=True)  # xpd: removed-in-sklego
    model = DecayEstimator(LinearRegression(), decay=0.99).fit(X, y)  # xpd: removed-in-sklego
    linear = DecayEstimator(LinearRegression(), decay=0.99, decay_func="linear").fit(X, y)  # xpd: removed-in-sklego
    new = DecayEstimator(LinearRegression(), decay_kwargs={"decay_rate": 0.99}).fit(X, y)
    return frame, series, model, linear, new


def compat_paths():
    try:
        from sklego.preprocessing import FormulaicTransformer as Formula
    except ImportError:
        from sklego.preprocessing import PatsyTransformer as Formula
    try:
        from sklego.meta import GroupedPredictor
    except ImportError:
        from sklego.meta import GroupedEstimator as GroupedPredictor
    return Formula, GroupedPredictor
