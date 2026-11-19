`fit` of `PolynomialWrapper` and `NestedCVWrapper` returns None, so calling a method on its result raises AttributeError. Call `fit` on one line and the method on the next, or call `fit_transform`.

```diff lang="python"
-encoded = PolynomialWrapper(ce.TargetEncoder(cols=["city"])).fit(X, y).transform(X)
+wrapper = PolynomialWrapper(ce.TargetEncoder(cols=["city"]))
+wrapper.fit(X, y)
+encoded = wrapper.transform(X)
```

## What the rule reports

The rule reports a method call on the result of `fit` of a `PolynomialWrapper` or a
`NestedCVWrapper`, and a name assigned from that `fit`. Their `fit` has no return statement, so it
returns `None` where every scikit-learn estimator returns itself.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The old form raised `AttributeError: 'NoneType' object has no attribute 'transform'` on both
wrappers. `fit_transform` returns the encoded frame on both and is left alone.
