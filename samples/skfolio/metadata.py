"""Panel policy, routed fit and seed settings."""

import numpy as np
from skfolio.containers import AssetPanel, Field2D, FieldCategorical, InactivePolicy
from skfolio.distribution import VineCopula
from skfolio.model_selection import MultipleRandomizedCV, OnlineRandomizedSearch, WalkForward
from skfolio.moments import ImpliedCovariance
from skfolio.optimization import MeanRisk, Random
from skfolio.preprocessing import prices_to_returns
from skfolio.prior import CharacteristicsFactorModel, EmpiricalPrior, SyntheticData, TimeSeriesFactorModel
from skfolio.uncertainty_set import BootstrapCovarianceUncertaintySet, BootstrapMuUncertaintySet


def metadata(returns, volatility, factor_returns):
    prior = EmpiricalPrior(covariance_estimator=ImpliedCovariance())
    MeanRisk(prior_estimator=prior).fit(returns, implied_vol=volatility)  # xpd: skfolio-metadata-routing-setup
    MeanRisk(prior_estimator=TimeSeriesFactorModel()).fit(returns, factors=factor_returns)  # xpd: skfolio-metadata-routing-setup
    ImpliedCovariance().fit(returns, implied_vol=volatility)
    TimeSeriesFactorModel().fit(returns, factors=factor_returns)


def panel():
    FieldCategorical(values=[[0, 0]], levels=["industry"], inactive_policy=InactivePolicy.ZERO)  # xpd: skfolio-panel-inactive-policy
    Field2D(values=[[0.1, 0.0]], inactive_policy="zero")  # xpd: skfolio-panel-inactive-policy
    AssetPanel({"returns": np.array([[0.1, 0.1]])}, observations=[0], asset_names=["A", "B"], active_mask=[[True, False]])  # xpd: skfolio-panel-inactive-policy
    field = Field2D(values=[[0.1, 0.0]], inactive_policy=InactivePolicy.ZERO)
    AssetPanel({"returns": field}, observations=[0], asset_names=["A", "B"], active_mask=[[True, False]])
    FieldCategorical(values=[[0, -1]], levels=["industry"], inactive_policy=InactivePolicy.MISSING)


def randomness(grid):
    VineCopula()  # xpd: skfolio-unseeded-estimator
    MultipleRandomizedCV(WalkForward(10, 20), 2, 2)  # xpd: skfolio-unseeded-estimator
    BootstrapMuUncertaintySet()  # xpd: skfolio-unseeded-estimator
    BootstrapCovarianceUncertaintySet()  # xpd: skfolio-unseeded-estimator
    SyntheticData()  # xpd: skfolio-unseeded-estimator
    OnlineRandomizedSearch(MeanRisk(raise_on_failure=False), grid)  # xpd: skfolio-unseeded-estimator
    BootstrapMuUncertaintySet(seed=42)
    SyntheticData(distribution_estimator=VineCopula(random_state=42))
    MultipleRandomizedCV(WalkForward(10, 20), 2, 2, None, 42)
    OnlineRandomizedSearch(MeanRisk(raise_on_failure=False), grid, random_state=42)


def panel_input(prices):
    characteristics = prices_to_returns(prices)
    CharacteristicsFactorModel(factors=[]).fit(characteristics=characteristics)  # xpd: skfolio-panel-inactive-policy


def global_randomness():
    Random()  # xpd: skfolio-unseeded-estimator
