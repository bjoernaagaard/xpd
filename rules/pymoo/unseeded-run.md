`minimize` without `seed` starts from a new random state on every run, so two runs return different fronts. Pass `seed`.

```diff lang="python"
-res = minimize(problem, NSGA2(pop_size=100), ("n_gen", 200))
+res = minimize(problem, NSGA2(pop_size=100), ("n_gen", 200), seed=1)
```

## What the rule reports

The rule reports `minimize` without `seed`. Two runs of `NSGA2` on ZDT1 then returned different fronts, and two runs with `seed=1` returned the same front. The rule leaves alone a call that takes `**kwargs`, a call whose algorithm was built with `seed`, and a file that calls `np.random.seed`, which `numpy-seed-ignored` reports.

## How to fix it

xpd has no automatic fix, so make the change by hand.
