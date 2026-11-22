Contradictory asset weight bounds are infeasible. Negative penalties can break convexity and out-of-range tail probabilities can break the selected risk objective.

```diff
-MeanRisk(min_weights=0.7, max_weights=0.2)
-MeanRisk(risk_measure=RiskMeasure.CVAR, cvar_beta=1.2)
+MeanRisk(min_weights=0, max_weights=0.8)
+MeanRisk(risk_measure=RiskMeasure.CVAR, cvar_beta=0.95)
```

## What the rule reports

The bounds check covers literal contradictions across zero, one and the middle of the unit interval. The confidence check requires the corresponding risk measure. Zero and one are accepted for the tested MeanRisk tail measures.
