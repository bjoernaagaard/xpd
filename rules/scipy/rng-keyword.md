Since SciPy 1.15, these functions take `rng=`, and SciPy's docs say new code should use it in place of `random_state=` or `seed=`. Pass `rng=`, and expect other random numbers for the same seed.

```diff lang="python"
-engine = qmc.Sobol(d=2, seed=0)
-U, s, Vt = svds(A, k=3, random_state=0)
+engine = qmc.Sobol(d=2, rng=0)
+U, s, Vt = svds(A, k=3, rng=0)
```

## What the rule reports

The rule reports `random_state=` and `seed=` passed by keyword to the functions and classes that
SciPy 1.15 moved to `rng=`. SciPy's docs say new code should pass `rng`. Examples are
`sparse.random`, `svds`, `bootstrap`, `permutation_test`, `kmeans2`, `differential_evolution`,
`basinhopping`, `dual_annealing`, `directed_hausdorff`, `Rotation.random` and the qmc engines.
The old keywords run without a warning on SciPy 1.18.1.

The rule is skipped on SciPy older than 1.15. It leaves alone a call under a SciPy version check,
because `rng=` raises on an older SciPy. xpd reads only the installed SciPy and can't see which
SciPy versions a library supports. The message therefore says to keep the old keyword when the
code must also run on SciPy older than 1.15.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `rng=` for `random_state=` and `seed=`. The same integer gives a different stream under `rng=`, and `qmc.Sobol` raises for a `RandomState` instance passed as `rng=`.

On `sparse.random`, [`sparse-matrix-api`](sparse-matrix-api.md) then advises `sparse.random_array`, which returns a sparse array.
