Since pymoo 0.6.0, the fourth positional argument of `Problem` is the number of equality constraints, so bounds given by position land in the wrong place. Pass `xl=` and `xu=` by keyword.

```diff lang="python"
 class Sphere(Problem):
     def __init__(self):
-        super().__init__(2, 1, 0, np.zeros(2), np.ones(2))
+        super().__init__(2, 1, 0, xl=np.zeros(2), xu=np.ones(2))
```

## What the rule reports

The rule reports `xl` and `xu` given by position to `super().__init__` in a problem class, or to `Problem` and `ElementwiseProblem` called by name. pymoo 0.5 had `Problem(n_var, n_obj, n_constr, xl, xu)`. Since pymoo 0.6.0 the signature is `(n_var, n_obj, n_ieq_constr, n_eq_constr, xl, xu)`, so the fourth argument is read as the number of equality constraints and the fifth as `xl`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `xl=` and `xu=` on the bounds given by position, as in `Problem(2, 1, 0, xl=np.zeros(2), xu=np.ones(2))`. The problem gets other bounds than before, and the fix drops comments inside the parentheses.

The first class was built with no error on 0.6.0 and later, and `minimize` failed afterwards.

| Arguments of `super().__init__` | pymoo | What happened |
| --- | --- | --- |
| `(2, 1, 0, np.zeros(2), np.ones(2))` | 0.6.0, 0.6.2 | The problem had `n_eq_constr` [0, 0], `xl` [1, 1] and `xu` None, and `minimize` with `GA` raised TypeError, "only integer scalar arrays can be converted to a scalar index". |
| `(2, 1, 0, np.zeros(2), xu=np.ones(2))` | 0.6.0, 0.6.2 | The problem had `xl` None. |
| either of the two above | 0.5.0 | The run had `xl` [0, 0] and `xu` [1, 1]. |
| `(2, 1, 0, xl=np.zeros(2), xu=np.ones(2))` | 0.5.0, 0.6.0, 0.6.2 | The run worked. |

The rule reads a call with exactly five positional arguments and no `xl` or `xu` keyword, and a
call with exactly four and `xu` as a keyword. A whole number written as the fourth argument is a
count, as in `Problem(2, 1, 0, 0, xl)`, and is left alone.
