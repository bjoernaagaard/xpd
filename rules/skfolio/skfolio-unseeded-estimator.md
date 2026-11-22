Several skfolio estimators draw random samples. Set an explicit seed when results must be reproducible.

```diff
-BootstrapMuUncertaintySet()
-SyntheticData()
+BootstrapMuUncertaintySet(seed=42)
+SyntheticData(distribution_estimator=VineCopula(random_state=42))
```

## What the rule reports

The bootstrap estimators take seed=. VineCopula, MultipleRandomizedCV and OnlineRandomizedSearch take random_state=. Seed the distribution passed to SyntheticData. Random has no per-estimator seed argument.
