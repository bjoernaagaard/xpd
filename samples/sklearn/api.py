"""Forms that scikit-learn 1.9.1 warns about or has removed, next to the forms that replace them.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes. A fix that drops a keyword changes the parameter the estimator stores, so those
cases return the fitted attributes and the predictions, which must stay bit equal. A removed
form raises, so its case builds it through `attempt`, which returns None when it raises.
"""

import warnings

import numpy as np
from sklearn.cluster import HDBSCAN
from sklearn.datasets import make_classification, make_regression
from sklearn.ensemble import GradientBoostingClassifier, GradientBoostingRegressor, RandomForestRegressor
from sklearn.linear_model import LogisticRegression, LogisticRegressionCV, PassiveAggressiveClassifier, SGDClassifier, SGDRegressor, lasso_path
from sklearn.manifold import MDS
from sklearn.model_selection import KFold
from sklearn.preprocessing import TargetEncoder
from sklearn.svm import SVC, NuSVC
from sklearn.tree import DecisionTreeRegressor

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def data():
    warnings.simplefilter("ignore", FutureWarning)
    return make_classification(n_samples=120, n_features=6, random_state=0)


def reg_data():
    warnings.simplefilter("ignore", FutureWarning)
    return make_regression(n_samples=120, n_features=6, noise=1.0, random_state=0)


def fitted(model):
    return {k: v for k, v in vars(model).items() if k.endswith("_") or k.startswith("_effective")}


@case
def svc_probability():
    X, y = data()
    model = SVC(probability=True, random_state=0).fit(X, y)  # xpd: deprecated-in-sklearn
    nu = NuSVC(probability=True, random_state=0).fit(X, y)  # xpd: deprecated-in-sklearn
    plain = SVC(probability=False).fit(X, y)  # xpd: deprecated-in-sklearn
    silent = SVC(C=2.0).fit(X, y)
    return fitted(plain), plain.predict(X), model.predict(X), nu.predict(X), silent.predict(X)


def svc_platt(model):
    return model.probA_, model.probB_  # xpd: deprecated-in-sklearn, deprecated-in-sklearn


@case
def logistic_penalty():
    X, y = data()
    l2 = LogisticRegression(penalty="l2", C=0.5).fit(X, y)  # xpd: deprecated-in-sklearn
    l1 = LogisticRegression(penalty="l1", solver="liblinear", random_state=0).fit(X, y)  # xpd: deprecated-in-sklearn
    net = LogisticRegression(penalty="elasticnet", l1_ratio=0.5, solver="saga", max_iter=3000, random_state=0).fit(X, y)  # xpd: deprecated-in-sklearn
    none = LogisticRegression(l1_ratio=None).fit(X, y)  # xpd: deprecated-in-sklearn
    jobs = LogisticRegression(n_jobs=-1, C=0.5).fit(X, y)  # xpd: deprecated-in-sklearn
    saga = LogisticRegression(solver="saga", penalty="l1", max_iter=3000, random_state=0).fit(X, y)  # xpd: deprecated-in-sklearn
    silent = LogisticRegression(l1_ratio=1, solver="liblinear", C=0.5).fit(X, y)
    return (fitted(l2), l2.predict_proba(X), fitted(l1), l1.predict_proba(X), fitted(net), net.predict_proba(X),
            fitted(none), none.predict_proba(X), fitted(jobs), jobs.predict_proba(X), fitted(saga), saga.predict_proba(X), silent.predict(X))


def logistic_penalty_other(X, y, kind):
    LogisticRegression(penalty=None).fit(X, y)  # xpd: deprecated-in-sklearn
    LogisticRegression(penalty=kind, l1_ratio=0.3).fit(X, y)  # xpd: deprecated-in-sklearn
    LogisticRegression(C=np.inf).fit(X, y)


@case
def logistic_cv():
    X, y = data()
    old = LogisticRegressionCV(cv=3)  # xpd: deprecated-in-sklearn
    penalty = LogisticRegressionCV(penalty="l1", solver="liblinear", l1_ratios=(1,), scoring="accuracy", use_legacy_attributes=False)  # xpd: deprecated-in-sklearn
    model = LogisticRegressionCV(cv=3, l1_ratios=(0,), scoring="accuracy", use_legacy_attributes=False).fit(X, y)
    return model.predict(X), old.cv, penalty.solver


