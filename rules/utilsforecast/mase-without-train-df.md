MASE divides forecast errors by a seasonal naive error measured on the training series. Pass that history to `evaluate`.

```diff lang="python"
- scores = evaluate(forecasts, metrics=[partial(mase, seasonality=7)])
+ scores = evaluate(forecasts, metrics=[partial(mase, seasonality=7)], train_df=train)
```

## What the rule reports

The rule looks for an inline metrics list or tuple containing the imported UtilsForecast `mase`, either directly or as the first argument to `functools.partial`. It reports the call when `train_df` is missing or explicitly `None` by keyword. It leaves calls with unpacked arguments or a positional training frame alone.

UtilsForecast 0.2.17 raised `ValueError` asking for `train_df`. The corrected call returned one MASE score for each of three series. See the [evaluation API](https://nixtlaverse.nixtla.io/utilsforecast/evaluation.html).

## How to fix it

Pass the observations used to train the forecast and bind the seasonal period when you construct the metric. xpd has no automatic fix because it cannot identify the correct history.
