StatsForecast stores in-sample predictions from `forecast` when you set `fitted=True`.

```diff lang="python"
  forecast = StatsForecast(models=[SeasonalNaive(season_length=7)], freq="D")
- base = forecast.forecast(df=train, h=7, fitted=False)
+ base = forecast.forecast(df=train, h=7, fitted=True)
  fitted = forecast.forecast_fitted_values()
```

## What the rule reports

The rule looks for three consecutive statements that create a StatsForecast instance, call `forecast` without `fitted=True`, then call `forecast_fitted_values` on the same instance. It reports a missing flag or a literal `False`. It leaves a dynamic flag, extra positional forecast arguments, and unpacked forecast options alone.

StatsForecast 2.1.1 raised an exception asking for `fitted=True`. On three series of 70 training observations, the corrected call returned 210 rows. See the [core API](https://nixtlaverse.nixtla.io/statsforecast/src/core/core.html).

## How to fix it

Request `fitted=True` and read the fitted values before using the same instance for another method that changes its training data. xpd has no automatic fix.
