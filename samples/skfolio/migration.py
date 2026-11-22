"""Removed and deprecated skfolio forms alongside supported replacements."""

import numpy as np
import skfolio.optimization as opt
from skfolio.cluster import HierarchicalClustering
from skfolio.model_selection import WalkForward
from skfolio.moments import EWCovariance, EWMu, ImpliedCovariance
from skfolio.optimization import HierarchicalEqualRiskContribution, MeanRisk, NestedClustersOptimization
from skfolio.portfolio import Portfolio
from skfolio.population import Population
from skfolio.prior import FactorModel, TimeSeriesFactorModel
from skfolio.seriation import HierarchicalSeriation
from skfolio.uncertainty_set import UncertaintySet
from skfolio.utils import sorting


def removed(returns, factors):
    EWMu(alpha=0.2)  # xpd: removed-in-skfolio
    EWCovariance(alpha=0.2)  # xpd: removed-in-skfolio
    WalkForward(10, 20, expend_train=True)  # xpd: removed-in-skfolio
    UncertaintySet(k=1, sigma=np.eye(3))  # xpd: removed-in-skfolio
    region = UncertaintySet(radius=1, geometry=np.eye(3), norm=2)
    region.k  # xpd: removed-in-skfolio
    region.sigma  # xpd: removed-in-skfolio
    MeanRisk(prior_estimator=FactorModel())  # xpd: skfolio-factor-model-class
    TimeSeriesFactorModel().fit(returns, factors)  # xpd: skfolio-factor-model-class
    MeanRisk(prior_estimator=TimeSeriesFactorModel()).fit(returns, factors)  # xpd: skfolio-factor-model-class


def deprecated(returns, weights, fitness):
    from skfolio.optimization.cluster import HierarchicalRiskParity  # xpd: deprecated-in-skfolio
    from skfolio.optimization.cluster.hierarchical import SchurComplementary  # xpd: deprecated-in-skfolio
    from skfolio.optimization.cluster.hierarchical import BaseHierarchicalOptimization

    Portfolio(returns, weights, annualized_factor=12)  # xpd: deprecated-in-skfolio
    Portfolio(returns, weights, annualized_factor=12, annualization_factor=12)  # xpd: deprecated-in-skfolio
    Portfolio(returns, weights).annualized_factor  # xpd: deprecated-in-skfolio
    ImpliedCovariance(annualized_factor=12)  # xpd: deprecated-in-skfolio
    Population().non_denominated_sort()  # xpd: deprecated-in-skfolio
    sorting.non_denominated_sort(fitness, False)  # xpd: deprecated-in-skfolio
    cluster = HierarchicalClustering()
    model = HierarchicalRiskParity(hierarchical_clustering_estimator=cluster)  # xpd: skfolio-hierarchical-seriation
    model.fit(returns)
    model.hierarchical_clustering_estimator_  # xpd: skfolio-hierarchical-seriation
    SchurComplementary(hierarchical_clustering_estimator=cluster, seriation_estimator=HierarchicalSeriation())  # xpd: skfolio-hierarchical-seriation
    isinstance(model, BaseHierarchicalOptimization)  # xpd: skfolio-base-hierarchical-check
    issubclass(SchurComplementary, BaseHierarchicalOptimization)  # xpd: skfolio-base-hierarchical-check

    class CustomOptimizer(BaseHierarchicalOptimization):  # xpd: skfolio-base-hierarchical-check
        pass


def current(returns, factors, weights, fitness):
    EWMu(half_life=3.11)
    WalkForward(10, 20, expand_train=True)
    region = UncertaintySet(radius=1, geometry=np.eye(3), norm=2)
    region.radius
    TimeSeriesFactorModel().fit(returns, factors=factors)
    Portfolio(returns, weights, annualization_factor=12).annualization_factor
    factor = None
    ImpliedCovariance(annualized_factor=factor, annualization_factor=252)
    sorting.non_dominated_sort(fitness, False)
    cluster = HierarchicalClustering()
    HierarchicalEqualRiskContribution(hierarchical_clustering_estimator=cluster)
    NestedClustersOptimization(clustering_estimator=cluster)
    model = opt.HierarchicalRiskParity(seriation_estimator=HierarchicalSeriation())
    model.fit(returns)
    model.seriation_estimator_.hierarchical_clustering_estimator_
    return isinstance(model, opt.BaseOptimization)


def conflicting_spellings():
    WalkForward(10, 20, expand_train=False, expend_train=True)  # xpd: removed-in-skfolio
