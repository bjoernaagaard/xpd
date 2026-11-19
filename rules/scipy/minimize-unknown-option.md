`minimize` warns and ignores an option that the chosen method does not know, so the setting has no effect. Use an option that the method takes, which `show_options` lists.

```diff lang="python"
-res = minimize(f, x0, method="L-BFGS-B", options={"disp": True, "maxiter": 200})
+res = minimize(f, x0, method="L-BFGS-B", options={"maxiter": 200})
```

## What the rule reports

The rule reports a key in the `options=` dict of `optimize.minimize` that the chosen `method=`
doesn't take but another method takes. Examples are `disp` and `iprint` for L-BFGS-B, `maxiter`
for TNC and `maxfun` for BFGS. On SciPy 1.18.1, minimize warns OptimizeWarning and ignores such a
key, and with `'trust-constr'` it raises TypeError. xpd took the keys of each method from a run of
minimize with each pair of method and key.

## How to fix it

xpd has no automatic fix, so make the change by hand.
