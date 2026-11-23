skfolio 1.0 removed the decay keyword on exponentially weighted moments, the misspelled WalkForward keyword and the old uncertainty set fields.

```diff
-EWMu(alpha=0.2)
-WalkForward(60, 252, expend_train=True)
+EWMu(half_life=3.10628371950539)
+WalkForward(60, 252, expand_train=True)
```

## What the rule reports

The WalkForward fix changes an exception into a constructor call. Convert alpha with `-1 / log2(1 - alpha)`. For uncertainty sets, choose a radius, a geometry and an explicit norm that express the intended set.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to rename `expend_train` to `expand_train` and `UncertaintySet` `k`/`sigma` fields to `radius`/`geometry`. Convert `alpha` to `half_life` by hand.
