pymoo moved or deprecated these forms, and the old import warns or raises. Use the place that the message names.

```diff lang="python"
-from pymoo.core.problem import StarmapParallelization
+from pymoo.parallelization import StarmapParallelization
 
-feasible = res.pop.get("feasible")
+feasible = res.pop.get("FEAS")
```

## What the rule reports

The rule reports two forms that pymoo deprecated.

| Form | On pymoo | Replacement |
| --- | --- | --- |
| `StarmapParallelization` imported from `pymoo.core.problem` | 0.6.1.6: ImportError. 0.6.2: DeprecationWarning. | the import from `pymoo.parallelization`, which gives the same class on 0.6.2 |
| the key `"feasible"` given to `get` | since 0.6.1.2: a DeprecationWarning for every individual, 40 for a population of 20 | `"FEAS"`, which returned an equal array with no warning |

Two documents read the key. One needs `res.pop`, `res.opt` or `res.off`, where `res` is the
result of `minimize`, and it has the fix. The other reads the name of the object, a variable or
an attribute named `pop`, `opt`, `off` or `population`, and it has no fix. It gives a wrong
report on `get` of a dictionary with such a name.

## How to fix it

`xpd check --fix` applies the fix of this rule.

The safe fix writes `from pymoo.parallelization import StarmapParallelization` for the import from `pymoo.core.problem` when the statement imports that one name, and `"FEAS"` for `"feasible"` on the population of a result of `minimize`. The key on another object has no fix.
