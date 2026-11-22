`MLForecast.predict(level=...)` needs a fit that calibrates prediction intervals. Without calibration, prediction can succeed with only point forecasts.

```diff lang="python"
- forecast.fit(train)
+ forecast.fit(train, prediction_intervals=PredictionIntervals(n_windows=2, h=7))
  predicted = forecast.predict(h=7, level=[95])
```

## What the rule reports

The rule follows the receiver to an imported `MLForecast` constructor. It reports a forecast with a literal confidence level when the previous statement is a call to `fit` on the same receiver with no interval configuration or with `prediction_intervals=None`. It leaves calls with unpacked fit arguments alone.

On MLForecast 1.1.0, the uncalibrated call warned and returned `Ridge` alone. The calibrated call returned `Ridge`, `Ridge-lo-95` and `Ridge-hi-95`. See [Nixtla’s prediction interval guide](https://nixtlaverse.nixtla.io/mlforecast/docs/tutorials/prediction_intervals_in_forecasting_models.html).

## How to fix it

Fit with `PredictionIntervals` and a calibration horizon and number of windows that fit the data. xpd has no automatic fix because the choice depends on the forecast horizon and the available history.
