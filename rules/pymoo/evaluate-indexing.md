The index takes the wrong axis of `x` for this problem class, so it raises IndexError or reads the wrong values. Index one solution of an `ElementwiseProblem` with one subscript, and a `Problem` with two.

```diff lang="python"
-class Sphere(Problem):
+class Sphere(ElementwiseProblem):
     def _evaluate(self, x, out, *args, **kwargs):
         out["F"] = x[0] ** 2 + x[1] ** 2
```

## What the rule reports

The rule reports an index that takes the wrong axis of `x` for the class.

| Class | Code | What happened |
| --- | --- | --- |
| `ElementwiseProblem` | `x[:, 0]` on the parameter of `_evaluate` | IndexError, "too many indices for array" |
| `Problem` | `x[0]`, when `_evaluate` uses the parameter only with whole numbers as subscripts and has no loop | `x[0]` takes the first solution and not the first variable. With 100 solutions of 2 variables it raised Exception, "Problem Error: F can not be set, expected shape (100, 1) but provided (2,)". |

The rule leaves alone a `Problem` class that passes `elementwise=` with any value but `False`, or passes `**kwargs` on to `__init__`, because it can be built with `elementwise=True` and then gets one solution.

## How to fix it

`xpd check --fix --unsafe-fixes` rewrites `x[:, i]` to `x[i]` inside `ElementwiseProblem._evaluate` and `x[i]` to `x[:, i]` inside `Problem._evaluate`. It is unsafe because changing the base class instead is also a valid fix.
