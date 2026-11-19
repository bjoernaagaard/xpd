`GroupedClassifier` drops a shrinkage keyword without an error, so the setting has no effect. Use `HierarchicalClassifier`, which takes `shrinkage=`, or remove the keyword.

```diff lang="python"
-classifier = GroupedClassifier(LogisticRegression(), groups=["shop"], shrinkage="relative")
-regressor = GroupedRegressor(LinearRegression(), groups=["region", "shop"], shrinkage="constant", alpha=0.1)
+classifier = HierarchicalClassifier(LogisticRegression(), groups=["shop"], shrinkage="relative")
+regressor = GroupedRegressor(LinearRegression(), groups=["region", "shop"], shrinkage="constant", shrinkage_kwargs={"alpha": 0.1})
```

## What the rule reports

The rule reports spellings of shrinkage that fail on the grouped and hierarchical meta estimators:

- any extra keyword on `GroupedClassifier`, which drops it
- `shrinkage="constant"` or `"min_n_obs"` without `shrinkage_kwargs`
- a bare `alpha=` or `min_n_obs=` keyword

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `shrinkage_kwargs={"alpha": 0.1}` for `alpha=0.1`, and the same for `min_n_obs=`. The other forms have no fix.

`GroupedClassifier` ends its signature with `**shrinkage_kwargs` and never reads it. With
`shrinkage="relative"` its probabilities were bit equal to the probabilities without the keyword. They
differed by up to 0.29 from `HierarchicalClassifier(shrinkage="relative")`, which does shrink.

| Form | Error |
| --- | --- |
| `shrinkage="constant"` without a setting | TypeError "constant_shrinkage() missing 1 required positional argument: 'alpha'" in `fit` |
| a bare `alpha=` keyword | TypeError "got an unexpected keyword argument 'alpha'" |

`GroupedPredictor` took the bare keyword until scikit-lego 0.9.2, so the document about its bare
keyword carries `since: "0.9.2"`.
