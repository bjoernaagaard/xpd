`minimize` runs a deep copy of the algorithm and leaves the one you passed untouched, so reading results from it gives nothing. Read them from the result, as in `res.algorithm`.

```diff lang="python"
 algorithm = NSGA2(pop_size=100, callback=Best())
 res = minimize(problem, algorithm, ("n_gen", 200), seed=1)
-best = algorithm.callback.data["best"]
+best = res.algorithm.callback.data["best"]
```

## What the rule reports

The rule reports `pop`, `opt`, `off`, `n_gen`, `n_iter`, `history`, `callback` and `result` read from an algorithm after it was given to `minimize`. `minimize` runs a deep copy and returns it as `res.algorithm`.

| After a run of 10 generations | Value |
| --- | --- |
| `algorithm.pop`, `algorithm.opt`, `algorithm.n_gen` | None |
| `algorithm.result()` | TypeError |
| records of a callback given to the constructor | 0 |
| records of the callback in `res.algorithm` | 10 |

The rule leaves alone a call with `copy_algorithm`, and a name that is assigned again after the run or bound by a walrus, as in `(algorithm := res.algorithm)`. It also leaves alone a file that drives the same name with `setup` or `next`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `res.algorithm.pop` for `algorithm.pop`, with the name the code gave the result. The read then gets the algorithm that ran, and a name bound again between the run and the read gives another object.
