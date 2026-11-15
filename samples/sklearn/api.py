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
