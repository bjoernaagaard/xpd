pymoo ignores what `_evaluate` returns and reads the objectives from `out`, so every objective comes back as inf without an error. Write the value to `out["F"]`.

```diff lang="python"
 class Sphere(Problem):
     def _evaluate(self, x, out, *args, **kwargs):
-        return (x**2).sum(axis=1)
+        out["F"] = (x**2).sum(axis=1)
```

## What the rule reports

The rule reports a `return` with a value in `_evaluate` when the method never names its `out` parameter. pymoo ignores the returned value, and every objective then comes back as inf. `GA` returned `res.F = [inf]` without an error. A method that hands `out` to a helper is left alone.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `out["F"] = <value>` for the `return`. pymoo then reads the value where it read inf, the method goes on past that line, and a value of another shape raises.
