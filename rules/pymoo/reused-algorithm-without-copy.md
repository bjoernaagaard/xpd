With `copy_algorithm=False`, a second `minimize` on the same algorithm does not set it up again and returns the earlier result. Build a new algorithm for each run, or drop `copy_algorithm=False`.

```diff lang="python"
-algorithm = NSGA2(pop_size=100)
 for problem in problems:
+    algorithm = NSGA2(pop_size=100)
     results.append(minimize(problem, algorithm, ("n_gen", 200), seed=1, copy_algorithm=False))
```

## What the rule reports

The rule reports a second `minimize` on the same algorithm when both calls pass `copy_algorithm=False`, in one block or in a loop that never binds the name. With `copy_algorithm=False`, `minimize` sets the algorithm up only when it has no problem yet. A second call with another problem, `("n_gen", 30)` and another seed returned the `F` of the first call, still at generation 11 and still on the first problem. A loop over the algorithms, as in `for algorithm in [NSGA2(), SMSEMOA()]`, gives each pass another object and is left alone.

## How to fix it

`xpd check --fix --unsafe-fixes` changes `copy_algorithm=False` to `copy_algorithm=True` on the repeated `minimize` call so each run starts cleanly. It is unsafe because constructing a fresh algorithm object per run is also a valid fix.
