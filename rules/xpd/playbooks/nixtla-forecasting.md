# Forecast and reconcile a Nixtla hierarchy

Use a chronological holdout to score a forecast hierarchy. Backtest the statistical baseline on earlier windows, check that the machine learning model has future features for each date, and pass fitted base forecasts to MinTrace before scoring the reconciled forecasts. The example uses two made-up store series and their total.

Before fitting, use `xpd ts-review start` and the bare `xpd ts-review` list to inspect the series. Record decisions with `xpd ts-review plan`, use `approve`, `gate` or `drop` for fixes, and use `status` and `finish` to close the review.

## Asks

- forecast and reconcile a Nixtla hierarchy
- forecast multiple time series with StatsForecast and MLForecast
- evaluate reconciled forecasts with MASE
- check future exogenous feature rows
- pass fitted values to MinTrace

## Code

```python
# /// script
# requires-python = ">=3.13"
# dependencies = [
#   "coreforecast==0.0.20",
#   "hierarchicalforecast==1.5.3",
#   "mlforecast==1.1.0",
#   "numpy==2.5.5",
#   "pandas==2.3.3",
#   "scikit-learn==1.9.1",
#   "statsforecast==2.1.1",
#   "utilsforecast==0.2.17",
# ]
# ///
"""Forecast two stores and check that the reconciled total equals their sum."""

from functools import partial

import numpy as np
import pandas as pd
from coreforecast.grouped_array import GroupedArray
from coreforecast.lag_transforms import ExpandingMean
from hierarchicalforecast.core import HierarchicalReconciliation
from hierarchicalforecast.methods import MinTrace
from hierarchicalforecast.utils import aggregate
from mlforecast import MLForecast
from mlforecast.utils import PredictionIntervals
from sklearn.linear_model import Ridge
from statsforecast import StatsForecast
from statsforecast.models import SeasonalNaive
from utilsforecast.evaluation import evaluate
from utilsforecast.losses import mae, mase

HORIZON = 7
SEASON = 7
FRIDAY = 4
KEYS = ["unique_id", "ds"]


def example_observations() -> pd.DataFrame:
    """Make two daily series with independent noise and a known promotion calendar."""
    periods = 77
    dates = pd.date_range("2024-01-01", periods=periods, freq="D")
    rng = np.random.default_rng(73)
    base = np.repeat([50.0, 70.0], periods)
    noise = rng.normal(0.0, 4.0, size=base.size)
    return pd.DataFrame(
        {
            "region": "all",
            "store": np.repeat(["east", "west"], periods),
            "ds": np.tile(dates, 2),
            "y": base + noise,
        }
    )


def check_group_boundaries(train: pd.DataFrame) -> int:
    """Check that a lagged mean starts over at each series boundary."""
    ordered = train.sort_values(KEYS)
    lengths = ordered.groupby("unique_id", sort=False, observed=True).size().to_numpy()
    offsets = np.concatenate(([0], lengths.cumsum()))
    groups = GroupedArray(ordered["y"].to_numpy(), offsets)
    lagged = ExpandingMean(lag=1).transform(groups)
    if not np.isnan(lagged[offsets[:-1]]).all():
        msg = "Expected each series to begin with an unavailable lag."
        raise ValueError(msg)
    return len(lengths)


def features_with_promo(frame: pd.DataFrame) -> pd.DataFrame:
    """Add a Friday promotion that is known ahead of time."""
    promo = frame["ds"].dt.dayofweek.eq(FRIDAY).astype("int8")
    return frame.assign(promo=promo)


def forecast_ml(train: pd.DataFrame, future: pd.DataFrame) -> pd.DataFrame:
    """Fit interval calibration and check the keys of the future features."""
    ml = MLForecast(models=[Ridge()], freq="D", lags=[1, SEASON])
    calibration = PredictionIntervals(n_windows=2, h=HORIZON)
    ml.fit(features_with_promo(train), static_features=[], prediction_intervals=calibration)
    future_features = features_with_promo(future[KEYS])
    missing = ml.get_missing_future(h=HORIZON, X_df=future_features)
    if not missing.empty:
        msg = f"Missing future features for {len(missing)} series and date pairs."
        raise ValueError(msg)
    return ml.predict(h=HORIZON, X_df=future_features, level=[95])


def main() -> None:
    observations = example_observations()
    series, summing, tags = aggregate(observations, [["region"], ["region", "store"]])
    holdout_start = sorted(series["ds"].unique())[-HORIZON]
    train = series.loc[series["ds"] < holdout_start]
    holdout = series.loc[series["ds"] >= holdout_start]
    group_count = check_group_boundaries(train)

    stats = StatsForecast(models=[SeasonalNaive(season_length=SEASON)], freq="D")
    windows = stats.cross_validation(df=train, h=HORIZON, n_windows=2, step_size=HORIZON)
    backtest_scores = evaluate(windows, metrics=[mae], models=["SeasonalNaive"])

    base = stats.forecast(df=train, h=HORIZON, fitted=True)
    fitted = stats.forecast_fitted_values()
    reconciliation = HierarchicalReconciliation(reconcilers=[MinTrace(method="mint_shrink")])
    reconciled = reconciliation.reconcile(Y_hat_df=base, Y_df=fitted, S_df=summing, tags=tags)
    model = next(column for column in reconciled if "/MinTrace_" in column)
    forecast_and_actual = holdout.merge(reconciled, on=KEYS, validate="one_to_one")
    if len(forecast_and_actual) != len(holdout) or len(reconciled) != len(holdout):
        msg = "Forecasts and holdout observations have different series and date pairs."
        raise ValueError(msg)
    wide = forecast_and_actual.pivot_table(
        index="ds", columns="unique_id", values=model, aggfunc="sum", observed=True
    )
    leaf_sum = wide[["all/east", "all/west"]].sum(axis=1)
    np.testing.assert_allclose(wide["all"].to_numpy(), leaf_sum.to_numpy())

    seasonal_mase = partial(mase, seasonality=SEASON)
    holdout_scores = evaluate(
        forecast_and_actual, metrics=[seasonal_mase], models=[model], train_df=train
    )
    ml_forecast = forecast_ml(train, holdout)
    interval_columns = ["Ridge-lo-95", "Ridge-hi-95"]
    print(f"{group_count} groups, {len(wide)} coherent forecast dates")
    print(f"{windows['cutoff'].nunique()} backtest cutoffs, {len(backtest_scores)} MAE scores")
    interval_count = len(ml_forecast[interval_columns])
    print(f"{len(holdout_scores)} holdout MASE scores, {interval_count} ML intervals")


if __name__ == "__main__":
    main()
```

## Commands

1. Save the code as `nixtla_forecasting.py` and run `xpd check nixtla_forecasting.py`.
2. Run `uv run --python 3.13 nixtla_forecasting.py`. Expect three groups, seven coherent dates, two backtest cutoffs, three holdout scores, and 21 ML prediction intervals.

## Rules

- coreforecast/indptr-missing-zero
- utilsforecast/mase-without-train-df
- mlforecast/intervals-without-calibration
- statsforecast/fitted-values-without-fitted
- hierarchicalforecast/mintrace-without-insample
