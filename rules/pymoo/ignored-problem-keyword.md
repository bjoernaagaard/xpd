pymoo 0.6 stores these pymoo 0.4 keywords of `Problem` and never reads them, so the problem does not behave as the code expects. Inherit from `ElementwiseProblem` or use the replacement that the message names.

```diff lang="python"
-class Sphere(Problem):
+class Sphere(ElementwiseProblem):
     def __init__(self, pool):
-        super().__init__(n_var=3, n_obj=1, xl=0.0, xu=1.0, elementwise_evaluation=True, parallelization=("starmap", pool.starmap))
+        super().__init__(n_var=3, n_obj=1, xl=0.0, xu=1.0, elementwise_runner=StarmapParallelization(pool.starmap))
 
     def _evaluate(self, x, out, *args, **kwargs):
         out["F"] = x[0] ** 2 + x[1] ** 2 + x[2] ** 2
```

## What the rule reports

The rule reports a keyword of the pymoo 0.4 `Problem` that pymoo 0.6 stores in `problem.data` and never
reads.

| Keyword | What happened on pymoo 0.6 |
| --- | --- |
| `elementwise_evaluation=True` | `_evaluate` gets the whole population where it expects one solution. A method of `x[0]`, `x[1]` and `x[2]` returned [0.66, 0.93, 1.26] for three solutions whose values are [0.14, 0.77, 1.94]. |
| `parallelization=("starmap", pool.starmap)` | The starmap was called 0 times in 3 generations, where pymoo 0.4.2.2 called it 3 times. |
| `type_var=int` | The problem had `vtype` None. |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The severity is error for `elementwise_evaluation` and `parallelization`, warning for `type_var`. unsafe for `type_var`: `vtype=` with the same value, which operators and helpers that read `vtype` can act on. None for `elementwise_evaluation` and `parallelization`, and none for `type_var` in a call that passes `vtype` too.
