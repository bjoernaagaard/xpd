The distance indicators accept `normalize` and do nothing with it, so the distances stay on the scale of the objectives. Pass `zero_to_one=True` with the ideal and nadir points.

```diff lang="python"
-igd = IGD(pf, normalize=True)(res.F)
+igd = IGD(pf, zero_to_one=True)(res.F)
```

## What the rule reports

The rule reports `normalize=True` on `GD`, `GDPlus`, `IGD` and `IGDPlus`. The keyword is from pymoo 0.4, and the constructors still accept it and do nothing with it.

| A front whose first objective runs to 1000 | pymoo 0.4.2.2 | pymoo 0.6.2 |
| --- | --- | --- |
| `IGD(pf, normalize=True)` | 0.082 | 25.25 |
| `IGD(pf)` | not run | 25.25 |
| `IGD(pf, zero_to_one=True)` | not run | 0.082 |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `zero_to_one=`.
