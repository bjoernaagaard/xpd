"""Run the documented minimum-variance walk-forward strategy from a Polars frame."""

from collections.abc import Iterable, Iterator

import cvxpy as cp
import numpy as np
import polars as pl
from numpy.typing import NDArray

from native import DATE, check_dates

type Floats = NDArray[np.float64]


def observation_folds(
    rows: int, *, train: int, test: int
) -> Iterator[tuple[range, range]]:
    """Yield rolling windows with a complete test period."""
    if train < 2 or test < 1:
        message = (
            "Training needs at least two observations and testing needs at least one"
        )
        raise ValueError(message)
    for start in range(train, rows - test + 1, test):
        yield range(start - train, start), range(start, start + test)


def minimum_variance(values: Floats) -> Floats:
    """Solve the fully invested long-only empirical minimum-variance problem."""
    covariance = np.atleast_2d(np.cov(values, rowvar=False, ddof=1))
    factor = np.linalg.cholesky(covariance)
    weights = cp.Variable(values.shape[1])
    risk = cp.Variable(nonneg=True)
    budget = cp.sum(weights) == 1.0
    bounds = [weights >= 0.0, weights <= 1.0]
    model = cp.Problem(
        cp.Minimize(cp.square(risk)),
        [*bounds, budget, cp.SOC(risk, factor.T @ weights)],
    )
    model.solve(solver="CLARABEL", tol_gap_abs=1e-9, tol_gap_rel=1e-9)
    if model.status != cp.OPTIMAL or weights.value is None:
        message = f"Minimum-variance solve did not converge: {model.status}"
        raise RuntimeError(message)
    return weights.value


def walk_forward(
    frame: pl.DataFrame,
    folds: Iterable[tuple[range, range]],
    *,
    transaction_cost: float,
) -> tuple[pl.DataFrame, pl.DataFrame]:
    """Return dated strategy returns and named ending weights for each fold."""
    check_dates(frame)
    assets = [column for column in frame.columns if column != DATE]
    returns = frame.select(assets).to_numpy()
    if not np.isfinite(returns).all():
        message = "The optimizer requires finite asset returns"
        raise ValueError(message)

    daily = []
    endings = []
    previous = np.zeros(len(assets))
    for train, test in folds:
        weights = minimum_variance(returns[train.start : train.stop])
        path = returns[test.start : test.stop]
        positions = np.cumprod(1 + path, axis=0) * weights
        wealth = positions.sum(axis=1) + (1 - weights.sum())
        if np.any(wealth <= 0):
            message = (
                f"Non-positive portfolio wealth in fold starting at row {test.start}"
            )
            raise ValueError(message)
        cost = (transaction_cost * np.abs(previous - weights)).sum()
        observation_returns = wealth / np.concatenate(([1.0], wealth[:-1])) - 1 - cost
        dates = frame[DATE].slice(test.start, len(test))
        daily.append(pl.DataFrame({DATE: dates, "return": observation_returns}))
        previous = positions[-1] / wealth[-1]
        ending = {DATE: dates.tail(1)}
        ending.update(
            {asset: [value] for asset, value in zip(assets, previous, strict=True)}
        )
        endings.append(pl.DataFrame(ending))

    if not daily:
        message = "No complete walk-forward folds"
        raise ValueError(message)

    result = pl.concat(daily, rechunk=True).with_columns(
        wealth=(1 + pl.col("return")).cum_prod()
    )
    return result, pl.concat(endings, rechunk=True)
