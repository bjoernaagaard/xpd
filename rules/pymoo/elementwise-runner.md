pymoo calls `elementwise_runner` as `runner(f, X)`, so a bare `starmap` or a runner it never uses raises or does nothing. Wrap it, as in `StarmapParallelization(pool.starmap)`.

```diff lang="python"
-problem = MyElementwiseProblem(elementwise_runner=pool.starmap)
+problem = MyElementwiseProblem(elementwise_runner=StarmapParallelization(pool.starmap))
```

## What the rule reports

The rule reports an `elementwise_runner=` that pymoo cannot call or never calls.

| Code | What happened |
| --- | --- |
| `elementwise_runner=pool.starmap` | TypeError, "ElementwiseEvaluationFunction.__call__() takes 2 positional arguments but 3 were given", because a runner is called as `runner(f, X)` |
| `elementwise_runner=` given to a class that the file defines with `Problem` as its base | A `Problem` evaluates the population in one call, so the runner was called 0 times in 3 generations, where an `ElementwiseProblem` called it 3 times. |

A call that also passes `elementwise=` or `**kwargs` is left alone. `StarmapParallelization` is in `pymoo.core.problem` on 0.6.0 to 0.6.1.5 and in `pymoo.parallelization` from 0.6.1.6 on, and the message names the module of the release in the venv.

## How to fix it

xpd has no automatic fix, so make the change by hand.