@case
def friedman_mse():
    X, y = reg_data()
    forest = RandomForestRegressor(n_estimators=5, criterion="friedman_mse", random_state=0).fit(X, y)  # xpd: deprecated-in-sklearn
    tree = DecisionTreeRegressor(criterion='friedman_mse', random_state=0).fit(X, y)  # xpd: deprecated-in-sklearn
    silent = DecisionTreeRegressor(criterion="absolute_error", random_state=0).fit(X, y)
    return forest, forest.predict(X), tree, tree.predict(X), silent.predict(X)


@case
def boosting_criterion():
    X, y = data()
    Xr, yr = reg_data()
    clf = GradientBoostingClassifier(n_estimators=10, criterion="friedman_mse", random_state=0).fit(X, y)  # xpd: deprecated-in-sklearn
    reg = GradientBoostingRegressor(criterion="squared_error", n_estimators=10, random_state=0).fit(Xr, yr)  # xpd: deprecated-in-sklearn
    silent = GradientBoostingRegressor(n_estimators=10, loss="absolute_error", random_state=0).fit(Xr, yr)
    return fitted(clf), clf.predict_proba(X), fitted(reg), reg.predict(Xr), silent.predict(Xr)


@case
def passive_aggressive():
    X, y = data()
    old = PassiveAggressiveClassifier(random_state=0)  # xpd: deprecated-in-sklearn
    new = SGDClassifier(loss="hinge", penalty=None, learning_rate="pa1", eta0=1.0, random_state=0).fit(X, y)
    return new.predict(X), old.random_state


@case
def target_encoder():
    X = np.array([["a"], ["b"], ["c"]] * 20)
    y = np.arange(60) % 2
    old = TargetEncoder(shuffle=True, random_state=0)  # xpd: deprecated-in-sklearn
    new = TargetEncoder(cv=KFold(5, shuffle=True, random_state=0))
    return new.fit_transform(X, y), old.cv


@case
def mds():
    X, _ = data()
    D = np.sqrt(((X[:, None] - X[None]) ** 2).sum(-1))
    pre = MDS(dissimilarity="precomputed", init="random", random_state=0).fit(D)  # xpd: deprecated-in-sklearn
    nonmetric = MDS(metric=False, init="random", random_state=0).fit(X)  # xpd: deprecated-in-sklearn
    default = MDS(random_state=0)  # xpd: deprecated-in-sklearn
    silent = MDS(metric="euclidean", init="random", random_state=0).fit(X)
    return fitted(pre), fitted(nonmetric), default.random_state, silent.embedding_


@case
def hdbscan():
    X, _ = data()
    old = HDBSCAN(min_cluster_size=5).fit(X)  # xpd: deprecated-in-sklearn
    new = HDBSCAN(min_cluster_size=5, copy=False).fit(X)
    return old.labels_, new.labels_


@case
def path_alphas():
    X, y = reg_data()
    old = lasso_path(X, y, n_alphas=20)  # xpd: deprecated-in-sklearn
    none = lasso_path(X, y, alphas=None)  # xpd: deprecated-in-sklearn
    both = lasso_path(X, y, n_alphas=20, alphas=[0.1, 1.0])  # xpd: deprecated-in-sklearn
    new = lasso_path(X, y, alphas=20)
    return old, none, both, new


@case
def power_t():
    X, y = reg_data()
    old = SGDRegressor(learning_rate="invscaling", power_t=-0.25, random_state=0)  # xpd: deprecated-in-sklearn
    new = SGDRegressor(learning_rate="invscaling", power_t=0.25, random_state=0).fit(X, y)
    return new.predict(X), old.power_t


def removed_helpers():
    from sklearn.utils.extmath import stable_cumsum  # xpd: deprecated-in-sklearn
    from sklearn.metrics.cluster import entropy  # xpd: deprecated-in-sklearn
    from sklearn.metrics.cluster import mutual_info_score
    return stable_cumsum, entropy, mutual_info_score


def attempt(build):
    """What build returns, or None when it raises, as a removed form does on 1.9.1."""
    try:
        return build()
    except (ImportError, TypeError, ValueError, AttributeError):
        return None


