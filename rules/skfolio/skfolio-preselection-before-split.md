Fitting skfolio asset selection on the full input before splitting can change the assets selected using future returns.

```diff
-selected = SelectKExtremes(k=1).fit_transform(returns)
-result = cross_val_predict(EqualWeighted(), selected, cv=splitter)
+model = make_pipeline(SelectKExtremes(k=1), EqualWeighted())
+result = cross_val_predict(model, returns, cv=splitter)
```

## What the rule reports

The rule follows results from a direct selector construction and `fit_transform` into a split or skfolio cross-validation. Fit on the training set and call `transform` separately when splitting by hand.
