"""Financial split and validation cases."""

from sklearn.model_selection import GridSearchCV, KFold, ShuffleSplit, cross_val_predict as sklearn_predict, train_test_split
from sklearn.pipeline import make_pipeline
from skfolio.datasets import load_sp500_dataset
from skfolio.model_selection import CombinatorialPurgedCV, WalkForward, cross_val_predict, online_predict
from skfolio.moments import EWCovariance, EWMu
from skfolio.optimization import EqualWeighted, MeanRisk
from skfolio.pre_selection import DropCorrelated, DropZeroVariance
from skfolio.prior import EmpiricalPrior
from skfolio.preprocessing import prices_to_returns


def financial_splits(prices):
    returns = prices_to_returns(prices)
    train_test_split(returns)  # xpd: skfolio-shuffled-returns-split
    KFold(n_splits=4, shuffle=True, random_state=42).split(returns)  # xpd: skfolio-shuffled-returns-split
    ShuffleSplit(random_state=42).split(returns)  # xpd: skfolio-shuffled-returns-split
    cross_val_predict(EqualWeighted(), returns, cv=KFold(shuffle=True, random_state=42))  # xpd: skfolio-shuffled-returns-split
    train_test_split(returns, shuffle=False)
    walk = WalkForward(10, 20)
    walk.split(returns)
    CombinatorialPurgedCV(n_folds=5, n_test_folds=2)  # xpd: skfolio-cpcv-zero-purge
    CombinatorialPurgedCV(5, 2, purged_size=0, embargo_size=0)  # xpd: skfolio-cpcv-zero-purge
    CombinatorialPurgedCV(5, 2, 0, 0)  # xpd: skfolio-cpcv-zero-purge
    CombinatorialPurgedCV(5, 2, purged_size=1)


def preselection(returns):
    selected = DropCorrelated().fit_transform(returns)
    train_test_split(selected, shuffle=False)  # xpd: skfolio-preselection-before-split
    edited = DropZeroVariance().fit_transform(returns)
    splitter = WalkForward(10, 20)
    cross_val_predict(EqualWeighted(), edited, cv=splitter)  # xpd: skfolio-preselection-before-split
    pipeline = make_pipeline(DropCorrelated(), EqualWeighted())
    cross_val_predict(pipeline, returns, cv=splitter)


def validations(returns, grid):
    walk = WalkForward(10, 20, expand_train=True)
    sklearn_predict(EqualWeighted(), returns, cv=walk)  # xpd: skfolio-sklearn-cross-val-predict
    sklearn_predict(EqualWeighted(), returns, cv=KFold(n_splits=3))  # xpd: skfolio-sklearn-cross-val-predict
    GridSearchCV(MeanRisk(fallback=EqualWeighted()), grid, cv=walk)  # xpd: skfolio-grid-search-vs-online
    cross_val_predict(MeanRisk(), returns, cv=walk)  # xpd: skfolio-missing-fallback-in-cv
    cross_val_predict(MeanRisk(fallback=EqualWeighted()), returns, cv=walk)
    cross_val_predict(MeanRisk(raise_on_failure=False), returns, cv=walk)
    model = MeanRisk()
    for train, test in WalkForward(10, 20, expand_train=True).split(returns):
        model.fit(returns.iloc[train])  # xpd: skfolio-fit-loop-vs-online
        model.predict(returns.iloc[test])
    online_predict(MeanRisk(prior_estimator=EmpiricalPrior(mu_estimator=EWMu(half_life=8), covariance_estimator=EWCovariance(half_life=8))), returns, warmup_size=20, test_size=10)


def input_data(prices):
    log_returns = prices_to_returns(prices, log_returns=True)
    EqualWeighted().fit(log_returns)  # xpd: skfolio-log-returns-in-optimizer
    levels = load_sp500_dataset()
    EqualWeighted().fit(levels)  # xpd: skfolio-prices-not-returns
    EqualWeighted().fit(prices_to_returns(levels))
