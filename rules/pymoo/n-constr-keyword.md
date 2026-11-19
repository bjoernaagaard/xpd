Since pymoo 0.6.0, `Problem` has no `n_constr` parameter and reads it as the number of inequality constraints. Write `n_ieq_constr` or `n_eq_constr`.

```diff lang="python"
 class Welded(Problem):
     def __init__(self):
-        super().__init__(n_var=4, n_obj=2, n_constr=4, xl=xl, xu=xu)
+        super().__init__(n_var=4, n_obj=2, n_ieq_constr=4, xl=xl, xu=xu)
```

## What the rule reports

The rule reports `n_constr=` given to `super().__init__` in a problem class, or to `Problem` and `ElementwiseProblem` called by name. `Problem` has no such parameter since pymoo 0.6.0 and reads it from `**kwargs` as `n_ieq_constr`. A problem built with either keyword had the same `n_ieq_constr`, and `GA` with `seed=1` returned the same `X`, `F` and `G` for both, bit for bit. A call that passes both keywords is left alone.

## How to fix it

`xpd check --fix` applies the fix of this rule.

The safe fix writes `n_ieq_constr=`.
