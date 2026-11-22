"""Ordinary sklearn code remains valid in a file that also imports skfolio."""

from random import Random

from sklearn import set_config
from sklearn.linear_model import Ridge
from sklearn.model_selection import GridSearchCV, KFold, cross_val_predict, train_test_split
from skfolio.model_selection import WalkForward
from skfolio.moments import ImpliedCovariance
from skfolio.optimization import EqualWeighted, MeanRisk
from skfolio.prior import EmpiricalPrior


def regression(features, target):
    Random()
    regressor = Ridge(alpha=0.5)
    train_test_split(features, target, random_state=42)
    folds = KFold(shuffle=True, random_state=42)
    GridSearchCV(regressor, {"alpha": [0.5, 1.0]}, cv=folds)
    return cross_val_predict(regressor, features, target, cv=folds)


def routed(returns, implied_vol):
    set_config(enable_metadata_routing=True)
    MeanRisk(prior_estimator=EmpiricalPrior(covariance_estimator=ImpliedCovariance())).fit(returns, implied_vol=implied_vol)  # xpd: skfolio-metadata-routing-setup
    covariance = ImpliedCovariance().set_fit_request(implied_vol=True)
    prior = EmpiricalPrior(covariance_estimator=covariance)
    model = MeanRisk(prior_estimator=prior, fallback=EqualWeighted())
    model.fit(returns, implied_vol=implied_vol)
    return WalkForward(10, 20, expand_train=True)
