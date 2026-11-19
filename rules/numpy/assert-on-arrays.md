A failed `assert np.allclose(a, b)` reports only False. Call `np.testing.assert_allclose`, which shows the elements that differ.

```diff lang="python"
-assert np.allclose(weights, expected)
+np.testing.assert_allclose(weights, expected, rtol=1e-05, atol=1e-08)
```

## What the rule reports

The rule reports `assert np.allclose(a, b)`, `assert np.array_equal(a, b)`, `assert np.all(a == b)` and `assert (a == b).all()`. A failed assert shows only False, where the testing functions show the elements that differ.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.testing.assert_allclose(a, b, rtol=1e-05, atol=1e-08, equal_nan=False)`, with the `rtol` and `atol` that the call passes, and `np.testing.assert_array_equal(a, b)`, each with `err_msg=` for a message written as a string. assert_allclose fails on shapes that `np.allclose` broadcasts, assert_array_equal treats NaN in the same place as equal, and both still run under `python -O`. A message that is not a string, `equal_nan` and a splat have no fix.
