skfolio MeanRisk and RiskBudgeting take a prior estimator that produces a joint estimate of returns and covariance.

```diff
-MeanRisk(mu_estimator=EWMu())
+MeanRisk(prior_estimator=EmpiricalPrior(mu_estimator=EWMu()))
```

## What the rule reports

Put the moment estimators in `EmpiricalPrior` and give it to the optimizer. ConvexOptimization is an abstract base class.
