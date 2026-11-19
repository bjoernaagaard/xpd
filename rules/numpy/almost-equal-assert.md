`assert_almost_equal` and its kin are on NumPy's not recommended list, and their default test hides an absolute tolerance of 1.5e-7. Use `assert_allclose` with the tolerance written out.

```diff lang="python"
-np.testing.assert_array_almost_equal(result, expected, decimal=4)
+np.testing.assert_allclose(result, expected, rtol=0, atol=1.5e-4)
```

## What the rule reports

The rule reports `assert_almost_equal`, `assert_array_almost_equal` and `assert_approx_equal`, which NumPy lists under "Asserts (not recommended)".

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.testing.assert_allclose(a, b, rtol=0, atol=1.5e-7)`, with `1.5e-6` for `assert_array_almost_equal` and `1.5e-<decimal>` for a literal decimal. A difference of exactly `atol` failed before and passes after, dicts and ragged lists are no longer compared item by item, and complex values are compared by magnitude. A bare name that the file imports from `numpy.testing` gets `assert_allclose(a, b, rtol=0, atol=1.5e-7)` in a file that also imports `assert_allclose` from `numpy.testing`. `assert_approx_equal(a, b)` gets `assert_allclose(a, b, rtol=1e-6)`, which can fail where a difference up to ten times `rtol` passed. Any other bare name, an `err_msg` argument, a computed decimal and `significant=` have no fix.