@case
def model_selection_module():
    def old():
        from sklearn.cross_validation import cross_val_score, train_test_split  # xpd: removed-in-sklearn
        return train_test_split(np.arange(10), random_state=0), cross_val_score

    from sklearn.model_selection import train_test_split
    return attempt(old), train_test_split(np.arange(10), random_state=0)


@case
def externals_joblib():
    def whole():
        from sklearn.externals import joblib  # xpd: removed-in-sklearn
        return joblib.hash([1, 2])

    def names():
        from sklearn.externals.joblib import hash  # xpd: removed-in-sklearn
        return hash([1, 2])

    def aliased():
        import sklearn.externals.joblib as jl  # xpd: removed-in-sklearn
        return jl.hash([1, 2])

    import joblib
    return attempt(whole), attempt(names), attempt(aliased), joblib.hash([1, 2])


@case
def private_modules():
    def logistic():
        from sklearn.linear_model.logistic import LogisticRegression  # xpd: removed-in-sklearn
        return LogisticRegression().fit(*data()).coef_

    def blobs():
        from sklearn.datasets.samples_generator import make_blobs  # xpd: removed-in-sklearn
        return make_blobs(n_samples=10, random_state=0)

    def stop_words():
        from sklearn.feature_extraction.stop_words import ENGLISH_STOP_WORDS  # xpd: removed-in-sklearn
        return sorted(ENGLISH_STOP_WORDS)[:5]

    from sklearn.feature_extraction.text import ENGLISH_STOP_WORDS
    return attempt(logistic), attempt(blobs), attempt(stop_words), sorted(ENGLISH_STOP_WORDS)[:5]


@case
def feature_names():
    from sklearn.feature_extraction.text import CountVectorizer
    vec = CountVectorizer().fit(["red apple", "green apple"])
    old = attempt(lambda: vec.get_feature_names())  # xpd: removed-in-sklearn
    return old, vec.get_feature_names_out()


@case
def renamed_keywords():
    from sklearn.calibration import CalibratedClassifierCV
    from sklearn.cluster import AgglomerativeClustering
    from sklearn.ensemble import BaggingClassifier
    from sklearn.linear_model import LassoCV, RidgeCV
    from sklearn.manifold import TSNE
    from sklearn.model_selection import StratifiedKFold, StratifiedShuffleSplit
    from sklearn.preprocessing import OneHotEncoder
    from sklearn.svm import LinearSVC
    from sklearn.tree import DecisionTreeClassifier
    from sklearn.utils import check_array
    X, y = data()
    Xr, yr = reg_data()
    cats = [["a"], ["b"], ["a"]]
    return (
        attempt(lambda: OneHotEncoder(sparse=False).fit_transform(cats)),  # xpd: removed-in-sklearn
        OneHotEncoder(sparse_output=False).fit_transform(cats),
        attempt(lambda: BaggingClassifier(base_estimator=DecisionTreeClassifier(random_state=0), random_state=0).fit(X, y).predict(X)),  # xpd: removed-in-sklearn
        BaggingClassifier(estimator=DecisionTreeClassifier(random_state=0), random_state=0).fit(X, y).predict(X),
        attempt(lambda: CalibratedClassifierCV(base_estimator=LinearSVC(random_state=0)).fit(X, y).predict(X)),  # xpd: removed-in-sklearn
        attempt(lambda: TSNE(n_iter=250, random_state=0).fit_transform(X[:40])),  # xpd: removed-in-sklearn
        attempt(lambda: RidgeCV(store_cv_values=True).fit(Xr, yr).alpha_),  # xpd: removed-in-sklearn
        attempt(lambda: check_array([[1.0, np.nan]], force_all_finite=False)),  # xpd: removed-in-sklearn
        attempt(lambda: AgglomerativeClustering(affinity="manhattan", linkage="average").fit(X).labels_),  # xpd: removed-in-sklearn
        attempt(lambda: LassoCV(n_alphas=10).fit(Xr, yr).alpha_),  # xpd: removed-in-sklearn
        attempt(lambda: StratifiedKFold(n_folds=3).get_n_splits()),  # xpd: removed-in-sklearn
        attempt(lambda: StratifiedShuffleSplit(n_iter=3, random_state=0).get_n_splits()),  # xpd: removed-in-sklearn
        StratifiedKFold(n_splits=3).get_n_splits(),
    )


