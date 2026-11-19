An array built from a list of objectives has one row per objective, and pymoo reshapes it without transposing, so each solution gets the wrong values. Use `np.column_stack`, or transpose the array.

```diff lang="python"
 class TwoObjectives(Problem):
     def _evaluate(self, x, out, *args, **kwargs):
         f1 = x[:, 0]
         f2 = 10 + x[:, 1]
-        out["F"] = np.array([f1, f2])
+        out["F"] = np.column_stack([f1, f2])
```

## What the rule reports

The rule reports `out["F"]`, `out["G"]` or `out["H"]` set to `np.array`, `np.asarray`, `np.vstack`, `np.stack` or `np.row_stack` of a list, in a class that inherits from `Problem`. Such an array has one row per objective, and pymoo reshapes it to one row per solution without swapping its rows and columns. Two objectives over four solutions, `f1 = x0` and `f2 = 10 + x1`, came back as [[0.1, 0.3], [0.5, 0.7], [10.2, 10.4], [10.6, 10.8]], and `NSGA2` then reported a least `f2` of 0.003 where the true least is 10.

The rule leaves alone an `ElementwiseProblem`, where a list of numbers is right, and a stack followed by `.T`. It also leaves alone a class that passes `elementwise=` with any value but `False`, such as `elementwise=elementwise`, or passes `**kwargs` on to `__init__`, because the caller can then build it with `elementwise=True`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.column_stack` on the same list.
