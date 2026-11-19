`save_history=True` keeps a deep copy of the algorithm for every generation, which made a run 2.6 times slower and its result 150 times larger, and this file never reads it. Drop `save_history=True`.

```diff lang="python"
-res = minimize(problem, algorithm, ("n_gen", 200), seed=1, save_history=True)
+res = minimize(problem, algorithm, ("n_gen", 200), seed=1)
 print(res.F)
```

## What the rule reports

The rule reports `save_history=True` given to `minimize` outside every function, in a file that never names `history` and does not use pickle, dill or joblib. The option keeps a deep copy of the algorithm for every generation. When another file reads the history of the run, the report is wrong.

| `NSGA2`, 100 solutions, 200 generations | Time | Pickled result | `F` |
| --- | --- | --- | --- |
| `save_history=True` | 1.17 seconds | 20.3 MB | the same |
| without it | 0.45 seconds | 0.13 MB | the same |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix drops `save_history=True`, so `res.history` is empty.