@case
def dropped_keywords():
    from sklearn.cluster import KMeans
    from sklearn.ensemble import AdaBoostClassifier, IsolationForest, RandomForestClassifier
    from sklearn.linear_model import Ridge
    from sklearn.model_selection import GridSearchCV
    X, y = data()
    return (
        attempt(lambda: KMeans(n_clusters=2, n_jobs=-1, random_state=0).fit(X).labels_),  # xpd: removed-in-sklearn
        KMeans(n_clusters=2, random_state=0).fit(X).labels_,
        attempt(lambda: RandomForestClassifier(n_estimators=5, min_impurity_split=None, random_state=0).fit(X, y).predict(X)),  # xpd: removed-in-sklearn
        attempt(lambda: Ridge(normalize=False).fit(X, y).coef_),  # xpd: removed-in-sklearn
        attempt(lambda: GridSearchCV(LogisticRegression(), {"C": [0.1, 1.0]}, fit_params=None).fit(X, y).best_params_),  # xpd: removed-in-sklearn
        attempt(lambda: IsolationForest(behaviour="new", random_state=0).fit(X).predict(X)),  # xpd: removed-in-sklearn
        attempt(lambda: AdaBoostClassifier(algorithm="SAMME", random_state=0).fit(X, y).predict(X)),  # xpd: removed-in-sklearn
        attempt(lambda: GridSearchCV(LogisticRegression(), {"C": [0.1, 1.0]}, iid=False).fit(X, y).best_params_),  # xpd: removed-in-sklearn
        attempt(lambda: LogisticRegression(multi_class="multinomial").fit(X, y).coef_),  # xpd: removed-in-sklearn
        LogisticRegression().fit(X, y).coef_,
    )


@case
def refused_values():
    from sklearn.ensemble import ExtraTreesClassifier, RandomForestClassifier
    X, y = data()
    Xr, yr = reg_data()
    return (
        attempt(lambda: RandomForestClassifier(n_estimators=5, max_features="auto", random_state=0).fit(X, y).predict(X)),  # xpd: removed-in-sklearn
        RandomForestClassifier(n_estimators=5, max_features="sqrt", random_state=0).fit(X, y).predict(X),
        attempt(lambda: RandomForestRegressor(n_estimators=5, max_features="auto", random_state=0).fit(Xr, yr).predict(Xr)),  # xpd: removed-in-sklearn
        attempt(lambda: SGDClassifier(loss="log", random_state=0).fit(X, y).predict(X)),  # xpd: removed-in-sklearn
        SGDClassifier(loss="log_loss", random_state=0).fit(X, y).predict(X),
        attempt(lambda: DecisionTreeRegressor(criterion="mse", random_state=0).fit(Xr, yr).predict(Xr)),  # xpd: removed-in-sklearn
        attempt(lambda: DecisionTreeRegressor(criterion="mae", random_state=0).fit(Xr, yr).predict(Xr)),  # xpd: removed-in-sklearn
        attempt(lambda: GradientBoostingRegressor(loss="ls", n_estimators=5, random_state=0).fit(Xr, yr).predict(Xr)),  # xpd: removed-in-sklearn
        attempt(lambda: GradientBoostingClassifier(loss="deviance", n_estimators=5, random_state=0).fit(X, y).predict(X)),  # xpd: removed-in-sklearn
        ExtraTreesClassifier(n_estimators=5, max_features="log2", random_state=0).fit(X, y).predict(X),
    )


