Writing `out["G"]` or `out["H"]` without declaring the number of constraints raises on the first evaluation. Pass `n_ieq_constr` or `n_eq_constr` with the number of constraints.

```diff lang="python"
 class Bounded(Problem):
     def __init__(self):
-        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0)
+        super().__init__(n_var=2, n_obj=1, n_ieq_constr=1, xl=0.0, xu=1.0)
 
     def _evaluate(self, x, out, *args, **kwargs):
         out["F"] = (x**2).sum(axis=1)
         out["G"] = 0.5 - x[:, 0]
```

## What the rule reports

The rule reports `out["G"]` written in a class that never passes `n_ieq_constr`, and `out["H"]` in a class that never passes `n_eq_constr`. Both counts default to 0, and the first evaluation raises Exception, "Problem Error: G can not be set, expected shape (2, 0) but provided (2,)". pymoo 0.5.0 ignored such a constraint without an error. The class must call `super().__init__` with keywords only, so a class that passes `**kwargs` on is left alone.

## How to fix it

xpd has no automatic fix, so make the change by hand.
