`np.polyfit` and its kin are NumPy's legacy polynomial API, and the docs recommend `Polynomial.fit` as more stable. Use the `np.polynomial` classes.

```diff lang="python"
-slope, intercept = np.polyfit(x, y, 1)
+intercept, slope = np.polynomial.Polynomial.fit(x, y, 1).convert().coef
```

## What the rule reports

The rule reports `np.polyfit`, `np.poly1d`, `np.polyval`, `np.roots`, `np.poly`, `np.polyder`, `np.polyint`, `np.polyadd`, `np.polysub`, `np.polymul` and `np.polydiv`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.polynomial.Polynomial.fit(x, y, deg).convert().coef[::-1]` for `polyfit`, and the `numpy.polynomial` function on `coef[::-1]` for `poly1d`, `polyval`, `roots`, `poly`, `polyder` and `polyint`, reversed again where coefficients come out. A poly1d argument raises, whole number coefficients become floats, roots come out sorted, and `poly1d` becomes a `Polynomial` with other methods. `polyadd`, `polysub`, `polymul`, `polydiv` and calls with other arguments have no fix.