def removed_forms(model, X, y, X_test, y_test, grid):
    from sklearn import grid_search  # xpd: removed-in-sklearn
    from sklearn import datasets, metrics
    from sklearn.calibration import CalibratedClassifierCV
    from sklearn.cross_validation import LabelKFold  # xpd: removed-in-sklearn
    from sklearn.datasets import load_boston  # xpd: removed-in-sklearn
    from sklearn.datasets import load_diabetes
    from sklearn.datasets.base import Bunch  # xpd: removed-in-sklearn
    from sklearn.ensemble.forest import BaseForest  # xpd: removed-in-sklearn
    from sklearn.dummy import DummyClassifier
    from sklearn.ensemble import GradientBoostingClassifier, RandomForestClassifier
    from sklearn.externals.six import StringIO  # xpd: removed-in-sklearn
    from sklearn.feature_selection import RFE
    from sklearn.linear_model import LassoCV, Ridge
    from sklearn.metrics import SCORERS  # xpd: removed-in-sklearn
    from sklearn.metrics import get_scorer_names, jaccard_score, mean_squared_error
    from sklearn.metrics import jaccard_similarity_score  # xpd: removed-in-sklearn
    from sklearn.metrics import plot_confusion_matrix, plot_roc_curve  # xpd: removed-in-sklearn, removed-in-sklearn
    from sklearn.model_selection import GridSearchCV, ShuffleSplit, StratifiedKFold, cross_val_score
    from sklearn.preprocessing import Imputer  # xpd: removed-in-sklearn
    from sklearn.impute import SimpleImputer
    import sklearn.externals.joblib  # xpd: removed-in-sklearn
    datasets.load_boston()  # xpd: removed-in-sklearn
    load_diabetes()
    metrics.SCORERS.keys()  # xpd: removed-in-sklearn
    metrics.plot_precision_recall_curve(model, X_test, y_test)  # xpd: removed-in-sklearn
    get_scorer_names()
    LogisticRegression(multi_class="ovr")  # xpd: removed-in-sklearn
    RandomForestClassifier(min_impurity_split=0.1, random_state=0)  # xpd: removed-in-sklearn
    RandomForestClassifier(min_impurity_decrease=0.1, random_state=0)
    Ridge(normalize=True)  # xpd: removed-in-sklearn
    GridSearchCV(model, grid, fit_params={"sample_weight": y})  # xpd: removed-in-sklearn
    SGDClassifier(n_iter=5, random_state=0)  # xpd: removed-in-sklearn
    LassoCV(n_alphas=100, alphas=None)  # xpd: removed-in-sklearn
    mean_squared_error(y, y, squared=False)  # xpd: removed-in-sklearn
    GradientBoostingClassifier(max_features="auto", random_state=0)  # xpd: removed-in-sklearn
    grid = {"max_features": ["auto", "sqrt"], "n_estimators": [10, 50]}  # xpd: removed-in-sklearn
    grid = {"rf__max_features": ["sqrt", "log2"], "solver": ["auto"]}
    CalibratedClassifierCV(model, cv="prefit")  # xpd: removed-in-sklearn
    LogisticRegression(penalty="none")  # xpd: removed-in-sklearn
    RandomForestClassifier(min_samples_split=1, random_state=0)  # xpd: removed-in-sklearn
    DummyClassifier("most_frequent")  # xpd: removed-in-sklearn
    DummyClassifier(strategy="most_frequent")
    RFE(model, 5)  # xpd: removed-in-sklearn
    RFE(model, n_features_to_select=5)
    GridSearchCV(model, grid, "accuracy")  # xpd: removed-in-sklearn
    cross_val_score(model, X, y, 5)  # xpd: removed-in-sklearn
    cross_val_score(model, X, y, cv=5)
    StratifiedKFold(y, n_folds=3)  # xpd: removed-in-sklearn
    ShuffleSplit(len(X), 10, test_size=0.4, random_state=0)  # xpd: removed-in-sklearn
    StratifiedKFold(3)
    StratifiedKFold(n_splits=3).split(X, y)
    jaccard_score(y, y)
    SimpleImputer(strategy="mean")
    return Bunch, BaseForest, StringIO, SCORERS, jaccard_similarity_score, plot_confusion_matrix, plot_roc_curve, LabelKFold, grid_search, load_boston, Imputer


def plot_confusion_matrix_of(model, X, y):
    def plot_confusion_matrix(m, a, b):
        return m.score(a, b)
    return plot_confusion_matrix(model, X, y)


def compat_paths():
    try:
        from sklearn.model_selection import train_test_split
    except ImportError:
        from sklearn.cross_validation import train_test_split
    try:
        enc = OneHotEncoder(sparse_output=False)
    except TypeError:
        enc = OneHotEncoder(sparse=False)
    return train_test_split, enc
