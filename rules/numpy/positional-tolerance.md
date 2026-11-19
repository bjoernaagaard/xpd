The third argument of `np.isclose` and `np.allclose` is `rtol`, a relative tolerance, and many readers expect an absolute one. Name the argument, as `rtol=` or as `atol=` when an absolute tolerance was meant.

```diff lang="python"
-same = np.isclose(a, b, 1e-3)
+same = np.isclose(a, b, atol=1e-3)
```

## What the rule reports

The rule reports `np.isclose(a, b, 1e-3)` and `np.allclose(a, b, 1e-3)`. The third argument sets `rtol`, and `atol` stays 1e-8.

The rule leaves a call alone that also passes `atol`, or that passes a value named like `rtol`.

## How to fix it

`xpd check --fix` applies the fix of this rule.

The safe fix writes `np.isclose(a, b, rtol=1e-3)`, which names the parameter that the third argument already fills. Where an absolute tolerance was meant, change it to `atol=`. A call with a splat before the third argument has no fix.
