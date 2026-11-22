"""Optimizer settings and portfolio accounting."""

from skfolio.measures import RiskMeasure
from skfolio.moments import EWCovariance, EWMu
from skfolio.optimization import MeanRisk, ObjectiveFunction, RiskBudgeting
from skfolio.portfolio import FailedPortfolio, MultiPeriodPortfolio, Portfolio
from skfolio.prior import EmpiricalPrior


def accounting(returns, weights):
    Portfolio(returns, weights, weight_drift=True)  # xpd: skfolio-weight-drift-uncompounded
    MultiPeriodPortfolio([Portfolio(returns, weights, weight_drift=True, compounded=True)])  # xpd: skfolio-weight-drift-uncompounded
    Portfolio(returns, weights, management_fees=0.01)  # xpd: skfolio-cost-periodicity
    Portfolio(returns, weights, transaction_costs=0.01)  # xpd: skfolio-cost-periodicity
    Portfolio(returns, weights, weight_drift=True, compounded=True)
    Portfolio(returns, weights, transaction_costs=0.01, previous_weights=weights)


def failures(returns):
    book = MeanRisk(min_return=5, raise_on_failure=False).fit(returns).predict(returns)
    return book.weights  # xpd: skfolio-unchecked-failed-portfolio


def checked_failure(returns):
    book = MeanRisk(min_return=5, raise_on_failure=False).fit(returns).predict(returns)
    if isinstance(book, FailedPortfolio):
        raise RuntimeError(book.optimization_error)
    return book.weights


def constraints():
    MeanRisk(risk_measure="variance")  # xpd: skfolio-string-enum-option
    MeanRisk(objective_function="minimize_risk")  # xpd: skfolio-string-enum-option
    MeanRisk(min_weights=0.7, max_weights=0.2)  # xpd: skfolio-invalid-convex-bounds
    MeanRisk(l1_coef=-0.1)  # xpd: skfolio-invalid-convex-bounds
    MeanRisk(l2_coef=-0.1)  # xpd: skfolio-invalid-convex-bounds
    MeanRisk(risk_measure=RiskMeasure.EVAR, evar_beta=1.2)  # xpd: skfolio-invalid-convex-bounds
    MeanRisk(risk_measure=RiskMeasure.CVAR, cvar_beta=-0.2)  # xpd: skfolio-invalid-convex-bounds
    RiskBudgeting(risk_measure=RiskMeasure.CDAR, cdar_beta=1.2)  # xpd: skfolio-invalid-convex-bounds
    MeanRisk(mu_estimator=EWMu())  # xpd: skfolio-estimator-on-optimizer
    RiskBudgeting(covariance_estimator=EWCovariance())  # xpd: skfolio-estimator-on-optimizer
    MeanRisk(risk_measure=RiskMeasure.CVAR, cvar_beta=1, objective_function=ObjectiveFunction.MINIMIZE_RISK)
    MeanRisk(prior_estimator=EmpiricalPrior(mu_estimator=EWMu(), covariance_estimator=EWCovariance()))
