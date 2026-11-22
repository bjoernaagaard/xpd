"""Examples of forecast inputs that xpd can check without running Nixtla."""

from functools import partial

import numpy as np
from coreforecast.grouped_array import GroupedArray
from hierarchicalforecast.core import HierarchicalReconciliation
from hierarchicalforecast.methods import BottomUp, MinTrace
from mlforecast import MLForecast
from mlforecast.utils import PredictionIntervals
from sklearn.linear_model import Ridge
from statsforecast import StatsForecast
from statsforecast.models import SeasonalNaive
from utilsforecast.evaluation import evaluate
from utilsforecast.losses import mae, mase


def grouped(values):
    GroupedArray(values, np.array([3, 10]))  # xpd: indptr-missing-zero
    return GroupedArray(values, np.array([0, 3, 10]))


def scores(forecasts, train, options):
    evaluate(forecasts, [mae, partial(mase, seasonality=7)])  # xpd: mase-without-train-df
    evaluate(forecasts, metrics=[mae])
    evaluate(forecasts, metrics=[partial(mase, seasonality=7)], **options)
    return evaluate(forecasts, metrics=[partial(mase, seasonality=7)], train_df=train)


def ml_intervals(train):
    forecaster = MLForecast(models=[Ridge()], freq="D", lags=[1, 7])
    forecaster.fit(train)
    forecast = forecaster.predict(h=7, level=[95])  # xpd: intervals-without-calibration
    forecaster.fit(train, prediction_intervals=PredictionIntervals(n_windows=2, h=7))
    return forecast, forecaster.predict(h=7, level=[95])


def fitted_for_reconciliation(train):
    forecaster = StatsForecast(models=[SeasonalNaive(season_length=7)], freq="D")
    base = forecaster.forecast(df=train, h=7, fitted=False)
    fitted = forecaster.forecast_fitted_values()  # xpd: fitted-values-without-fitted
    return base, fitted


def fitted_for_reconciliation_corrected(train):
    forecaster = StatsForecast(models=[SeasonalNaive(season_length=7)], freq="D")
    forecaster.forecast(df=train, h=7, fitted=True)
    return forecaster.forecast_fitted_values()


def reconcile(base, summing, tags, fitted):
    hrec = HierarchicalReconciliation(reconcilers=[MinTrace(method="mint_shrink")])
    forecasts = hrec.reconcile(base, tags, summing)  # xpd: mintrace-without-insample
    bottom_up = HierarchicalReconciliation(reconcilers=[BottomUp()])
    bottom_up.reconcile(Y_hat_df=base, S_df=summing, tags=tags)
    corrected = hrec.reconcile(Y_hat_df=base, S_df=summing, tags=tags, Y_df=fitted)
    return forecasts, corrected
