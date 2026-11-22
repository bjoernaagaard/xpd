When a parent passes metadata to a skfolio estimator, the consuming estimator may need an explicit request and the scikit-learn configuration must allow routing.

```diff
-prior = EmpiricalPrior(covariance_estimator=ImpliedCovariance())
-model = MeanRisk(prior_estimator=prior)
-model.fit(returns, implied_vol=volatility)
+set_config(enable_metadata_routing=True)
+covariance = ImpliedCovariance().set_fit_request(implied_vol=True)
+prior = EmpiricalPrior(covariance_estimator=covariance)
+model = MeanRisk(prior_estimator=prior)
+model.fit(returns, implied_vol=volatility)
```

## What the rule reports

Use the request for the metadata key and method that consume it. Direct ImpliedCovariance.fit accepts implied_vol as an argument. A file with no local routing setup may still receive configuration from its caller.
