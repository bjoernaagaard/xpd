skfolio expects enum members for risk measures and objectives. It uses methods and properties on those members during fit.

```diff
-MeanRisk(risk_measure='variance', objective_function='minimize_risk')
+MeanRisk(risk_measure=RiskMeasure.VARIANCE, objective_function=ObjectiveFunction.MINIMIZE_RISK)
```

## What the rule reports

Import RiskMeasure from skfolio.measures and ObjectiveFunction from skfolio.optimization.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to convert string `risk_measure` and `objective_function` values to `RiskMeasure` and `ObjectiveFunction` enum members when those enums are imported.
